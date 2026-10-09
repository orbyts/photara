use super::{
    Connection, Digest, JsonLimits, LocalDatabase, OptionalExtension, Registration, Result, Sha256,
    Transaction, TransactionBehavior, Uuid, Value, bootstrap_ids, canonical, decimal, ensure, err,
    fields, hex, json, now, params, parse_canonical_json, sha, sql, uuid,
};
fn domain(label: &str, bytes: &[u8]) -> Vec<u8> {
    let mut sha = Sha256::new();
    sha.update(label.as_bytes());
    sha.update([0]);
    sha.update(bytes);
    sha.finalize().to_vec()
}
fn parse_request(
    reg: &Registration,
    value: &Value,
    action: &str,
) -> Result<(Uuid, Uuid, Vec<u8>, Vec<u8>)> {
    let mut names = vec![
        "schema",
        "kind",
        "authority",
        "principal",
        "operation_id",
        "initiating_device_id",
        "library_id",
        "name",
    ];
    if action == "rename" {
        names.push("expected_revision");
    }
    fields(value, &names)?;
    fields(&value["authority"], &["kind", "database_id", "epoch"])?;
    fields(&value["principal"], &["kind", "id"])?;
    ensure(
        value["schema"] == "photara.library-lifecycle.v1"
            && value["kind"] == action
            && value["authority"] == reg.authority()
            && value["principal"] == reg.principal(),
    )?;
    let operation = uuid(&value["operation_id"])?;
    let library = uuid(&value["library_id"])?;
    uuid(&value["initiating_device_id"])?;
    let name = value["name"]
        .as_str()
        .ok_or_else(|| err("Library name expected"))?;
    ensure(
        name == name.trim()
            && !name.is_empty()
            && name.len() <= 128
            && !name.chars().any(char::is_control),
    )?;
    if action == "rename" {
        ensure(decimal(&value["expected_revision"])? > 0)?;
    }
    let bytes = canonical(value)?;
    ensure(bytes.len() <= 65536)?;
    let parsed = parse_canonical_json(
        &bytes,
        JsonLimits {
            max_bytes: 65536,
            max_depth: 16,
            max_members: 16,
            max_array_elements: 0,
        },
    )
    .map_err(|_| err("lifecycle bounds"))?;
    ensure(parsed == *value)?;
    let hash = domain("photara.library-lifecycle.request.v1", &bytes);
    Ok((operation, library, bytes, hash))
}
pub(super) fn local(conn: &Connection, reg: &Registration, library: Uuid) -> Result<()> {
    ensure(local_allowed(conn, reg, library)?)
}
fn local_allowed(conn: &Connection, reg: &Registration, library: Uuid) -> Result<bool> {
    let allowed: bool = conn
        .query_row(
            crate::gen2::LOCAL_AUTHORITY_SQL,
            params![library.as_bytes(), reg.principal_id.as_bytes()],
            |r| r.get(0),
        )
        .map_err(sql)?;
    Ok(allowed)
}
fn authenticate(conn: &Connection, reg: &Registration) -> Result<()> {
    let allowed:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM library_contract_state c JOIN libraries l USING(library_id) WHERE c.authority_mode='local-only' AND c.local_principal_id=? AND l.state='active')",[reg.principal_id.as_bytes()],|r|r.get(0)).map_err(sql)?;
    ensure(allowed)
}
fn inventory(
    tx: &Transaction<'_>,
    reg: &Registration,
    library: Uuid,
    expected: Option<i64>,
    at: i64,
) -> Result<()> {
    let (name, revision, created): (String, i64, i64) = tx
        .query_row(
            "SELECT display_name,local_revision,created_at_ms FROM libraries WHERE library_id=?",
            [library.as_bytes()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(sql)?;
    let post = canonical(
        &json!({"meta":{"id":library,"library_id":library,"revision":revision,"created_at":created,"updated_at":at,"state":"active"},"display_name":name,"extensions":{}}),
    )?;
    let change = if expected.is_none() {
        "create"
    } else {
        "update"
    };
    let envelope = canonical(
        &json!({"schema":"photara.local-command.v1","kind":"library","entity_id":library,"expected_local_revision":expected,"change":change,"post_state_sha256":hex(&sha(&post))}),
    )?;
    let mutation = Uuid::new_v4();
    tx.execute("INSERT INTO mutations(mutation_id,library_id,source_device_id,origin,command_schema,primary_entity_kind,primary_entity_id,created_at_ms,envelope_json,envelope_sha256) VALUES(?,?,?,'local',1,'library',?,?,?,?)",params![mutation.as_bytes(),library.as_bytes(),reg.device_id.as_bytes(),library.as_bytes(),at,String::from_utf8(envelope.clone()).map_err(|_|err("UTF8"))?,sha(&envelope)]).map_err(sql)?;
    tx.execute("INSERT INTO local_changes(library_id,mutation_id,entity_kind,entity_id,local_revision,change_kind,changed_at_ms,post_state_json,post_state_sha256) VALUES(?,?,'library',?,?,?,?,?,?)",params![library.as_bytes(),mutation.as_bytes(),library.as_bytes(),revision,change,at,String::from_utf8(post.clone()).map_err(|_|err("UTF8"))?,sha(&post)]).map_err(sql)?;
    Ok(())
}
impl LocalDatabase {
    #[expect(
        clippy::too_many_lines,
        reason = "One atomic authority, effect, inventory and original-receipt transaction"
    )]
    pub(super) fn lifecycle(&mut self, request: &Value, action: &str) -> Result<Value> {
        self.check_pins()?;
        self.settings()?;
        self.capacity()?;
        let (operation, library, bytes, hash) = parse_request(&self.registration, request, action)?;
        let reg = &self.registration;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql)?;
        authenticate(&tx, reg)?;
        let prior:Option<(Vec<u8>,Vec<u8>,Vec<u8>)>=tx.query_row("SELECT request_sha256,result_canonical,result_sha256 FROM lifecycle_receipts WHERE authority_id=? AND principal_kind='local' AND principal_id=? AND operation_id=?",params![reg.epoch.as_bytes(),reg.principal_id.as_bytes(),operation.as_bytes()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(sql)?;
        if let Some((original, receipt, digest)) = prior {
            ensure(
                original == hash
                    && domain("photara.library-lifecycle.receipt.v1", &receipt) == digest,
            )?;
            let value = parse_canonical_json(
                &receipt,
                JsonLimits {
                    max_bytes: 65536,
                    max_depth: 16,
                    max_members: 16,
                    max_array_elements: 0,
                },
            )
            .map_err(|_| err("stored receipt corrupt"))?;
            tx.commit().map_err(sql)?;
            return Ok(value);
        }
        let at = now()?;
        let initiating = uuid(&request["initiating_device_id"])?;
        ensure(initiating == reg.device_id)?;
        tx.execute(
            "INSERT INTO lifecycle_intents VALUES(?,'local',?,?,?,?,?,?,?,'prepared',?,?)",
            params![
                reg.epoch.as_bytes(),
                reg.principal_id.as_bytes(),
                operation.as_bytes(),
                library.as_bytes(),
                action,
                bytes,
                hash,
                initiating.as_bytes(),
                at,
                at
            ],
        )
        .map_err(sql)?;
        let name = request["name"].as_str().ok_or_else(|| err("name"))?;
        let result = if action == "create" {
            let default = bootstrap_ids(reg.database_id).0;
            if !local_allowed(&tx, reg, default)? {
                json!({"kind":"rejected","reason":"authority-denied"})
            } else if tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM libraries WHERE library_id=?)",
                    [library.as_bytes()],
                    |r| r.get::<_, bool>(0),
                )
                .map_err(sql)?
            {
                json!({"kind":"rejected","reason":"library-id-unavailable"})
            } else {
                let policy:(String,Vec<u8>)=tx.query_row("SELECT unicode_version,rules_sha256 FROM normalization_policies WHERE policy_version=1",[],|r|Ok((r.get(0)?,r.get(1)?))).map_err(sql)?;
                ensure(policy.0 == "16.0.0" && policy.1 == sha(crate::gen2::POLICY.as_bytes()))?;
                tx.execute(
                    "INSERT INTO libraries VALUES(?,?,'active',1,?,?,NULL,1,'{}')",
                    params![library.as_bytes(), name, at, at],
                )
                .map_err(sql)?;
                tx.execute(
                    "INSERT INTO library_contract_state VALUES(?,1,'local-only',?,1,1,1,?,?)",
                    params![library.as_bytes(), reg.principal_id.as_bytes(), at, at],
                )
                .map_err(sql)?;
                inventory(&tx, reg, library, None, at)?;
                json!({"kind":"created","revision":"1"})
            }
        } else if !local_allowed(&tx, reg, library)? {
            json!({"kind":"rejected","reason":"authority-denied"})
        } else {
            let expected = decimal(&request["expected_revision"])?;
            let current: i64 = tx
                .query_row(
                    "SELECT local_revision FROM libraries WHERE library_id=?",
                    [library.as_bytes()],
                    |r| r.get(0),
                )
                .map_err(sql)?;
            if current == expected {
                let next = expected
                    .checked_add(1)
                    .ok_or_else(|| err("revision overflow"))?;
                ensure(tx.execute("UPDATE libraries SET display_name=?,local_revision=?,updated_at_ms=? WHERE library_id=? AND local_revision=?",params![name,next,at,library.as_bytes(),expected]).map_err(sql)?==1)?;
                inventory(&tx, reg, library, Some(expected), at)?;
                json!({"kind":"renamed","previous_revision":expected.to_string(),"revision":next.to_string()})
            } else {
                json!({"kind":"rejected","reason":"revision-conflict"})
            }
        };
        let receipt = json!({"schema":"photara.library-lifecycle.v1","kind":"receipt","authority":request["authority"],"principal":request["principal"],"operation_id":request["operation_id"],"initiating_device_id":request["initiating_device_id"],"library_id":request["library_id"],"action":action,"request_sha256":hex(&hash),"committed_at_ms":at.to_string(),"result":result});
        let canonical = canonical(&receipt)?;
        ensure(canonical.len() <= 65536)?;
        let reviewed = result.get("revision").map(decimal).transpose()?;
        tx.execute(
            "INSERT INTO lifecycle_receipts VALUES(?,'local',?,?,?,?,?,?,?,?,?,?,?)",
            params![
                reg.epoch.as_bytes(),
                reg.principal_id.as_bytes(),
                operation.as_bytes(),
                library.as_bytes(),
                action,
                result["kind"].as_str(),
                hash,
                canonical,
                domain("photara.library-lifecycle.receipt.v1", &canonical),
                initiating.as_bytes(),
                at,
                reviewed
            ],
        )
        .map_err(sql)?;
        ensure(tx.execute("UPDATE lifecycle_intents SET state='terminal',updated_at=? WHERE authority_id=? AND principal_kind='local' AND principal_id=? AND operation_id=?",params![at,reg.epoch.as_bytes(),reg.principal_id.as_bytes(),operation.as_bytes()]).map_err(sql)?==1)?;
        Self::trip(&mut self.fault, "before-commit")?;
        tx.commit().map_err(sql)?;
        Self::trip(&mut self.fault, "after-commit")?;
        self.check_pins()?;
        self.capacity()?;
        Ok(receipt)
    }
}
