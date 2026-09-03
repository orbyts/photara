use std::{collections::BTreeSet, env, fs, path::PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};
use storexa::Database;
use uuid::Uuid;

use crate::{PhotaraError, Result, config::PhotaraConfig, project};

const LIGHTROOM_FILES: &[(&str, &[u8])] = &[
    (
        "AddPhotographerFinalMain.lua",
        include_bytes!("../lightroom/photara.lrplugin/AddPhotographerFinalMain.lua"),
    ),
    (
        "ApplyCloudPresenceMain.lua",
        include_bytes!("../lightroom/photara.lrplugin/ApplyCloudPresenceMain.lua"),
    ),
    (
        "ApplyCloudWithdrawalMain.lua",
        include_bytes!("../lightroom/photara.lrplugin/ApplyCloudWithdrawalMain.lua"),
    ),
    (
        "ApplyProjectMain.lua",
        include_bytes!("../lightroom/photara.lrplugin/ApplyProjectMain.lua"),
    ),
    (
        "ApplySelectionsMain.lua",
        include_bytes!("../lightroom/photara.lrplugin/ApplySelectionsMain.lua"),
    ),
    (
        "Config.lua",
        include_bytes!("../lightroom/photara.lrplugin/Config.lua"),
    ),
    (
        "ImportMastersMain.lua",
        include_bytes!("../lightroom/photara.lrplugin/ImportMastersMain.lua"),
    ),
    (
        "Info.lua",
        include_bytes!("../lightroom/photara.lrplugin/Info.lua"),
    ),
    (
        "MetadataProvider.lua",
        include_bytes!("../lightroom/photara.lrplugin/MetadataProvider.lua"),
    ),
    (
        "Photara.lua",
        include_bytes!("../lightroom/photara.lrplugin/Photara.lua"),
    ),
    (
        "PlanTransferMain.lua",
        include_bytes!("../lightroom/photara.lrplugin/PlanTransferMain.lua"),
    ),
    (
        "PrepareEditComparisonMain.lua",
        include_bytes!("../lightroom/photara.lrplugin/PrepareEditComparisonMain.lua"),
    ),
    (
        "ReconcileMastersMain.lua",
        include_bytes!("../lightroom/photara.lrplugin/ReconcileMastersMain.lua"),
    ),
    (
        "RemovePhotographerFinalMain.lua",
        include_bytes!("../lightroom/photara.lrplugin/RemovePhotographerFinalMain.lua"),
    ),
    (
        "ValidateMain.lua",
        include_bytes!("../lightroom/photara.lrplugin/ValidateMain.lua"),
    ),
];

#[derive(Debug, Serialize)]
pub struct LightroomPluginReport {
    pub schema_version: u32,
    pub version: &'static str,
    pub destination: PathBuf,
    pub package_sha256: String,
    pub installed: bool,
    pub matches_release: bool,
    pub reused_existing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup: Option<PathBuf>,
}

pub fn lightroom_status() -> Result<LightroomPluginReport> {
    let destination = lightroom_destination()?;
    let installed = destination.is_dir();
    let matches_release = installed && installed_matches(&destination)?;
    Ok(LightroomPluginReport {
        schema_version: 1,
        version: env!("CARGO_PKG_VERSION"),
        destination,
        package_sha256: lightroom_package_sha256(),
        installed,
        matches_release,
        reused_existing: false,
        backup: None,
    })
}

pub fn install_lightroom_plugin() -> Result<LightroomPluginReport> {
    let status = lightroom_status()?;
    if status.matches_release {
        return Ok(LightroomPluginReport {
            reused_existing: true,
            ..status
        });
    }
    let destination = status.destination;
    let parent = destination.parent().ok_or_else(|| {
        PhotaraError::Configuration("Lightroom Modules destination has no parent".into())
    })?;
    fs::create_dir_all(parent).map_err(|source| {
        PhotaraError::filesystem("create Lightroom Modules directory", parent, source)
    })?;
    let staging = parent.join(format!(".photara.lrplugin.{}.tmp", Uuid::new_v4()));
    fs::create_dir(&staging).map_err(|source| {
        PhotaraError::filesystem("create staged Lightroom plugin", &staging, source)
    })?;
    for (name, contents) in LIGHTROOM_FILES {
        let path = staging.join(name);
        fs::write(&path, contents).map_err(|source| {
            PhotaraError::filesystem("write Lightroom plugin file", path, source)
        })?;
    }
    let backup = if destination.symlink_metadata().is_ok() {
        let backup = parent.join(format!(
            "photara.lrplugin.backup-{}-{}",
            env!("CARGO_PKG_VERSION"),
            Uuid::new_v4()
        ));
        fs::rename(&destination, &backup).map_err(|source| {
            PhotaraError::filesystem("back up installed Lightroom plugin", &destination, source)
        })?;
        Some(backup)
    } else {
        None
    };
    if let Err(source) = fs::rename(&staging, &destination) {
        if let Some(backup) = &backup {
            let _ = fs::rename(backup, &destination);
        }
        return Err(PhotaraError::filesystem(
            "install staged Lightroom plugin",
            staging,
            source,
        ));
    }
    Ok(LightroomPluginReport {
        schema_version: 1,
        version: env!("CARGO_PKG_VERSION"),
        destination,
        package_sha256: lightroom_package_sha256(),
        installed: true,
        matches_release: true,
        reused_existing: false,
        backup,
    })
}

