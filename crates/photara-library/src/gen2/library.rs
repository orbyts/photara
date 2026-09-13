use super::*;
use std::collections::{BTreeMap, BTreeSet};

impl LocalLibraryStore {
    /// Creates or CAS-updates a local Library. Retirement never cascades.
    /// # Errors
    /// Rejects invalid identity, stale revision or live dependent authority.
    pub async fn put_library(
        &self,
        value: &Library,
        expected: Option<Revision>,
    ) -> Result<WriteOutcome> {
        validation::name(&value.display_name, 512)?;
        validation::extensions(&value.extensions)?;
        let m = &value.meta;
        if m.id != m.library_id {
            return Err(Error::Invalid);
        }
        let mut tx = self.write().await?;
        check_meta(&mut tx, Table::Library, m, expected).await?;
        if m.state == Lifecycle::Tombstoned {
            for table in [
                Table::Person,
                Table::Organization,
                Table::Kind,
                Table::Location,
                Table::Social,
                Table::Relationship,
                Table::Root,
                Table::Locator,
            ] {
                let sql = sqlx::AssertSqlSafe(format!(
                    "SELECT EXISTS(SELECT 1 FROM {} WHERE library_id=? AND state='active')",
                    table.names().0
                ));
                if sqlx::query_scalar::<_, bool>(sql)
                    .bind(m.id.bytes())
                    .fetch_one(&mut *tx)
                    .await?
                {
                    return Err(Error::Constraint);
                }
            }
            if sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM project_catalog WHERE library_id=? AND visibility='visible')").bind(m.id.bytes()).fetch_one(&mut *tx).await? {return Err(Error::Constraint);}
        }
        sqlx::query(if m.revision==Revision::INITIAL {"INSERT INTO libraries(library_id,display_name,state,local_revision,created_at_ms,updated_at_ms,retired_at_ms,term_policy_version,extensions_json) VALUES(?,?,?,?,?,?,?,1,?)"} else {"UPDATE libraries SET library_id=?1,display_name=?2,state=?3,local_revision=?4,created_at_ms=?5,updated_at_ms=?6,retired_at_ms=?7,term_policy_version=1,extensions_json=?8 WHERE library_id=?1 AND library_id=?1"})
            .bind(m.id.bytes()).bind(&value.display_name).bind(m.state.sql()).bind(m.revision.get()).bind(m.created_at.get()).bind(m.updated_at.get()).bind(m.retired_at()).bind(canonical(&value.extensions)?).execute(&mut *tx).await?;
        let outcome = finish(
            &mut tx,
            self.info.device_id,
            Table::Library,
            m,
            expected,
            value,
        )
        .await?;
        tx.commit().await?;
        Ok(outcome)
    }
    /// # Errors
    /// Returns storage or typed decoding failure.
    pub async fn library(&self, id: LibraryId) -> Result<Option<Library>> {
        let mut tx = self.read().await?;
        let rows = rows(&mut tx, Table::Library, id, Some(id.uuid()), None, 1, true).await?;
        rows.first().map(library).transpose()
    }
    /// Lists local Libraries by stable UUID, including tombstones only on request.
    /// # Errors
    /// Returns invalid bounds or storage/decoding failure.
    pub async fn libraries(
        &self,
        after: Option<LibraryId>,
        limit: u32,
        retired: bool,
    ) -> Result<Vec<Library>> {
        page(0, limit)?;
        let rows=sqlx::query("SELECT * FROM libraries WHERE library_id>? AND (? OR state='active') ORDER BY library_id LIMIT ?")
            .bind(after.map_or_else(Vec::new,LibraryId::bytes)).bind(retired).bind(limit).fetch_all(self.db.pool()).await?;
        rows.iter().map(library).collect()
    }
    /// Atomically replaces Person details and capability/label sets at one revision.
    /// # Errors
    /// Returns validation, CAS, uniqueness or reference errors.
    pub async fn put_person(
        &self,
        value: &Person,
        expected: Option<Revision>,
    ) -> Result<WriteOutcome> {
        validation::party(&value.details)?;
        validation::set(&value.capabilities, 256)?;
        for capability in &value.capabilities {
            validation::identifier(capability)?;
        }
        let mut tx = self.write().await?;
        check_meta(&mut tx, Table::Person, &value.meta, expected).await?;
        put_party(&mut tx, Table::Person, &value.meta, &value.details).await?;
        sqlx::query("DELETE FROM person_capabilities WHERE library_id=? AND person_id=?")
            .bind(value.meta.library_id.bytes())
            .bind(value.meta.id.bytes())
            .execute(&mut *tx)
            .await?;
        for capability in &value.capabilities {
            sqlx::query("INSERT INTO person_capabilities VALUES(?,?,?)")
                .bind(value.meta.library_id.bytes())
                .bind(value.meta.id.bytes())
                .bind(capability)
                .execute(&mut *tx)
                .await?;
        }
        let result = finish(
            &mut tx,
            self.info.device_id,
            Table::Person,
            &value.meta,
            expected,
            value,
        )
        .await?;
        tx.commit().await?;
        Ok(result)
    }
    /// Organizations represent client identities without a separate Client table.
    /// # Errors
    /// Returns validation, CAS, uniqueness or reference errors.
    pub async fn put_organization(
        &self,
        value: &Organization,
        expected: Option<Revision>,
    ) -> Result<WriteOutcome> {
        validation::party(&value.details)?;
        let mut tx = self.write().await?;
        check_meta(&mut tx, Table::Organization, &value.meta, expected).await?;
        put_party(&mut tx, Table::Organization, &value.meta, &value.details).await?;
        let result = finish(
            &mut tx,
            self.info.device_id,
            Table::Organization,
            &value.meta,
            expected,
            value,
        )
        .await?;
        tx.commit().await?;
        Ok(result)
    }
    /// Half-open periods for the same pair/type may be adjacent, never overlap.
    /// # Errors
    /// Returns validation, CAS or same-Library/live-reference/interval errors.
    pub async fn put_relationship(
        &self,
        value: &Relationship,
        expected: Option<Revision>,
    ) -> Result<WriteOutcome> {
        validation::identifier(&value.relationship_type)?;
        validation::bounded(&value.notes, 8192)?;
        validation::labels(&value.labels)?;
        validation::extensions(&value.extensions)?;
        if let (Some(from), Some(until)) = (value.valid_from, value.valid_until)
            && until <= from
        {
            return Err(Error::Invalid);
        }
        let m = &value.meta;
        let mut tx = self.write().await?;
        check_meta(&mut tx, Table::Relationship, m, expected).await?;
        sqlx::query(if m.revision==Revision::INITIAL {"INSERT INTO person_organization_relationships(relationship_id,library_id,person_id,organization_id,relationship_type,valid_from_ms,valid_until_ms,notes,labels_json,record_schema,local_revision,created_at_ms,updated_at_ms,state,retired_at_ms,extensions_json) VALUES(?,?,?,?,?,?,?,?,?,1,?,?,?,?,?,?)"} else {"UPDATE person_organization_relationships SET relationship_id=?1,library_id=?2,person_id=?3,organization_id=?4,relationship_type=?5,valid_from_ms=?6,valid_until_ms=?7,notes=?8,labels_json=?9,record_schema=1,local_revision=?10,created_at_ms=?11,updated_at_ms=?12,state=?13,retired_at_ms=?14,extensions_json=?15 WHERE relationship_id=?1 AND library_id=?2"})
            .bind(m.id.bytes()).bind(m.library_id.bytes()).bind(value.person_id.bytes()).bind(value.organization_id.bytes()).bind(&value.relationship_type).bind(value.valid_from.map(Timestamp::get)).bind(value.valid_until.map(Timestamp::get)).bind(&value.notes).bind(canonical(&value.labels)?).bind(m.revision.get()).bind(m.created_at.get()).bind(m.updated_at.get()).bind(m.state.sql()).bind(m.retired_at()).bind(canonical(&value.extensions)?).execute(&mut *tx).await?;
        let result = finish(
            &mut tx,
            self.info.device_id,
            Table::Relationship,
            m,
            expected,
            value,
        )
        .await?;
        tx.commit().await?;
        Ok(result)
    }
    /// Claims canonical and alias closure atomically; rename retains old claims.
    /// Merge/claim transfer is deliberately not exposed by this L2 API.
    /// # Errors
    /// Returns validation, CAS, term collision or live dependent errors.
    pub async fn put_location_kind(
        &self,
        value: &LocationKind,
        expected: Option<Revision>,
    ) -> Result<WriteOutcome> {
        validation::bounded(&value.description, 8192)?;
        validation::extensions(&value.extensions)?;
        let mut claims = validation::claims(&value.canonical_display, &value.aliases)?;
        let key = normalize_term(&value.canonical_display)?;
        let m = &value.meta;
        let mut tx = self.write().await?;
        check_meta(&mut tx, Table::Kind, m, expected).await?;
        for (old_key, spellings) in terms(&mut tx, m.library_id, m.id).await? {
            claims.entry(old_key).or_default().extend(spellings);
        }
        if claims.len() > 256 || claims.values().any(|s| s.len() > 256) {
            return Err(Error::Limit);
        }
        let retirement = if m.state == Lifecycle::Tombstoned {
            Some(canonical(&claims.iter().map(|(key,spellings)|serde_json::json!({"term_key":key,"spellings":spellings,"policy_version":1})).collect::<Vec<_>>())?)
        } else {
            None
        };
        sqlx::query(if m.revision==Revision::INITIAL {"INSERT INTO location_kinds(location_kind_id,library_id,canonical_key,canonical_display,description,record_schema,local_revision,created_at_ms,updated_at_ms,state,retired_at_ms,extensions_json,retirement_terms_json) VALUES(?,?,?,?,?,1,?,?,?,?,?,?,?)"} else {"UPDATE location_kinds SET location_kind_id=?1,library_id=?2,canonical_key=?3,canonical_display=?4,description=?5,record_schema=1,local_revision=?6,created_at_ms=?7,updated_at_ms=?8,state=?9,retired_at_ms=?10,extensions_json=?11,retirement_terms_json=?12 WHERE location_kind_id=?1 AND library_id=?2"})
            .bind(m.id.bytes()).bind(m.library_id.bytes()).bind(&key).bind(&value.canonical_display).bind(&value.description).bind(m.revision.get()).bind(m.created_at.get()).bind(m.updated_at.get()).bind(m.state.sql()).bind(m.retired_at()).bind(canonical(&value.extensions)?).bind(retirement).execute(&mut *tx).await?;
        for (term, spellings) in &claims {
            let count=sqlx::query("INSERT INTO location_kind_terms(library_id,term_key,location_kind_id,policy_version,spellings_json) VALUES(?,?,?,1,?) ON CONFLICT(library_id,term_key) DO UPDATE SET spellings_json=excluded.spellings_json WHERE location_kind_terms.location_kind_id=excluded.location_kind_id")
                .bind(m.library_id.bytes()).bind(term).bind(m.id.bytes()).bind(canonical(spellings)?).execute(&mut *tx).await?.rows_affected();
            if count != 1 {
                return Err(Error::Conflict);
            }
        }
        let row = rows(
            &mut tx,
            Table::Kind,
            m.library_id,
            Some(m.id.uuid()),
            None,
            1,
            true,
        )
        .await?
        .remove(0);
        let stored = kind(&mut tx, &row).await?;
        let result = finish(
            &mut tx,
            self.info.device_id,
            Table::Kind,
            m,
            expected,
            &stored,
        )
        .await?;
        tx.commit().await?;
        Ok(result)
    }
    /// Resolves a canonical/alias input to its reserved owner, including tombstones.
    /// # Errors
    /// Returns invalid normalization or storage errors.
    pub async fn resolve_location_kind(
        &self,
        library: LibraryId,
        input: &str,
    ) -> Result<Option<LocationKind>> {
        let key = normalize_term(input)?;
        let mut tx = self.read().await?;
        let row=sqlx::query("SELECT k.* FROM location_kind_terms t JOIN location_kinds k ON k.library_id=t.library_id AND k.location_kind_id=t.location_kind_id WHERE t.library_id=? AND t.term_key=?")
            .bind(library.bytes()).bind(key).fetch_optional(&mut *tx).await?;
        match row {
            Some(row) => Ok(Some(kind(&mut tx, &row).await?)),
            None => Ok(None),
        }
    }
    /// Locations require one active same-Library Kind and an acyclic live parent.
    /// # Errors
    /// Returns validation, CAS, kind/hierarchy or cross-Library errors.
    pub async fn put_location(
        &self,
        value: &Location,
        expected: Option<Revision>,
    ) -> Result<WriteOutcome> {
        validation::party(&value.details)?;
        if !value.details.labels.is_empty() {
            return Err(Error::Unsupported);
        }
        if value.address.lines.len() > 16 {
            return Err(Error::Limit);
        }
        for text in value.address.lines.iter().chain([
            &value.address.locality,
            &value.address.region,
            &value.address.postal_code,
            &value.address.country_code,
        ]) {
            validation::bounded(text, 512)?;
        }
        if let Some(c) = value.coordinates
            && (!c.latitude.is_finite()
                || !c.longitude.is_finite()
                || !(-90.0..=90.0).contains(&c.latitude)
                || !(-180.0..=180.0).contains(&c.longitude))
        {
            return Err(Error::Invalid);
        }
        let m = &value.meta;
        let d = &value.details;
        let mut tx = self.write().await?;
        check_meta(&mut tx, Table::Location, m, expected).await?;
        sqlx::query(if m.revision==Revision::INITIAL {"INSERT INTO locations(location_id,library_id,location_kind_id,parent_location_id,display_name,sort_key,description,aliases_json,address_json,latitude,longitude,record_schema,local_revision,created_at_ms,updated_at_ms,state,retired_at_ms,extensions_json) VALUES(?,?,?,?,?,?,?,?,?,?,?,1,?,?,?,?,?,?)"} else {"UPDATE locations SET location_id=?1,library_id=?2,location_kind_id=?3,parent_location_id=?4,display_name=?5,sort_key=?6,description=?7,aliases_json=?8,address_json=?9,latitude=?10,longitude=?11,record_schema=1,local_revision=?12,created_at_ms=?13,updated_at_ms=?14,state=?15,retired_at_ms=?16,extensions_json=?17 WHERE location_id=?1 AND library_id=?2"})
            .bind(m.id.bytes()).bind(m.library_id.bytes()).bind(value.kind_id.bytes()).bind(value.parent_id.map(LocationId::bytes)).bind(&d.display_name).bind(normalize_term(&d.display_name)?).bind(&d.description).bind(canonical(&d.aliases)?).bind(canonical(&value.address)?).bind(value.coordinates.map(|c|c.latitude)).bind(value.coordinates.map(|c|c.longitude)).bind(m.revision.get()).bind(m.created_at.get()).bind(m.updated_at.get()).bind(m.state.sql()).bind(m.retired_at()).bind(canonical(&d.extensions)?).execute(&mut *tx).await?;
        let result = finish(
            &mut tx,
            self.info.device_id,
            Table::Location,
            m,
            expected,
            value,
        )
        .await?;
        tx.commit().await?;
        Ok(result)
    }
    /// Manual-first profile metadata; this method performs no provider/OAuth I/O.
    /// Bound subjects reserve Library/provider/namespace/subject even on tombstones.
    /// # Errors
    /// Returns validation, CAS, immutable owner/subject or uniqueness failures.
    pub async fn put_social_profile(
        &self,
        value: &SocialProfile,
        expected: Option<Revision>,
    ) -> Result<WriteOutcome> {
        validation::social(value)?;
        let m = &value.meta;
        let mut tx = self.write().await?;
        check_meta(&mut tx, Table::Social, m, expected).await?;
        let (person, organization) = match value.owner {
            SocialOwner::Person(id) => (Some(id.bytes()), None),
            SocialOwner::Organization(id) => (None, Some(id.bytes())),
        };
        sqlx::query(if m.revision==Revision::INITIAL {"INSERT INTO social_profiles(social_profile_id,library_id,person_id,organization_id,provider_id,subject_namespace,provider_subject_id,handle,display_name,profile_url,account_kind,provider_account_kind,verification_kind,verified_at_ms,provenance_json,fetched_at_ms,refreshed_at_ms,next_refresh_after_ms,fetch_state,avatar_policy,record_schema,local_revision,created_at_ms,updated_at_ms,state,retired_at_ms,extensions_json) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,'none',1,?,?,?,?,?,?)"} else {"UPDATE social_profiles SET social_profile_id=?1,library_id=?2,person_id=?3,organization_id=?4,provider_id=?5,subject_namespace=?6,provider_subject_id=?7,handle=?8,display_name=?9,profile_url=?10,account_kind=?11,provider_account_kind=?12,verification_kind=?13,verified_at_ms=?14,provenance_json=?15,fetched_at_ms=?16,refreshed_at_ms=?17,next_refresh_after_ms=?18,fetch_state=?19,avatar_policy='none',record_schema=1,local_revision=?20,created_at_ms=?21,updated_at_ms=?22,state=?23,retired_at_ms=?24,extensions_json=?25 WHERE social_profile_id=?1 AND library_id=?2"})
            .bind(m.id.bytes()).bind(m.library_id.bytes()).bind(person).bind(organization).bind(&value.provider_id).bind(value.subject.as_ref().map(|s|&s.namespace)).bind(value.subject.as_ref().map(|s|&s.subject_id)).bind(&value.handle).bind(&value.display_name).bind(&value.profile_url).bind(value.account_kind.sql()).bind(&value.provider_account_kind).bind(value.verification.sql()).bind(value.verified_at.map(Timestamp::get)).bind(canonical(&value.provenance)?).bind(value.fetched_at.map(Timestamp::get)).bind(value.refreshed_at.map(Timestamp::get)).bind(value.next_refresh_after.map(Timestamp::get)).bind(value.fetch_state.sql()).bind(m.revision.get()).bind(m.created_at.get()).bind(m.updated_at.get()).bind(m.state.sql()).bind(m.retired_at()).bind(canonical(&value.extensions)?).execute(&mut *tx).await?;
        let mut result = finish(
            &mut tx,
            self.info.device_id,
            Table::Social,
            m,
            expected,
            value,
        )
        .await?;
        if let Some(handle) = &value.handle {
            // Provider-specific case rules are unknown: Unicode text-key matches are
            // conservative review hints, never identity or a uniqueness constraint.
            let others=sqlx::query("SELECT social_profile_id,handle FROM social_profiles WHERE library_id=? AND provider_id=? AND social_profile_id<>? AND state='active' AND handle IS NOT NULL")
                .bind(m.library_id.bytes()).bind(&value.provider_id).bind(m.id.bytes()).fetch_all(&mut *tx).await?;
            let key = normalize_term(handle)?;
            for row in others {
                if normalize_term(&row.try_get::<String, _>("handle")?)? == key {
                    result
                        .duplicate_manual_handles
                        .push(SocialProfileId::from_bytes(
                            &row.try_get::<Vec<u8>, _>("social_profile_id")?,
                        )?);
                }
            }
        }
        tx.commit().await?;
        Ok(result)
    }
}

