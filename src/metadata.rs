use serde::{Deserialize, Serialize};

use crate::{PhotaraError, Result, config::PhotaraConfig, project::ProjectRecord};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReconciliationPlan {
    pub schema_version: u32,
    pub project: ProjectRef,
    pub project_keyword: KeywordPath,
    pub managed_iptc: ManagedIptc,
    pub people_keywords: Vec<KeywordPath>,
    pub managed_keyword_catalog: Vec<KeywordPath>,
    pub collection_trees: Vec<CollectionTree>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProjectRef {
    pub id: String,
    pub slug: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ManagedIptc {
    pub job_identifier: String,
    pub scene: String,
    pub sublocation: String,
    pub city: String,
    pub state_province: String,
    pub country_region: String,
    pub iso_country_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct KeywordPath {
    pub path: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CollectionTree {
    pub path: Vec<String>,
    pub smart_collections: Vec<SmartCollection>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SmartCollection {
    pub collection_set_path: Vec<String>,
    pub name: String,
    pub rules: Vec<CollectionRule>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CollectionRule {
    pub field: RuleField,
    pub operator: RuleOperator,
    pub value: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleField {
    JobIdentifier,
    FileType,
    Keyword,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleOperator {
    Equals,
    Contains,
}

pub fn plan(config: &PhotaraConfig, project: &ProjectRecord) -> Result<ReconciliationPlan> {
    let scene = config.scenes.get(&project.scene).ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "project scene {:?} is missing from scenes.yml",
            project.scene
        ))
    })?;
    let location = config.locations.get(&project.location).ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "project location {:?} is missing from locations.yml",
            project.location
        ))
    })?;

    let mut people = Vec::new();
    let mut people_keywords = Vec::new();
    for slug in &project.people {
        let person = config.people.get(slug).ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "project person {slug:?} is missing from people.yml"
            ))
        })?;
        people.push(person.display_name.clone());
        for role in &person.roles {
            people_keywords.push(KeywordPath {
                path: vec!["people".into(), role.clone(), person.display_name.clone()],
            });
        }
    }
    people.sort();
    people_keywords.sort_by(|left, right| left.path.cmp(&right.path));

    let mut collection_paths = people
        .into_iter()
        .map(|display_name| vec!["People".into(), display_name, project.display_name.clone()])
        .collect::<Vec<_>>();
    collection_paths.extend([
        vec![
            "Locations".into(),
            location.display_name.clone(),
            project.display_name.clone(),
        ],
        vec![
            "Scenes".into(),
            scene.display_name.clone(),
            project.display_name.clone(),
        ],
        vec!["Projects".into(), project.display_name.clone()],
    ]);

    Ok(ReconciliationPlan {
        schema_version: 3,
        project: ProjectRef {
            id: project.id.to_string(),
            slug: project.slug.clone(),
            display_name: project.display_name.clone(),
        },
        project_keyword: KeywordPath {
            path: vec!["projects".into(), project.display_name.clone()],
        },
        managed_iptc: ManagedIptc {
            job_identifier: project.display_name.clone(),
            scene: scene.display_name.clone(),
            sublocation: location.sublocation.clone(),
            city: location.city.clone(),
            state_province: location.state.clone(),
            country_region: location.country.clone(),
            iso_country_code: location.iso_country_code.clone(),
            creator: config.settings.default_creator.clone(),
            copyright: config.settings.default_copyright.clone(),
        },
        people_keywords,
        managed_keyword_catalog: managed_keyword_catalog(),
        collection_trees: collection_paths
            .into_iter()
            .map(|path| CollectionTree {
                path,
                smart_collections: smart_collections(&project.display_name),
            })
            .collect(),
    })
}

fn managed_keyword_catalog() -> Vec<KeywordPath> {
    [
        &["workflow", "selection", "client-favorite"][..],
        &["workflow", "selection", "client-shortlist"],
        &["workflow", "selection", "hero"],
        &["workflow", "selection", "photographer-final"],
        &["workflow", "cloud", "present"],
        &["asset_type", "master", "psb"],
    ]
    .into_iter()
    .map(|path| KeywordPath {
        path: path.iter().map(|part| (*part).to_owned()).collect(),
    })
    .collect()
}