pub fn uninstall_lightroom_plugin() -> Result<LightroomPluginReport> {
    let status = lightroom_status()?;
    if !status.installed {
        return Ok(LightroomPluginReport {
            reused_existing: true,
            ..status
        });
    }
    let destination = status.destination;
    let parent = destination.parent().ok_or_else(|| {
        PhotaraError::Configuration("Lightroom Modules destination has no parent".into())
    })?;
    let backup = parent.join(format!(
        "photara.lrplugin.uninstalled-{}-{}",
        env!("CARGO_PKG_VERSION"),
        Uuid::new_v4()
    ));
    fs::rename(&destination, &backup).map_err(|source| {
        PhotaraError::filesystem("uninstall Lightroom plugin", &destination, source)
    })?;
    Ok(LightroomPluginReport {
        schema_version: 1,
        version: env!("CARGO_PKG_VERSION"),
        destination,
        package_sha256: lightroom_package_sha256(),
        installed: false,
        matches_release: false,
        reused_existing: false,
        backup: Some(backup),
    })
}

fn lightroom_destination() -> Result<PathBuf> {
    let home = env::var_os("HOME").ok_or_else(|| {
        PhotaraError::Configuration("HOME is required to locate Lightroom Classic Modules".into())
    })?;
    Ok(PathBuf::from(home)
        .join("Library/Application Support/Adobe/Lightroom/Modules/photara.lrplugin"))
}

fn installed_matches(destination: &std::path::Path) -> Result<bool> {
    let expected: BTreeSet<_> = LIGHTROOM_FILES
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    let actual = fs::read_dir(destination)
        .map_err(|source| {
            PhotaraError::filesystem("read installed Lightroom plugin", destination, source)
        })?
        .map(|entry| {
            entry
                .map_err(|source| {
                    PhotaraError::filesystem(
                        "read installed Lightroom plugin entry",
                        destination,
                        source,
                    )
                })
                .and_then(|entry| {
                    entry.file_name().into_string().map_err(|_| {
                        PhotaraError::Configuration(
                            "installed Lightroom plugin contains a non-UTF-8 filename".into(),
                        )
                    })
                })
        })
        .collect::<Result<BTreeSet<_>>>()?;
    if actual != expected {
        return Ok(false);
    }
    for (name, expected_contents) in LIGHTROOM_FILES {
        let path = destination.join(name);
        let actual_contents = fs::read(&path).map_err(|source| {
            PhotaraError::filesystem("read installed Lightroom plugin file", path, source)
        })?;
        if actual_contents != *expected_contents {
            return Ok(false);
        }
    }
    Ok(true)
}

fn lightroom_package_sha256() -> String {
    let mut digest = Sha256::new();
    for (name, contents) in LIGHTROOM_FILES {
        digest.update(name.as_bytes());
        digest.update([0]);
        digest.update(contents);
    }
    format!("{:x}", digest.finalize())
}

#[derive(Debug, Serialize)]
pub struct PluginContext<'a> {
    pub schema_version: u32,
    pub people: &'a std::collections::BTreeMap<String, crate::config::Person>,
    pub locations: &'a std::collections::BTreeMap<String, crate::config::Location>,
    pub scenes: &'a std::collections::BTreeMap<String, crate::config::Scene>,
    pub projects: Vec<project::ProjectRecord>,
}

pub async fn context<'a>(
    database: &Database,
    config: &'a PhotaraConfig,
) -> Result<PluginContext<'a>> {
    Ok(PluginContext {
        schema_version: 1,
        people: &config.people,
        locations: &config.locations,
        scenes: &config.scenes,
        projects: project::list(database).await?,
    })
}

pub fn to_lua<T: Serialize>(value: &T) -> Result<String> {
    let value = serde_json::to_value(value)?;
    let mut output = String::from("return ");
    write_lua(&value, &mut output);
    output.push('\n');
    Ok(output)
}

fn write_lua(value: &serde_json::Value, output: &mut String) {
    match value {
        serde_json::Value::Null => output.push_str("nil"),
        serde_json::Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        serde_json::Value::Number(value) => output.push_str(&value.to_string()),
        serde_json::Value::String(value) => write_lua_string(value, output),
        serde_json::Value::Array(values) => {
            output.push('{');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_lua(value, output);
            }
            output.push('}');
        }
        serde_json::Value::Object(values) => {
            output.push('{');
            for (index, (key, value)) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push('[');
                write_lua_string(key, output);
                output.push_str("]=");
                write_lua(value, output);
            }
            output.push('}');
        }
    }
}

fn write_lua_string(value: &str, output: &mut String) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_ascii_control() => {
                output.push_str(&format!("\\{:03}", character as u32));
            }
            _ => output.push(character),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_loadable_deterministic_lua_table_source() {
        let value = serde_json::json!({
            "name": "Trinity \"Trin\" Woodward — modèle",
            "roles": ["model"],
            "active": true,
            "missing": null
        });
        let output = to_lua(&value).unwrap();
        assert!(output.starts_with("return {"));
        assert!(output.contains("[\"active\"]=true"));
        assert!(output.contains("Trinity \\\"Trin\\\" Woodward"));
        assert!(output.contains("— modèle"));
        assert!(output.contains("[\"missing\"]=nil"));
    }

    #[test]
    fn bundled_lightroom_plugin_is_complete_and_deterministic() {
        assert_eq!(LIGHTROOM_FILES.len(), 15);
        let names: BTreeSet<_> = LIGHTROOM_FILES.iter().map(|(name, _)| *name).collect();
        assert_eq!(names.len(), LIGHTROOM_FILES.len());
        assert!(names.contains("Info.lua"));
        assert!(names.contains("Photara.lua"));
        assert_eq!(lightroom_package_sha256().len(), 64);
    }
}