async fn put_party<I: Copy + Into<Uuid>>(
    conn: &mut SqliteConnection,
    table: Table,
    m: &Metadata<I>,
    d: &PartyDetails,
) -> Result<()>
where
    Uuid: From<I>,
{
    let (table_name, id, _) = table.names();
    // Only closed table/column constants are formatted; all record values bind.
    let sql = sqlx::AssertSqlSafe(if m.revision == Revision::INITIAL {
        format!(
            "INSERT INTO {table_name}({id},library_id,record_schema,local_revision,created_at_ms,updated_at_ms,state,retired_at_ms,display_name,sort_key,description,aliases_json,extensions_json) VALUES(?,?,1,?,?,?,?,?,?,?,?,?,?)"
        )
    } else {
        format!(
            "UPDATE {table_name} SET {id}=?1,library_id=?2,record_schema=1,local_revision=?3,created_at_ms=?4,updated_at_ms=?5,state=?6,retired_at_ms=?7,display_name=?8,sort_key=?9,description=?10,aliases_json=?11,extensions_json=?12 WHERE {id}=?1 AND library_id=?2"
        )
    });
    sqlx::query(sql)
        .bind(Uuid::from(m.id).as_bytes().to_vec())
        .bind(m.library_id.bytes())
        .bind(m.revision.get())
        .bind(m.created_at.get())
        .bind(m.updated_at.get())
        .bind(m.state.sql())
        .bind(m.retired_at())
        .bind(&d.display_name)
        .bind(normalize_term(&d.display_name)?)
        .bind(&d.description)
        .bind(canonical(&d.aliases)?)
        .bind(canonical(&d.extensions)?)
        .execute(&mut *conn)
        .await?;
    let labels_table = match table {
        Table::Person => "person_labels",
        Table::Organization => "organization_labels",
        _ => return Err(Error::Invalid),
    };
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DELETE FROM {labels_table} WHERE library_id=? AND {id}=?"
    )))
    .bind(m.library_id.bytes())
    .bind(Uuid::from(m.id).as_bytes().to_vec())
    .execute(&mut *conn)
    .await?;
    for label in &d.labels {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO {labels_table} VALUES(?,?,?,?)"
        )))
        .bind(m.library_id.bytes())
        .bind(Uuid::from(m.id).as_bytes().to_vec())
        .bind(normalize_term(label)?)
        .bind(label)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}
