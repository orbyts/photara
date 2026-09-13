use super::*;

/// Validate a 1.0 or 1.1 package without rewriting any object or resolving external bytes.
/// # Errors
/// Fails closed on unsupported schemas, missing features, unsafe paths or invalid closure.
#[expect(clippy::too_many_lines, reason = "Auditable bounded commit walk")]
pub fn validate_directory(
    root: impl AsRef<Path>,
    limits: PackageLimits,
) -> Result<ValidatedPackageV1_1, PackageError> {
    let reader = super::super::reader::Reader::open(root.as_ref(), limits)?;
    let bootstrap_bytes = reader.json("manifest.json")?;
    let bootstrap_value = parse_canonical_json(&bootstrap_bytes, limits.json)?;
    super::super::records::uuid_field(&bootstrap_value, "project_id")?;
    let bootstrap: Bootstrap = decode(&bootstrap_value)?;
    if bootstrap.format != "photara.project-package"
        || bootstrap.canonical_json != "photara.canonical-json.v1"
    {
        return Err(PackageError::UnsupportedVersion);
    }
    format(&bootstrap.format_version)?;
    features(&bootstrap.required_features)?;
    super::super::records::timestamp(&bootstrap.created_at)?;
    let head_bytes = reader.json("HEAD.json")?;
    let head_value = parse_canonical_json(&head_bytes, limits.json)?;
    let head: Head = decode(&head_value)?;
    super::super::records::uuid_field(&head_value, "project_id")?;
    schema(&head.schema, "photara.package.head")?;
    if head.project_id != bootstrap.project_id {
        return Err(PackageError::Integrity);
    }
    let mut context = Context {
        generation_two: true,
        reader,
        limits,
        project_id: bootstrap.project_id,
        objects: BTreeMap::new(),
        blobs: BTreeMap::new(),
        json_bytes: bootstrap_bytes.len() + head_bytes.len(),
        blob_bytes: 0,
        diagnostics: Vec::new(),
    };
    let bootstrap_hash = digest(&bootstrap_bytes);
    let mut pending = Some(CommitParent {
        commit_id: head.commit_id,
        sha256: head.commit_sha256.clone(),
        extra: BTreeMap::new(),
    });
    let mut seen = BTreeSet::new();
    let mut commits: Vec<Commit> = Vec::new();
    while let Some(parent) = pending.take() {
        if commits.len() >= limits.max_commits {
            return Err(PackageError::Limit);
        }
        if !seen.insert(parent.commit_id) {
            return Err(PackageError::Integrity);
        }
        let bytes = context
            .reader
            .json(&format!("commits/{}.json", parent.commit_id))?;
        context.add_json(bytes.len())?;
        if digest(&bytes) != parent.sha256 {
            return Err(PackageError::Integrity);
        }
        let value = parse_canonical_json(&bytes, limits.json)?;
        super::super::records::uuid_field(&value, "project_id")?;
        let commit: Commit = decode(&value)?;
        schema(&commit.schema, "photara.package.commit")?;
        features(&commit.required_features)?;
        if bootstrap
            .required_features
            .iter()
            .any(|f| FEATURES.contains(&f.as_str()) && !commit.required_features.contains(f))
            || commits.last().is_some_and(|newer| {
                commit
                    .required_features
                    .iter()
                    .any(|f| !newer.required_features.contains(f))
            })
        {
            return Err(PackageError::UnsupportedFeature);
        }
        format(&commit.minimum_reader)?;
        super::super::records::timestamp(&commit.created_at)?;
        if commit.project_id != bootstrap.project_id
            || commit.commit_id != parent.commit_id
            || commit.bootstrap_sha256 != bootstrap_hash
            || commit.package_revision.get() == 0
        {
            return Err(PackageError::Integrity);
        }
        if let Some(newer) = commits.last()
            && commit.package_revision.get().checked_add(1) != Some(newer.package_revision.get())
        {
            return Err(PackageError::Integrity);
        }
        if commit.parent.is_none() && commit.package_revision.get() != 1 {
            return Err(PackageError::Integrity);
        }
        let inventory_value = context.load_json(&commit.inventory)?;
        let inventory: Inventory = decode(&inventory_value)?;
        schema(&inventory.schema, "photara.package.inventory")?;
        if inventory.project_id != bootstrap.project_id {
            return Err(PackageError::Integrity);
        }
        if inventory.objects.len() > limits.max_objects {
            return Err(PackageError::Limit);
        }
        if inventory.objects.windows(2).any(|p| p[0] >= p[1]) {
            return Err(PackageError::Integrity);
        }
        let closure = context.closure(&[commit.authored.clone(), commit.history.clone()])?;
        if closure != inventory.objects.into_iter().collect() {
            return Err(PackageError::Integrity);
        }
        let authored_value = context.load_json(&commit.authored)?;
        validate_commit(&commit, &closure, &context)?;
        let _ = authored_value;
        pending.clone_from(&commit.parent);
        commits.push(commit);
    }
    validate_links(&context)?;
    let current = commits.first().ok_or(PackageError::Integrity)?;
    let authored = context.object(&current.authored)?.clone();
    let graphs = array(&authored.value, "graphs")?
        .iter()
        .map(|g| context.object(&reference(g, "document")?).cloned())
        .collect::<Result<_, PackageError>>()?;
    if context.reader.json("HEAD.json")? != head_bytes
        || context.reader.json("manifest.json")? != bootstrap_bytes
    {
        return Err(PackageError::ChangedDuringRead);
    }
    Ok(ValidatedPackageV1_1 {
        bootstrap,
        head,
        commits,
        authored,
        graphs,
        objects: context.objects,
        verified_blobs: context.blobs.len(),
        diagnostics: context.diagnostics,
    })
}