fn smart_collections(project: &str) -> Vec<SmartCollection> {
    let project_rule = CollectionRule {
        field: RuleField::Keyword,
        operator: RuleOperator::Contains,
        value: project.into(),
    };
    [
        (&[][..], "All", None),
        (
            &["Originals"][..],
            "RAW",
            Some((RuleField::FileType, "raw")),
        ),
        (
            &["Selections"][..],
            "Client Favorites",
            Some((RuleField::Keyword, "client-favorite")),
        ),
        (
            &["Selections"][..],
            "Client Shortlist",
            Some((RuleField::Keyword, "client-shortlist")),
        ),
        (
            &["Selections"][..],
            "Hero",
            Some((RuleField::Keyword, "hero")),
        ),
        (
            &["Selections"][..],
            "Photographer Final",
            Some((RuleField::Keyword, "photographer-final")),
        ),
        (
            &["Cloud"][..],
            "Lightroom",
            Some((RuleField::Keyword, "present")),
        ),
        (&["Masters"][..], "PSB", Some((RuleField::FileType, "psb"))),
    ]
    .into_iter()
    .map(|(collection_set_path, name, extra)| {
        let mut rules = vec![project_rule.clone()];
        if let Some((field, value)) = extra {
            rules.push(CollectionRule {
                field,
                operator: if field == RuleField::FileType {
                    RuleOperator::Equals
                } else {
                    RuleOperator::Contains
                },
                value: value.into(),
            });
        }
        SmartCollection {
            collection_set_path: collection_set_path
                .iter()
                .map(|part| (*part).to_owned())
                .collect(),
            name: name.into(),
            rules,
        }
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Location, Person, Scene, Settings};
    use std::{collections::BTreeMap, path::PathBuf};
    use uuid::Uuid;

    #[test]
    fn red_meridian_plan_is_deterministic_and_uses_trinity() {
        let config = PhotaraConfig {
            root: PathBuf::from("/config"),
            settings: Settings {
                images_root: PathBuf::from("/images"),
                projects_root: PathBuf::from("/projects"),
                lightroom_inbox: PathBuf::from("/Pictures/Photara/Inbox"),
                default_catalog: "Lr_Photara".into(),
                default_creator: Some("Suhail".into()),
                default_author_code: "SUHAIL".into(),
                default_copyright: Some("@suhail".into()),
                default_country: "United States".into(),
                default_iso_country_code: "US".into(),
                proof_provider: "pixieset".into(),
                delivery_provider: "cloudinary".into(),
            },
            people: BTreeMap::from([(
                "trinity-woodward".into(),
                Person {
                    display_name: "Trinity Woodward".into(),
                    aliases: vec!["Trin".into()],
                    roles: vec!["model".into()],
                    social: BTreeMap::new(),
                },
            )]),
            locations: BTreeMap::from([(
                "golden-gate-bridge".into(),
                Location {
                    display_name: "Golden Gate Bridge".into(),
                    sublocation: "Golden Gate Bridge".into(),
                    city: "San Francisco".into(),
                    state: "California".into(),
                    country: "United States".into(),
                    iso_country_code: "US".into(),
                },
            )]),
            scenes: BTreeMap::from([(
                "architectural-portrait".into(),
                Scene {
                    display_name: "Architectural Portrait".into(),
                    description: None,
                },
            )]),
        };
        let project = ProjectRecord {
            id: Uuid::nil(),
            slug: "red-meridian".into(),
            display_name: "Red Meridian".into(),
            scene: "architectural-portrait".into(),
            location: "golden-gate-bridge".into(),
            people: vec!["trinity-woodward".into()],
            origin: "native".into(),
            status: "active".into(),
        };

        let plan = plan(&config, &project).unwrap();
        assert_eq!(
            plan.project_keyword,
            KeywordPath {
                path: vec!["projects".into(), "Red Meridian".into()]
            }
        );
        assert_eq!(
            plan.people_keywords,
            vec![KeywordPath {
                path: vec!["people".into(), "model".into(), "Trinity Woodward".into()]
            }]
        );
        assert_eq!(plan.collection_trees.len(), 4);
        assert!(
            plan.collection_trees
                .iter()
                .all(|tree| tree.smart_collections.len() == 8)
        );
        let collections = &plan.collection_trees[0].smart_collections;
        assert_eq!(collections[0].name, "All");
        assert!(collections[0].collection_set_path.is_empty());
        assert_eq!(collections[1].name, "RAW");
        assert_eq!(collections[1].collection_set_path, ["Originals"]);
        assert_eq!(collections[2].collection_set_path, ["Selections"]);
        assert_eq!(collections[4].name, "Hero");
        assert_eq!(collections[4].collection_set_path, ["Selections"]);
        assert_eq!(collections[6].collection_set_path, ["Cloud"]);
        assert_eq!(collections[7].collection_set_path, ["Masters"]);
        assert_eq!(
            collections[7].rules[1],
            CollectionRule {
                field: RuleField::FileType,
                operator: RuleOperator::Equals,
                value: "psb".into(),
            }
        );
        assert!(collections.iter().all(|collection| {
            collection.rules[0]
                == CollectionRule {
                    field: RuleField::Keyword,
                    operator: RuleOperator::Contains,
                    value: "Red Meridian".into(),
                }
        }));
        for collection in collections {
            for rule in &collection.rules {
                if rule.field == RuleField::Keyword {
                    assert!(
                        !rule.value.contains('|'),
                        "Lightroom smart collections require keyword leaf labels"
                    );
                }
            }
        }
    }
}