async fn rows(
    conn: &mut SqliteConnection,
    table: Table,
    library: LibraryId,
    id: Option<Uuid>,
    after: Option<Uuid>,
    limit: u32,
    retired: bool,
) -> Result<Vec<sqlx::sqlite::SqliteRow>> {
    page(0, limit)?;
    let (name, column, _) = table.names();
    let sql = sqlx::AssertSqlSafe(format!(
        "SELECT * FROM {name} WHERE library_id=? AND (? IS NULL OR {column}=?) AND {column}>? AND (? OR state='active') ORDER BY {column} LIMIT ?"
    ));
    let id = id.map(|id| id.as_bytes().to_vec());
    Ok(sqlx::query(sql)
        .bind(library.bytes())
        .bind(id.clone())
        .bind(id)
        .bind(after.map_or_else(Vec::new, |id| id.as_bytes().to_vec()))
        .bind(retired)
        .bind(limit)
        .fetch_all(conn)
        .await?)
}
fn library(row: &sqlx::sqlite::SqliteRow) -> Result<Library> {
    Ok(Library {
        meta: metadata(row, "library_id")?,
        display_name: row.try_get("display_name")?,
        extensions: json(&row.try_get::<String, _>("extensions_json")?)?,
    })
}
fn details(row: &sqlx::sqlite::SqliteRow) -> Result<PartyDetails> {
    Ok(PartyDetails {
        display_name: row.try_get("display_name")?,
        description: row.try_get("description")?,
        aliases: json(&row.try_get::<String, _>("aliases_json")?)?,
        labels: BTreeSet::new(),
        extensions: json(&row.try_get::<String, _>("extensions_json")?)?,
    })
}
async fn party_labels(
    conn: &mut SqliteConnection,
    table: Table,
    library: LibraryId,
    id: Uuid,
) -> Result<BTreeSet<String>> {
    let (name, column) = match table {
        Table::Person => ("person_labels", "person_id"),
        Table::Organization => ("organization_labels", "organization_id"),
        _ => return Err(Error::Invalid),
    };
    let sql = sqlx::AssertSqlSafe(format!(
        "SELECT display_label FROM {name} WHERE library_id=? AND {column}=? ORDER BY label_key"
    ));
    Ok(sqlx::query_scalar(sql)
        .bind(library.bytes())
        .bind(id.as_bytes().to_vec())
        .fetch_all(conn)
        .await?
        .into_iter()
        .collect())
}
async fn person(conn: &mut SqliteConnection, row: &sqlx::sqlite::SqliteRow) -> Result<Person> {
    let meta: Metadata<PersonId> = metadata(row, "person_id")?;
    let mut details = details(row)?;
    details.labels = party_labels(conn, Table::Person, meta.library_id, meta.id.uuid()).await?;
    let capabilities=sqlx::query_scalar("SELECT capability_id FROM person_capabilities WHERE library_id=? AND person_id=? ORDER BY capability_id").bind(meta.library_id.bytes()).bind(meta.id.bytes()).fetch_all(conn).await?.into_iter().collect();
    Ok(Person {
        meta,
        details,
        capabilities,
    })
}
async fn organization(
    conn: &mut SqliteConnection,
    row: &sqlx::sqlite::SqliteRow,
) -> Result<Organization> {
    let meta: Metadata<OrganizationId> = metadata(row, "organization_id")?;
    let mut details = details(row)?;
    details.labels =
        party_labels(conn, Table::Organization, meta.library_id, meta.id.uuid()).await?;
    Ok(Organization { meta, details })
}
async fn terms(
    conn: &mut SqliteConnection,
    library: LibraryId,
    id: LocationKindId,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    sqlx::query("SELECT term_key,spellings_json FROM location_kind_terms WHERE library_id=? AND location_kind_id=? ORDER BY term_key").bind(library.bytes()).bind(id.bytes()).fetch_all(conn).await?.iter().map(|r|Ok((r.try_get("term_key")?,json(&r.try_get::<String,_>("spellings_json")?)?))).collect()
}
async fn kind(conn: &mut SqliteConnection, row: &sqlx::sqlite::SqliteRow) -> Result<LocationKind> {
    let meta: Metadata<LocationKindId> = metadata(row, "location_kind_id")?;
    let canonical_display: String = row.try_get("canonical_display")?;
    let mut aliases: BTreeSet<String> = terms(conn, meta.library_id, meta.id)
        .await?
        .into_values()
        .flatten()
        .collect();
    aliases.remove(&canonical_display);
    Ok(LocationKind {
        meta,
        canonical_display,
        description: row.try_get("description")?,
        aliases,
        extensions: json(&row.try_get::<String, _>("extensions_json")?)?,
    })
}
async fn relationship(
    _conn: &mut SqliteConnection,
    row: &sqlx::sqlite::SqliteRow,
) -> Result<Relationship> {
    Ok(Relationship {
        meta: metadata(row, "relationship_id")?,
        person_id: PersonId::from_bytes(&row.try_get::<Vec<u8>, _>("person_id")?)?,
        organization_id: OrganizationId::from_bytes(
            &row.try_get::<Vec<u8>, _>("organization_id")?,
        )?,
        relationship_type: row.try_get("relationship_type")?,
        valid_from: time(row, "valid_from_ms")?,
        valid_until: time(row, "valid_until_ms")?,
        notes: row.try_get("notes")?,
        labels: json(&row.try_get::<String, _>("labels_json")?)?,
        extensions: json(&row.try_get::<String, _>("extensions_json")?)?,
    })
}
async fn location(_conn: &mut SqliteConnection, row: &sqlx::sqlite::SqliteRow) -> Result<Location> {
    let coordinates = match (
        row.try_get::<Option<f64>, _>("latitude")?,
        row.try_get::<Option<f64>, _>("longitude")?,
    ) {
        (None, None) => None,
        (Some(latitude), Some(longitude)) => Some(Coordinates {
            latitude,
            longitude,
        }),
        _ => return Err(Error::Corrupt),
    };
    Ok(Location {
        meta: metadata(row, "location_id")?,
        kind_id: LocationKindId::from_bytes(&row.try_get::<Vec<u8>, _>("location_kind_id")?)?,
        parent_id: row
            .try_get::<Option<Vec<u8>>, _>("parent_location_id")?
            .map(|b| LocationId::from_bytes(&b))
            .transpose()?,
        details: details(row)?,
        address: json(&row.try_get::<String, _>("address_json")?)?,
        coordinates,
    })
}
async fn social(
    _conn: &mut SqliteConnection,
    row: &sqlx::sqlite::SqliteRow,
) -> Result<SocialProfile> {
    if row.try_get::<String, _>("avatar_policy")? != "none"
        || row
            .try_get::<Option<Vec<u8>>, _>("avatar_media_sha256")?
            .is_some()
    {
        return Err(Error::Unsupported);
    }
    let owner = match (
        row.try_get::<Option<Vec<u8>>, _>("person_id")?,
        row.try_get::<Option<Vec<u8>>, _>("organization_id")?,
    ) {
        (Some(p), None) => SocialOwner::Person(PersonId::from_bytes(&p)?),
        (None, Some(o)) => SocialOwner::Organization(OrganizationId::from_bytes(&o)?),
        _ => return Err(Error::Corrupt),
    };
    let subject = match (
        row.try_get::<Option<String>, _>("subject_namespace")?,
        row.try_get::<Option<String>, _>("provider_subject_id")?,
    ) {
        (Some(namespace), Some(subject_id)) => Some(ProviderSubject {
            namespace,
            subject_id,
        }),
        (None, None) => None,
        _ => return Err(Error::Corrupt),
    };
    Ok(SocialProfile {
        meta: metadata(row, "social_profile_id")?,
        owner,
        provider_id: row.try_get("provider_id")?,
        subject,
        handle: row.try_get("handle")?,
        display_name: row.try_get("display_name")?,
        profile_url: row.try_get("profile_url")?,
        account_kind: AccountKind::parse(&row.try_get::<String, _>("account_kind")?)?,
        provider_account_kind: row.try_get("provider_account_kind")?,
        verification: VerificationKind::parse(&row.try_get::<String, _>("verification_kind")?)?,
        verified_at: time(row, "verified_at_ms")?,
        provenance: json(&row.try_get::<String, _>("provenance_json")?)?,
        fetched_at: time(row, "fetched_at_ms")?,
        refreshed_at: time(row, "refreshed_at_ms")?,
        next_refresh_after: time(row, "next_refresh_after_ms")?,
        fetch_state: FetchState::parse(&row.try_get::<String, _>("fetch_state")?)?,
        extensions: json(&row.try_get::<String, _>("extensions_json")?)?,
    })
}
fn time(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<Option<Timestamp>> {
    row.try_get::<Option<i64>, _>(column)?
        .map(Timestamp::try_from)
        .transpose()
}

macro_rules! reads {
    ($get:ident,$list:ident,$id:ident,$record:ident,$table:ident,$decode:ident) => {
        impl LocalLibraryStore {
            /// Reads one typed record, including a retained tombstone.
            /// # Errors
            /// Returns storage, unsupported record or decoding failure.
            pub async fn $get(&self, library: LibraryId, id: $id) -> Result<Option<$record>> {
                let mut tx = self.read().await?;
                let rows = rows(
                    &mut tx,
                    Table::$table,
                    library,
                    Some(id.uuid()),
                    None,
                    1,
                    true,
                )
                .await?;
                match rows.first() {
                    Some(row) => Ok(Some($decode(&mut tx, row).await?)),
                    None => Ok(None),
                }
            }
            /// Stable UUID keyset page. Limit must be 1..500.
            /// # Errors
            /// Returns invalid bounds or storage/decoding failure.
            pub async fn $list(
                &self,
                library: LibraryId,
                after: Option<$id>,
                limit: u32,
                retired: bool,
            ) -> Result<Vec<$record>> {
                let mut tx = self.read().await?;
                let rows = rows(
                    &mut tx,
                    Table::$table,
                    library,
                    None,
                    after.map($id::uuid),
                    limit,
                    retired,
                )
                .await?;
                let mut records = Vec::with_capacity(rows.len());
                for row in rows {
                    records.push($decode(&mut tx, &row).await?);
                }
                Ok(records)
            }
        }
    };
}
reads!(person, people, PersonId, Person, Person, person);
reads!(
    organization,
    organizations,
    OrganizationId,
    Organization,
    Organization,
    organization
);
reads!(
    relationship,
    relationships,
    RelationshipId,
    Relationship,
    Relationship,
    relationship
);
reads!(
    location_kind,
    location_kinds,
    LocationKindId,
    LocationKind,
    Kind,
    kind
);
reads!(
    location, locations, LocationId, Location, Location, location
);
reads!(
    social_profile,
    social_profiles,
    SocialProfileId,
    SocialProfile,
    Social,
    social
);
