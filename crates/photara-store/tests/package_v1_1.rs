//! Synthetic packages are written only below newly allocated disposable roots.
use photara_core::{canonical_json, context as cx};
use photara_store::package::{self, PackageError, PackageLimits};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};
fn id(n: u32) -> String {
    format!("62000000-0000-4000-8000-{n:012}")
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn canon(v: &Value) -> Vec<u8> {
    canonical_json(v).unwrap()
}
fn r(name: &str) -> Value {
    json!({"$ref":name})
}
fn decode<T: serde::de::DeserializeOwned>(v: Value) -> T {
    serde_json::from_value(v).unwrap()
}
const TIME: &str = "2026-09-12T00:00:00.000Z";
fn record(k: &str, version: u32, mut v: Value) -> Value {
    v["schema"] = json!({"id":k,"version":version});
    v["project_id"] = json!(id(20000));
    v
}
#[expect(
    clippy::too_many_lines,
    reason = "Declarative synthetic object specimen"
)]
fn fixture_objects() -> BTreeMap<String, Value> {
    let project = id(20000);
    let library = id(20001);
    let scope = json!({"kind":"project","project_id":project});
    let mut objects = BTreeMap::new();
    let mut add = |name: &str, k: &str, ver: u32, v: Value| {
        objects.insert(name.into(), record(k, ver, v));
    };
    add(
        "parties",
        "photara.project.party-assignments",
        1,
        json!({"assignments":[]}),
    );
    add(
        "locations",
        "photara.project.location-assignments",
        1,
        json!({"assignments":[]}),
    );
    let media = json!({"media_type":"image/png","byte_length":"5"});
    add(
        "managed",
        "photara.project.managed-resource",
        1,
        json!({"resource_id":id(20002),"version_id":id(20003),"blob":{"kind":"blob","sha256":hash(b"photo"),"byte_length":"5"},"media":media,"retention":"managed-project","original_name":"image.png","provenance":{}}),
    );
    add(
        "external",
        "photara.project.external-resource",
        1,
        json!({"external_ref_id":id(20004),"revision_id":id(20005),"source_library_id":library,"storage_location_id":id(20006),"coordinate":{"kind":"filesystem","components":["image.png"]},"content_evidence":{"kind":"unverified"},"storage_class":"external-source","provenance":{}}),
    );
    let fingerprint = json!({"kind":"content-digest","algorithm":"sha256","value":hash(b"photo"),"evidence_strength":"verified-bytes"});
    add(
        "content",
        "photara.project.representation-content",
        2,
        json!({"asset_id":id(20007),"representation_id":id(20008),"content_revision_id":id(20009),"media":media,"fingerprint":fingerprint,"lineage":[],"binding":{"kind":"managed","resource_id":id(20002),"version_id":id(20003),"version":r("managed")}}),
    );
    add(
        "assets",
        "photara.project.asset-ledger",
        1,
        json!({"assets":[{"asset_id":id(20007),"display_name":"Image","revision":"1","created_at":TIME,"updated_at":TIME,"representations":[{"representation_id":id(20008),"roles":["original"],"capabilities":[],"current_content":r("content"),"retained_content":[]}]}]}),
    );
    add(
        "resources",
        "photara.project.resource-ledger",
        1,
        json!({"managed_resources":[r("managed")],"external_resources":[r("external")],"artifacts":[]}),
    );
    let typed = json!({"ty":cx::value::Type::string(),"value":{"kind":"string","value":"red"}});
    add(
        "observation",
        "photara.metadata.observation",
        1,
        json!({"observation_id":id(20010),"asset_id":id(20007),"representation_id":id(20008),"content_revision_id":id(20009),"fingerprint":fingerprint,"field_schema":{"id":"photara.string","version":1},"field":"example.color","value":typed,"provenance":{"extractor":"example.test","version":1,"digest":hash(b"extractor")},"captured_at":TIME,"sensitivity":"ordinary","portability":"portable"}),
    );
    add(
        "page",
        "photara.value.asset-set-page",
        1,
        json!({"snapshot_id":id(20011),"start_ordinal":0,"members":[{"asset_id":id(20007),"representations":[{"representation_id":id(20008),"content_revision_id":id(20009),"descriptor":r("content")}],"metadata_refs":[r("observation")],"missing_facts":[]}]}),
    );
    add(
        "set",
        "photara.value.asset-set-snapshot",
        1,
        json!({"snapshot_id":id(20011),"member_count":1,"content_digest":"AUTO","pages":[{"start_ordinal":0,"count":1,"page":r("page")}],"provenance":{}}),
    );
    add(
        "groups",
        "photara.value.group-set",
        1,
        json!({"group_snapshot_id":id(20012),"input_snapshot":r("set"),"input_digest":"AUTO","groups":[{"group_id":id(20013),"label":"Selected","member_asset_ids":[id(20007)]}]}),
    );
    add(
        "patch",
        "photara.metadata.patch",
        1,
        json!({"patch_id":id(20014),"input_snapshot":r("set"),"input_digest":"AUTO","target":{"kind":"group","group_snapshot":r("groups"),"group_id":id(20013)},"operations":[{"kind":"replace","field_schema":{"id":"photara.string","version":1},"value":typed,"preconditions":[{"asset_id":id(20007),"representation_id":id(20008),"content_revision_id":id(20009),"fingerprint":fingerprint,"descriptor":r("content")}]}],"provenance":{}}),
    );
    let env: cx::expression::BindingEnvironment = decode(
        json!({"owner":scope,"owning_library_id":library,"owner_revision":{"kind":"local","revision":1},"run_id":null,"asset_id":null,"bindings":[],"queries":[]}),
    );
    let expression = cx::expression::Expression::compile(
        decode(json!(id(20015))),
        cx::expression::FieldMode::Expression,
        "`\"red\"`",
        env,
        &cx::value::Type::string(),
    )
    .unwrap();
    let mut e = serde_json::to_value(expression.record()).unwrap();
    e["owner"] = scope.clone();
    e["source_utf8"] = e.as_object_mut().unwrap().remove("source").unwrap();
    e["bound_dependencies"] = e.as_object_mut().unwrap().remove("dependencies").unwrap();
    add("expression", "photara.context.expression", 1, e);
    add(
        "variable",
        "photara.context.variable",
        1,
        json!({"variable_id":id(20016),"scope":scope,"owning_library_id":library,"definition_version":1,"namespace":"example.project","name":"color","names":["color"],"label":"Color","description":"","ty":cx::value::Type::string(),"default":{"kind":"expression","value":r("expression")},"current_value":null,"value_id":null,"revision":1,"owner_revision":{"kind":"local","revision":1},"state":"active","allow_run_override":false,"sensitivity":"ordinary","portability":"portable","origin":"manual","created_at":TIME,"updated_at":TIME,"provenance":{}}),
    );
    let snap=cx::snapshot::ContextSnapshot::build(decode(json!({"snapshot_id":id(20017),"owning_library_id":library,"project_id":project,"captured_at":TIME,"versions":{"context":1,"expression":1,"interpreter":1,"query":1},"audience":{"project_id":project,"policy_digest":hash(b"local")},"destination":"local-project","consent":null,"roots":[],"entries":[],"device_required":false,"secret_requirements":[]}))).unwrap();
    let wire = serde_json::to_value(&snap).unwrap();
    let mut snapshot = wire["spec"].clone();
    for k in ["content_digest", "completeness", "replayability"] {
        snapshot[k] = wire[k].clone();
    }
    snapshot["captures"] = json!([r("set")]);
    snapshot["expressions"] = json!([]);
    add("snapshot", "photara.context.snapshot", 1, snapshot);
    add(
        "context",
        "photara.context.authored",
        1,
        json!({"scope":scope,"variables":[r("variable")],"expressions":[r("expression")],"captures":[r("snapshot")],"metadata_selections":[r("set"),r("groups"),r("patch")],"node_contexts":[]}),
    );
    add(
        "graph-context",
        "photara.context.authored",
        1,
        json!({"scope":{"kind":"graph","project_id":project,"graph_id":id(20018)},"variables":[],"expressions":[],"captures":[],"metadata_selections":[],"node_contexts":[]}),
    );
    add(
        "graph",
        "photara.project.saved-graph",
        2,
        json!({"graph_id":id(20018),"name":"Main","name_normalization_version":1,"metadata_revision":"1","created_at":TIME,"updated_at":TIME,"required_packages":[],"graph":{"schema_version":1,"id":id(20018),"revision":0,"nodes":[],"connections":[]},"context":r("graph-context"),"node_contracts":[]}),
    );
    add(
        "authored",
        "photara.project.authored",
        2,
        json!({"owning_library_id":library,"origin":{"library_id":null,"source_format":"photara.project-package.1.1"},"created_at":TIME,"updated_at":TIME,"title":"D19 package specimen","description":"Synthetic","lifecycle":"active","authored_revision":"1","party_assignments":r("parties"),"location_assignments":r("locations"),"asset_ledger":r("assets"),"resource_ledger":r("resources"),"context":r("context"),"graphs":[{"graph_id":id(20018),"document":r("graph")}]}),
    );
    add(
        "history",
        "photara.project.history",
        2,
        json!({"runs":[],"operations":[],"evidence":[],"snapshots":[r("snapshot")],"metadata_observations":[r("observation")],"proposals":[],"apply_receipts":[],"artifacts":[]}),
    );
    objects
}
struct Builder {
    objects: BTreeMap<String, Value>,
    resolved: BTreeMap<String, Value>,
    files: BTreeMap<String, Vec<u8>>,
    refs: BTreeSet<package::ObjectRef>,
}
impl Builder {
    fn object(&mut self, name: &str) -> Value {
        if let Some(r) = self.resolved.get(name) {
            return r.clone();
        }
        let mut v = self.objects[name].clone();
        self.walk(&mut v);
        if v["schema"]["id"] == "photara.value.asset-set-snapshot" && v["content_digest"] == "AUTO"
        {
            let page = self.read(&v["pages"][0]["page"]);
            let mut members = Vec::new();
            for (ordinal, m) in page["members"].as_array().unwrap().iter().enumerate() {
                let mut member = m.clone();
                member["ordinal"] = json!(ordinal);
                for rep in member["representations"].as_array_mut().unwrap() {
                    rep["project_id"] = v["project_id"].clone();
                    rep["asset_id"] = m["asset_id"].clone();
                }
                let metadata:Vec<_>=m["metadata_refs"].as_array().unwrap().iter().map(|r|{let obs=self.read(r);json!({"project_id":v["project_id"],"asset_id":m["asset_id"],"target":{"kind":"representation","representation_id":obs["representation_id"],"content_revision_id":obs["content_revision_id"]},"object":r})}).collect();
                member.as_object_mut().unwrap().remove("metadata_refs");
                member["metadata"] = json!(metadata);
                members.push(member);
            }
            // Hash even intentionally invalid members so negative cases reach reader semantics.
            v["content_digest"] = json!(hash(&canon(
                &json!({"members":members,"project_id":v["project_id"]})
            )));
        }
        if v.get("input_digest") == Some(&json!("AUTO")) {
            v["input_digest"] = self.read(&v["input_snapshot"])["content_digest"].clone();
        }
        if v["schema"]["id"] == "photara.context.apply-receipt" && v["request_digest"] == "AUTO" {
            let reference = self.object("proposal");
            let mut proposal = self.read(&reference);
            proposal.as_object_mut().unwrap().remove("schema");
            let checked: cx::proposal::VariableChangeProposal = decode(proposal);
            v["request_digest"] = json!(checked.request_digest().unwrap());
        }
        let b = canon(&v);
        let r = json!({"kind":"json","sha256":hash(&b),"byte_length":b.len().to_string()});
        self.files
            .insert(decode::<package::ObjectRef>(r.clone()).path(), b);
        self.refs.insert(decode(r.clone()));
        self.resolved.insert(name.into(), r.clone());
        r
    }
    fn read(&self, r: &Value) -> Value {
        serde_json::from_slice(&self.files[&decode::<package::ObjectRef>(r.clone()).path()])
            .unwrap()
    }
    fn walk(&mut self, v: &mut Value) {
        if let Some(name) = v.get("$ref").and_then(Value::as_str) {
            *v = self.object(name);
        } else if let Some(xs) = v.as_array_mut() {
            for x in xs {
                self.walk(x);
            }
        } else if let Some(xs) = v.as_object_mut() {
            for x in xs.values_mut() {
                self.walk(x);
            }
        }
    }
}
fn build(mut edit: impl FnMut(&mut BTreeMap<String, Value>)) -> BTreeMap<String, Vec<u8>> {
    let mut objects = fixture_objects();
    edit(&mut objects);
    let mut b = Builder {
        objects,
        resolved: BTreeMap::new(),
        files: BTreeMap::new(),
        refs: BTreeSet::new(),
    };
    let blob: package::ObjectRef =
        decode(json!({"kind":"blob","sha256":hash(b"photo"),"byte_length":"5"}));
    b.files.insert(blob.path(), b"photo".to_vec());
    b.refs.insert(blob);
    let authored = b.object("authored");
    let history = b.object("history");
    let inventory = record("photara.package.inventory", 1, json!({"objects":b.refs}));
    let bytes = canon(&inventory);
    let inventory_ref =
        json!({"kind":"json","sha256":hash(&bytes),"byte_length":bytes.len().to_string()});
    b.files.insert(
        decode::<package::ObjectRef>(inventory_ref.clone()).path(),
        bytes,
    );
    let features = json!([
        "photara.asset-set.v2",
        "photara.context.v1",
        "photara.history.v1",
        "photara.immutable-objects.v1",
        "photara.library-project.v1",
        "photara.node-contract.v2",
        "photara.resources.v2"
    ]);
    let manifest = json!({"format":"photara.project-package","format_version":{"major":1,"minor":1},"project_id":id(20000),"created_at":TIME,"canonical_json":"photara.canonical-json.v1","required_features":features});
    let manifest_bytes = canon(&manifest);
    let commit = record(
        "photara.package.commit",
        1,
        json!({"commit_id":id(20019),"package_revision":"1","parent":null,"write_id":id(20020),"created_at":TIME,"bootstrap_sha256":hash(&manifest_bytes),"minimum_reader":{"major":1,"minor":1},"required_features":features,"authored":authored,"history":history,"inventory":inventory_ref}),
    );
    let commit_bytes = canon(&commit);
    let head = record(
        "photara.package.head",
        1,
        json!({"commit_id":id(20019),"commit_sha256":hash(&commit_bytes)}),
    );
    b.files.insert("manifest.json".into(), manifest_bytes);
    b.files
        .insert(format!("commits/{}.json", id(20019)), commit_bytes);
    b.files.insert("HEAD.json".into(), canon(&head));
    b.files
}
fn materialize(files: &BTreeMap<String, Vec<u8>>) -> tempfile::TempDir {
    let root = tempfile::Builder::new()
        .prefix("photara-cxt3a-")
        .tempdir()
        .unwrap();
    for (p, b) in files {
        assert!(
            std::path::Path::new(p)
                .components()
                .all(|x| matches!(x, std::path::Component::Normal(_)))
        );
        let target = root.path().join(p);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, b).unwrap();
    }
    root
}
fn validate(
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<package::v1_1::ValidatedPackageV1_1, PackageError> {
    let root = materialize(files);
    package::v1_1::validate_directory(root.path(), PackageLimits::default())
}
#[test]
fn complete_fixture_validates() {
    let files = build(|_| {});
    let result = validate(&files).unwrap();
    assert_eq!(result.authored.value["schema"]["version"], 2);
    assert_eq!(result.verified_blobs, 1);
    for o in result.objects.values() {
        assert_eq!(files[&o.reference.path()], o.canonical_bytes);
    }
}
#[test]
fn old_reader_refuses_new_package() {
    let root = materialize(&build(|_| {}));
    assert_eq!(
        package::validate_directory(root.path(), PackageLimits::default()).unwrap_err(),
        PackageError::UnsupportedVersion
    );
}
#[test]
fn legacy_specimen_still_validates_with_both_apis() {
    let archive: Value = serde_json::from_str(include_str!(
        "../../../docs/fixtures/generation-two/package-specimen.json"
    ))
    .unwrap();
    let files = archive["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["path"].as_str().unwrap().to_owned(),
                f["utf8"].as_str().unwrap().as_bytes().to_vec(),
            )
        })
        .collect();
    let root = materialize(&files);
    let old = package::validate_directory(root.path(), PackageLimits::default()).unwrap();
    let new = validate(&files).unwrap();
    assert_eq!(old.objects.len(), new.objects.len());
    for (k, o) in old.objects {
        assert_eq!(o.canonical_bytes, new.objects[&k].canonical_bytes);
    }
}
#[test]
fn semantic_rejections_after_rehash() {
    let cases: Vec<(&str, &str, Value)> = vec![
        ("authored", "owning_library_id", json!(id(30000))),
        (
            "context",
            "scope",
            json!({"kind":"project","project_id":id(30000)}),
        ),
        ("expression", "source_utf8", json!("`\"blue\"`")),
        ("expression", "compiler_version", json!(2)),
        ("variable", "names", json!(["other"])),
        ("variable", "revision", json!(0)),
        ("variable", "portability", json!("host-only")),
        ("snapshot", "content_digest", json!(hash(b"wrong"))),
        ("snapshot", "owning_library_id", json!(id(30000))),
        ("page", "start_ordinal", json!(1)),
        ("page", "snapshot_id", json!(id(30000))),
        ("set", "member_count", json!(2)),
        ("set", "content_digest", json!(hash(b"wrong"))),
        ("managed", "retention", json!("host-only")),
        ("managed", "host_path", json!("/private/secret")),
        (
            "external",
            "coordinate",
            json!({"kind":"filesystem","components":[".."]}),
        ),
        ("content", "asset_id", json!(id(30000))),
        ("observation", "fingerprint", json!({"kind":"unverified"})),
        ("observation", "portability", json!("host-only")),
        ("groups", "input_digest", json!(hash(b"wrong"))),
        ("patch", "input_digest", json!(hash(b"wrong"))),
        (
            "graph",
            "node_contracts",
            json!([{"node_id":id(30000),"contract_digest":hash(b"wrong")}]),
        ),
    ];
    for (name, key, value) in cases {
        let files = build(|o| {
            o.get_mut(name).unwrap()[key] = value.clone();
        });
        assert!(validate(&files).is_err(), "accepted {name}.{key}");
    }
}
#[test]
fn selected_membership_and_preconditions_are_checked() {
    for n in 0..5 {
        let files = build(|o| match n {
            0 => {
                o.get_mut("page").unwrap()["members"][0]["representations"][0]["content_revision_id"] =
                    json!(id(30000));
            }
            1 => o.get_mut("groups").unwrap()["groups"][0]["member_asset_ids"] = json!([id(30000)]),
            2 => {
                o.get_mut("patch").unwrap()["operations"][0]["preconditions"][0]["content_revision_id"] =
                    json!(id(30000));
            }
            3 => o.get_mut("page").unwrap()["members"][0]["metadata_refs"] = json!([r("content")]),
            _ => o.get_mut("resources").unwrap()["managed_resources"] = json!([]),
        });
        assert!(validate(&files).is_err(), "case {n}");
    }
}
#[test]
fn extensions_do_not_create_edges() {
    let files = build(|o| {
        o.get_mut("authored").unwrap()["extensions"] =
            json!({"example.opaque":{"kind":"json","sha256":hash(b"missing"),"byte_length":"9"}});
    });
    validate(&files).unwrap();
}
#[test]
fn missing_managed_bytes_are_incomplete() {
    let mut files = build(|_| {});
    files.remove(&format!("objects/blobs/sha256/{}", hash(b"photo")));
    assert!(validate(&files).is_err());
}
#[test]
fn unknown_required_schema_fails_closed() {
    let files = build(|o| {
        o.get_mut("expression").unwrap()["schema"]["version"] = json!(2);
    });
    assert_eq!(
        validate(&files).unwrap_err(),
        PackageError::UnsupportedVersion
    );
}
#[test]
#[ignore = "Explicit golden generation only"]
fn generate_package_golden() {
    let files = build(add_history);
    validate(&files).unwrap();
    let rows:Vec<_>=files.iter().map(|(p,b)|json!({"path":p,"utf8":String::from_utf8(b.clone()).unwrap(),"sha256":hash(b)})).collect();
    let v = json!({"schema":{"id":"photara.fixture.d19-package","version":1},"codec":"photara.canonical-json.v1","files":rows});
    fs::write(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/fixtures/generation-two/d19-package-specimen.json"),
        canon(&v),
    )
    .unwrap();
}
#[expect(
    clippy::too_many_lines,
    reason = "Declarative immutable history specimen"
)]
fn add_history(o: &mut BTreeMap<String, Value>) {
    let p = id(20000);
    let l = id(20001);
    let run = id(20100);
    let attempt = id(20101);
    let operation = id(20102);
    let node = id(20103);
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../docs/fixtures/generation-two/d19-contracts.json"
    ))
    .unwrap();
    let manifest = manifest["manifest"].clone();
    let definition = manifest["definitions"][0].clone();
    let pin = definition["coordinate"].clone();
    let config =
        json!({"schema":{"id":"example.review.config","version":1},"value":{"label":"Synthetic"}});
    o.get_mut("graph").unwrap()["graph"]["nodes"] =
        json!([{"id":node,"definition":pin,"configuration":config}]);
    o.get_mut("graph").unwrap()["required_packages"] = json!([{"package_id":manifest["package_id"],"package_version":manifest["package_version"],"manifest":r("manifest")}]);
    o.get_mut("graph").unwrap()["node_contracts"] = json!([{"node_id":node,"manifest":r("manifest"),"contract_digest":hash(&canon(&definition))}]);
    o.get_mut("graph-context").unwrap()["node_contexts"] =
        json!([{"node_id":node,"context":r("node-context")}]);
    let graph_digest = hash(&canon(&o["graph"]["graph"]));
    let snapshot_digest = o["snapshot"]["content_digest"].clone();
    let mut add = |name: &str, k: &str, version: u32, v: Value| {
        o.insert(name.into(), record(k, version, v));
    };
    add(
        "manifest",
        "photara.node.manifest",
        1,
        json!({"manifest":manifest}),
    );
    add(
        "node-context",
        "photara.context.authored",
        1,
        json!({"scope":{"kind":"node","project_id":p,"graph_id":id(20018),"node_id":node},"variables":[],"expressions":[],"captures":[],"metadata_selections":[],"node_contexts":[]}),
    );
    add(
        "run-start",
        "photara.history.run-start",
        2,
        json!({"run_id":run,"request_id":id(20104),"source_graph_id":id(20018),"source_graph_revision":"0","source_graph_digest":graph_digest,"source_authored":r("authored"),"dependencies":[],"targets":[node],"implementations":[pin],"queued_at":TIME,"started_at":TIME,"owning_library_id":l,"context_snapshot":r("snapshot"),"input_snapshots":[{"node_id":node,"port_id":"assets","snapshot":r("set")}],"authorization_observation":{"mode":"local","actions":255},"device_dependency_digest":null,"replayability":"portable"}),
    );
    let typed =
        json!({"ty":cx::value::Type::string(),"value":{"kind":"string","value":"Synthetic"}});
    let mut implementation = pin.clone();
    implementation["fingerprint"] = json!(hash(b"implementation"));
    let definition_pin = json!({"package_id":"example.review","package_version":"1.2.3","definition_id":"example.review.inspect","definition_version":3,"implementation_digest":hash(b"implementation")});
    add(
        "intent",
        "photara.history.effect-intent",
        2,
        json!({"operation_id":operation,"run_id":run,"attempt_id":attempt,"captured_at":TIME,"definition":definition_pin,"operation_kind":"example.write","target":{"kind":"project-artifacts","project_id":p},"expected_target":null,"request_digest":hash(b"request"),"idempotency_key":"synthetic-1","input_digest":hash(b"inputs"),"context_digest":snapshot_digest,"retention":"managed-project","request":typed}),
    );
    add(
        "attempt-start",
        "photara.history.attempt-start",
        2,
        json!({"run_id":run,"attempt_id":attempt,"node_id":node,"ordinal":1,"started_at":TIME,"configuration_digest":hash(&canon(&config)),"input_digest":hash(b"inputs"),"environment_digest":hash(b"environment"),"implementation":implementation,"representation_revisions":[r("content")],"node_context_digest":snapshot_digest,"input_snapshot_refs":[r("set")],"execution_contract_digest":hash(&canon(&definition)),"operation_refs":[r("intent")],"replayability":"portable"}),
    );
    add(
        "evidence",
        "photara.history.evidence",
        1,
        json!({"evidence_id":id(20105),"operation_id":operation,"captured_at":TIME,"provenance":{},"content":{"schema":{"id":"example.evidence","version":1},"value":{}}}),
    );
    add(
        "receipt",
        "photara.history.receipt",
        2,
        json!({"receipt_id":id(20106),"operation_id":operation,"observing_attempt_id":attempt,"provider":"example.test","verification_at":TIME,"evidence":[r("evidence")],"observation":{"kind":"unknown"},"prior_receipt_ids":[],"request_digest":hash(b"request")}),
    );
    add(
        "artifact",
        "photara.history.artifact",
        1,
        json!({"artifact_id":id(20107),"operation_id":operation,"storage_class":"managed-project","descriptor":r("managed"),"retention":"managed-project","availability":"ready","durability":"managed-package-bytes","evidence":[r("evidence")]}),
    );
    add(
        "attempt-outcome",
        "photara.history.attempt-outcome",
        1,
        json!({"run_id":run,"attempt_id":attempt,"status":"succeeded","ended_at":TIME,"diagnostics":[],"evidence":[r("evidence")],"operations":[{"operation_id":operation,"knowledge":"unknown"}],"artifacts":[r("artifact")]}),
    );
    add(
        "run-outcome",
        "photara.history.run-outcome",
        1,
        json!({"run_id":run,"status":"succeeded","ended_at":TIME,"diagnostics":[],"evidence":[r("evidence")],"artifacts":[r("artifact")]}),
    );
    add(
        "run",
        "photara.history.run",
        1,
        json!({"run_id":run,"start":r("run-start"),"outcome":r("run-outcome"),"attempts":[{"attempt_id":attempt,"start":r("attempt-start"),"outcome":r("attempt-outcome")}],"operations":[r("intent")],"receipts":[r("receipt")],"evidence":[r("evidence")]}),
    );
    add(
        "proposal",
        "photara.context.change-proposal",
        1,
        json!({"proposal_id":id(20108),"operation_id":id(20109),"target":{"coordinate":{"kind":"variable","scope":{"kind":"project","project_id":p},"variable_id":id(20016)},"projection":[]},"expected_aggregate_revision":1,"expected_owner_revision":{"kind":"local","revision":1},"literal":typed,"sensitivity":"ordinary","portability":"portable","run_id":run,"attempt_id":attempt,"snapshot_id":id(20017),"snapshot":r("snapshot"),"snapshot_digest":snapshot_digest,"output_digest":hash(b"output")}),
    );
    add(
        "apply-receipt",
        "photara.context.apply-receipt",
        1,
        json!({"receipt_id":id(20110),"operation_id":id(20109),"request_digest":"AUTO","authority":{"kind":"project","project_id":p},"outcome":"unknown","resulting_revisions":[],"prior_receipts":[],"observed_at":TIME,"evidence":[]}),
    );
    o.get_mut("history").unwrap()["runs"] = json!([{"run_id":run,"document":r("run")}]);
    o.get_mut("history").unwrap()["operations"] = json!([r("intent")]);
    o.get_mut("history").unwrap()["evidence"] = json!([r("evidence")]);
    o.get_mut("history").unwrap()["artifacts"] = json!([r("artifact")]);
    o.get_mut("history").unwrap()["proposals"] = json!([r("proposal")]);
    o.get_mut("history").unwrap()["apply_receipts"] = json!([r("apply-receipt")]);
}
#[test]
fn history_and_node_contracts_validate() {
    validate(&build(add_history)).unwrap();
}
#[test]
fn history_contract_scope_rejections() {
    for (name, key, value) in [
        ("run-start", "owning_library_id", json!(id(30000))),
        ("attempt-start", "node_id", json!(id(30000))),
        ("receipt", "request_digest", json!(hash(b"wrong"))),
        ("proposal", "snapshot_id", json!(id(30000))),
        ("artifact", "descriptor", r("content")),
    ] {
        let files = build(|o| {
            add_history(o);
            o.get_mut(name).unwrap()[key] = value.clone();
        });
        assert!(validate(&files).is_err(), "{name}.{key}");
    }
}
fn rewrite_envelopes(
    files: &mut BTreeMap<String, Vec<u8>>,
    mut edit: impl FnMut(&mut Value, &mut Value),
) {
    let path = format!("commits/{}.json", id(20019));
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    let mut commit: Value = serde_json::from_slice(&files[&path]).unwrap();
    edit(&mut manifest, &mut commit);
    files.insert("manifest.json".into(), canon(&manifest));
    commit["bootstrap_sha256"] = json!(hash(&files["manifest.json"]));
    files.insert(path.clone(), canon(&commit));
    let mut head: Value = serde_json::from_slice(&files["HEAD.json"]).unwrap();
    head["commit_sha256"] = json!(hash(&files[&path]));
    files.insert("HEAD.json".into(), canon(&head));
}
#[test]
fn reader_floor_features_and_retained_bootstrap() {
    let mut upgrade = build(|_| {});
    rewrite_envelopes(&mut upgrade, |manifest, _| {
        manifest["format_version"]["minor"] = json!(0);
        manifest["required_features"] = json!(["photara.history.v1"]);
    });
    validate(&upgrade).unwrap();
    let root = materialize(&upgrade);
    assert!(package::validate_directory(root.path(), PackageLimits::default()).is_err());
    for feature in [
        "photara.library-project.v1",
        "photara.resources.v2",
        "photara.asset-set.v2",
        "photara.context.v1",
        "photara.node-contract.v2",
    ] {
        let mut files = build(add_history);
        rewrite_envelopes(&mut files, |m, c| {
            for v in [m, c] {
                v["required_features"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|v| v != feature);
            }
        });
        assert_eq!(
            validate(&files).unwrap_err(),
            PackageError::UnsupportedFeature
        );
    }
    let mut files = build(|_| {});
    rewrite_envelopes(&mut files, |_, c| c["minimum_reader"]["minor"] = json!(0));
    assert_eq!(
        validate(&files).unwrap_err(),
        PackageError::UnsupportedFeature
    );
    let mut files = build(|_| {});
    rewrite_envelopes(&mut files, |_, c| c["minimum_reader"]["minor"] = json!(2));
    assert_eq!(
        validate(&files).unwrap_err(),
        PackageError::UnsupportedVersion
    );
}
#[test]
fn library_capture_projection() {
    let files = build(|o| {
        o.insert("library-snapshot".into(),record("photara.project.library-snapshot",2,json!({"snapshot_id":id(20200),"captured_at":TIME,"display_name":"Person","source":{"library_id":id(20001),"record_id":id(20201),"kind":"person","revision":{"kind":"local","revision":1}},"projection_schema":{"id":"example.person","version":1},"fields":["display_name"],"payload":{"display_name":"Person"},"sensitivity":"personal","portability":"portable"})));
        o.insert("party".into(),record("photara.project.party-assignment",1,json!({"assignment_id":id(20202),"revision":"1","party_snapshot":r("library-snapshot"),"roles":["subject"],"notes":"","created_at":TIME,"updated_at":TIME})));
        o.get_mut("parties").unwrap()["assignments"] = json!([r("party")]);
    });
    validate(&files).unwrap();
}
#[test]
fn receipt_and_manifest_mismatches_are_rejected() {
    for case in 0..3 {
        let files = build(|o| {
            add_history(o);
            match case {
                0 => o.get_mut("apply-receipt").unwrap()["request_digest"] = json!(hash(b"wrong")),
                1 => o.get_mut("graph").unwrap()["node_contracts"][0]["manifest"] = Value::Null,
                _ => {
                    o.get_mut("graph").unwrap()["node_contracts"][0]["contract_digest"] =
                        json!(hash(b"wrong"));
                }
            }
        });
        assert!(validate(&files).is_err());
    }
}
#[test]
fn limits_and_page_boundaries() {
    let files = build(|o| {
        let m = o["page"]["members"][0].clone();
        o.get_mut("page").unwrap()["members"] = json!(vec![m; 501]);
    });
    assert_eq!(validate(&files).unwrap_err(), PackageError::Limit);
    let root = materialize(&build(|_| {}));
    let limits = PackageLimits {
        max_objects: 5,
        ..PackageLimits::default()
    };
    assert_eq!(
        package::v1_1::validate_directory(root.path(), limits).unwrap_err(),
        PackageError::Limit
    );
}
#[test]
fn package_golden_is_exact_rust_canonical_bytes() {
    let expected =
        include_bytes!("../../../docs/fixtures/generation-two/d19-package-specimen.json");
    let archive: Value = serde_json::from_slice(expected).unwrap();
    assert_eq!(canon(&archive), expected);
    let files = build(add_history);
    let rows:Vec<_>=files.iter().map(|(p,b)|json!({"path":p,"utf8":String::from_utf8(b.clone()).unwrap(),"sha256":hash(b)})).collect();
    assert_eq!(archive["files"], json!(rows));
    validate(&files).unwrap();
}
