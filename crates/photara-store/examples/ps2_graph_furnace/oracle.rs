// Disposable PS1 specimen generator copied from package_v1_1 at 853781a.
use photara_core::{canonical_json, context as cx};
#[cfg(test)]
use photara_store::package;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::BTreeSet;
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
pub(super) fn add_node(o: &mut BTreeMap<String, Value>) {
    let raw: Value = serde_json::from_str(include_str!(
        "../../../../docs/fixtures/generation-two/d19-contracts.json"
    ))
    .unwrap();
    let manifest = raw["manifest"].clone();
    let definition = manifest["definitions"][0].clone();
    let node = id(20103);
    o.get_mut("graph").unwrap()["graph"]["nodes"] = json!([{"id":node,"definition":definition["coordinate"],"configuration":{"schema":{"id":"example.review.config","version":1},"value":{"label":"Synthetic"}},"photara.graph-position":{"x":0,"y":0}}]);
    o.get_mut("graph").unwrap()["required_packages"] = json!([{"package_id":manifest["package_id"],"package_version":manifest["package_version"],"manifest":r("manifest")}]);
    o.get_mut("graph").unwrap()["node_contracts"] = json!([{"node_id":node,"manifest":r("manifest"),"contract_digest":hash(&canon(&definition))}]);
    o.insert(
        "manifest".into(),
        record("photara.node.manifest", 1, json!({"manifest":manifest})),
    );
    o.get_mut("graph-context").unwrap()["node_contexts"] =
        json!([{"node_id":node,"context":r("node-context")}]);
    o.insert("node-context".into(),record("photara.context.authored",1,json!({"scope":{"kind":"node","project_id":id(20000),"graph_id":id(20018),"node_id":node},"variables":[],"expressions":[],"captures":[],"metadata_selections":[],"node_contexts":[]})));
}
pub(super) fn graph() -> photara_core::GraphDocument {
    let mut objects = fixture_objects();
    add_node(&mut objects);
    decode(objects["graph"]["graph"].clone())
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
#[cfg(test)]
struct Builder {
    objects: BTreeMap<String, Value>,
    resolved: BTreeMap<String, Value>,
    files: BTreeMap<String, Vec<u8>>,
    refs: BTreeSet<package::ObjectRef>,
}
#[cfg(test)]
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
#[cfg(test)]
pub(super) fn build(
    mut edit: impl FnMut(&mut BTreeMap<String, Value>),
) -> BTreeMap<String, Vec<u8>> {
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
