//! Typed overlay edge regressions, invoked by the mandatory session build gate.
use super::*;
use crate::package::v1_3::{Arena, PhysicalRef};
fn reference(v: &Value) -> Value {
    let bytes = photara_core::canonical_json(v).unwrap();
    json!({"kind":"json","sha256":crate::package::digest(&bytes).as_str(),"byte_length":bytes.len().to_string()})
}
struct Graph {
    objects: BTreeMap<Vec<u8>, Value>,
    physical: bool,
    next: u64,
}
impl Graph {
    fn add(&mut self, body: Value) -> Value {
        let r = if self.physical {
            self.next += 1;
            json!({"allocation_id":"10000000-0000-4000-8000-000000000004","arena":"metadata","offset":(self.next*4096).to_string(),"byte_length":"128","record_sha256":reference(&body)["sha256"]})
        } else {
            reference(&body)
        };
        self.objects
            .insert(photara_core::canonical_json(&r).unwrap(), body);
        r
    }
}
fn fixture(physical: bool, bad: u8) -> (Graph, Value) {
    let mut g = Graph {
        objects: BTreeMap::new(),
        physical,
        next: 0,
    };
    let project = "10000000-0000-4000-8000-000000000001";
    let prefix = if physical {
        "photara.storage.locator-overlay"
    } else {
        "photara.package.inventory-overlay"
    };
    let value = |name: &str, mut v: Value| {
        v["schema"] = json!({"id":name,"version": if name == "photara.package.inventory-overlay" { 2 } else { 1 }});
        v["project_id"] = json!(project);
        v["extensions"] = json!({});
        v
    };
    let base = g.add(value(
        if physical {
            "photara.storage.locator-leaf"
        } else {
            "photara.package.inventory-leaf"
        },
        json!({"count":"0","entries":[]}),
    ));
    let object = |c: char| json!({"kind":if bad == 11 && c == 'a' { "blob" } else { "json" },"sha256":c.to_string().repeat(64),"byte_length":"1"});
    let put = |c: char| {
        if physical {
            json!({"object":object(c),"change":{"kind":"put","membership":"semantic","physical":{"allocation_id":"10000000-0000-4000-8000-000000000003","arena":if bad==8{"metadata"}else{"data"},"offset":"0","byte_length":"128","record_sha256":"c".repeat(64)}}})
        } else {
            json!({"object":object(c),"change":"insert"})
        }
    };
    let put = |c: char| {
        let mut entry = put(c);
        if physical && bad == 11 && c == 'a' {
            entry["change"]["membership"] = json!("blob");
            entry["change"]["physical"] = json!({"kind":"whole-blob","allocation_id":"10000000-0000-4000-8000-000000000003","byte_length":"1","sha256":"a".repeat(64)});
        }
        entry
    };
    let remove = |c: char| json!({"object":object(c),"change":if physical {json!({"kind":"remove"})}else{json!("remove")}});
    let mut first = value(
        &format!("{prefix}-batch"),
        json!({"sequence":"1","before_count":"0","after_count":"1","entries":[put('a')]}),
    );
    match bad {
        1 => first["before_count"] = json!("1"),
        2 => first["after_count"] = json!("2"),
        3 => first["entries"] = json!([put('a'), put('a')]),
        5 => first["extensions"] = json!({"unknown":true}),
        _ => {}
    }
    let first = g.add(first);
    let second=g.add(value(&format!("{prefix}-batch"),json!({"sequence":"2","before_count":"1","after_count":"1","entries":if bad==6 {vec![remove('b'),put('b')]}else if bad==7 {vec![put('a')]}else{vec![remove('a'),put('b')]}})));
    let one = g.add(value(
        &format!("{prefix}-index-leaf"),
        json!({"count":"1","entries":[{"sequence":"1","batch":first}]}),
    ));
    let two = g.add(value(
        &format!("{prefix}-index-leaf"),
        json!({"count":"1","entries":[{"sequence":"2","batch":second}]}),
    ));
    let mut children = vec![
        json!({"first":"1","last":"1","count":"1","child":one}),
        json!({"first":"2","last":"2","count":"1","child":two}),
    ];
    if bad == 4 {
        children.reverse();
    }
    let index = g.add(value(
        &format!("{prefix}-index-branch"),
        json!({"count":"2","children":children}),
    ));
    let mut root =
        json!({"base":base,"base_count":"0","batches":index,"through_sequence":"2","count":"1"});
    if physical {
        root["role"] = json!("active");
        root["allocation_id"] = json!("10000000-0000-4000-8000-000000000004");
    } else {
        root["purpose"] = json!("state");
    }
    let mut root = value(prefix, root);
    if bad == 9 {
        root["schema"]["version"] = json!(if physical { 2 } else { 1 });
    }
    if bad == 10 {
        root.as_object_mut().unwrap().remove("purpose");
        root["controls"] = json!([]);
    }
    let root = g.add(root);
    (g, root)
}
#[expect(
    clippy::too_many_lines,
    reason = "Parallel logical and physical malformed histories use the same exact fixture"
)]
pub(crate) fn audit_regressions() {
    let project = PackageUuid::parse("10000000-0000-4000-8000-000000000001").unwrap();
    let limits = TreeLimits {
        max_depth: 8,
        max_pages: 32,
        max_entries: 32,
        max_leaf_entries: 2,
        max_branch_children: 2,
    };
    for physical in [false, true] {
        let family = if physical {
            Family::Locator {
                role: Some("active"),
                allocation: "10000000-0000-4000-8000-000000000004",
            }
        } else {
            Family::Inventory {
                purpose: Some("state"),
            }
        };
        let (g, root) = fixture(physical, 0);
        let load = |r: &Value| {
            g.objects
                .get(&photara_core::canonical_json(r).unwrap())
                .cloned()
                .ok_or(PackageError::Integrity)
        };
        assert_eq!(
            resolve(load, &root, project, family, limits)
                .unwrap()
                .entries
                .len(),
            1
        );
        // Scoped reads authenticate the index plus newest matching batch,
        // without repeatedly materializing the historical anchor. Full audit
        // remains mandatory and independently checks that skipped history.
        let reads = std::cell::Cell::new(0usize);
        let counted = |r: &Value| {
            reads.set(reads.get() + 1);
            load(r)
        };
        let all = resolve(counted, &root, project, family, limits).unwrap();
        let full_reads = reads.replace(0);
        let key = family.object(&all.entries[0]).unwrap();
        assert_eq!(
            lookup(counted, &root, project, family, limits, &key).unwrap(),
            Some(all.entries[0].clone())
        );
        assert!(reads.replace(0) < full_reads);
        let removed = serde_json::from_value(
            json!({"kind":"json","sha256":"a".repeat(64),"byte_length":"1"}),
        )
        .unwrap();
        assert_eq!(
            lookup(counted, &root, project, family, limits, &removed).unwrap(),
            None
        );
        let absent = serde_json::from_value(
            json!({"kind":"json","sha256":"f".repeat(64),"byte_length":"1"}),
        )
        .unwrap();
        assert_eq!(
            lookup(counted, &root, project, family, limits, &absent).unwrap(),
            None
        );
        let anchor = g.objects[&photara_core::canonical_json(&root).unwrap()]["base"].clone();
        let missing_anchor = |r: &Value| {
            if *r == anchor {
                Err(PackageError::Integrity)
            } else {
                load(r)
            }
        };
        assert!(lookup(missing_anchor, &root, project, family, limits, &key).is_ok());
        assert!(lookup(missing_anchor, &root, project, family, limits, &absent).is_err());
        assert!(resolve(missing_anchor, &root, project, family, limits).is_err());
        assert!(
            lookup(
                load,
                &root,
                project,
                family,
                TreeLimits {
                    max_depth: 3,
                    ..limits
                },
                &key
            )
            .is_err()
        );
        for bound in [
            TreeLimits {
                max_depth: 3,
                ..limits
            },
            TreeLimits {
                max_pages: 3,
                ..limits
            },
            TreeLimits {
                max_entries: 3,
                ..limits
            },
            TreeLimits {
                max_branch_children: 1,
                ..limits
            },
        ] {
            assert!(resolve(load, &root, project, family, bound).is_err());
        }
        let wrong = if physical {
            Family::Locator {
                role: Some("recovery"),
                allocation: "10000000-0000-4000-8000-000000000004",
            }
        } else {
            Family::Inventory {
                purpose: Some("global"),
            }
        };
        assert!(resolve(load, &root, project, wrong, limits).is_err());
        if physical {
            assert!(
                resolve(
                    load,
                    &root,
                    project,
                    Family::Locator {
                        role: Some("active"),
                        allocation: "10000000-0000-4000-8000-000000000005"
                    },
                    limits
                )
                .is_err()
            );
            let (mixed, mixed_root) = fixture(true, 11);
            assert_eq!(
                resolve(
                    |r| mixed
                        .objects
                        .get(&photara_core::canonical_json(r).unwrap())
                        .cloned()
                        .ok_or(PackageError::Integrity),
                    &mixed_root,
                    project,
                    family,
                    limits
                )
                .unwrap()
                .entries
                .len(),
                1
            );
            let parsed: PhysicalRef = serde_json::from_value(root.clone()).unwrap();
            assert_eq!(parsed.arena, Arena::Metadata);
        }
        for bad in (1..=7).chain(if physical { vec![8, 9] } else { vec![9, 10] }) {
            let (g, root) = fixture(physical, bad);
            assert!(
                resolve(
                    |r| g
                        .objects
                        .get(&photara_core::canonical_json(r).unwrap())
                        .cloned()
                        .ok_or(PackageError::Integrity),
                    &root,
                    project,
                    family,
                    limits
                )
                .is_err(),
                "physical={physical} bad={bad}"
            );
        }
        if !physical {
            let root_body = g.objects[&photara_core::canonical_json(&root).unwrap()].clone();
            // The selected control keeps its frozen v1 schema. New tree roots cannot cross-use it.
            assert!(
                super::super::selected::schema(
                    &root_body,
                    "photara.package.inventory-overlay",
                    1,
                    &["base", "controls", "count"],
                    project
                )
                .is_err()
            );
            let mut legacy = Graph {
                objects: g.objects.clone(),
                physical: false,
                next: 0,
            };
            let control = json!({"schema":{"id":"photara.package.inventory-overlay","version":1},"project_id":project.to_string(),"base":root_body["base"],"controls":[],"count":"0","extensions":{}});
            super::super::selected::schema(
                &control,
                "photara.package.inventory-overlay",
                1,
                &["base", "controls", "count"],
                project,
            )
            .unwrap();
            let control = legacy.add(control);
            assert!(
                resolve(
                    |r| legacy
                        .objects
                        .get(&photara_core::canonical_json(r).unwrap())
                        .cloned()
                        .ok_or(PackageError::Integrity),
                    &control,
                    project,
                    family,
                    limits
                )
                .is_err()
            );
        }
        let mut g = g;
        let key = g
            .objects
            .keys()
            .find(|k| **k != photara_core::canonical_json(&root).unwrap())
            .unwrap()
            .clone();
        g.objects.remove(&key);
        assert!(
            resolve(
                |r| g
                    .objects
                    .get(&photara_core::canonical_json(r).unwrap())
                    .cloned()
                    .ok_or(PackageError::Integrity),
                &root,
                project,
                family,
                limits
            )
            .is_err()
        );
    }
}
