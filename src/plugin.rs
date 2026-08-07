use serde::Serialize;
use storexa::Database;

use crate::{Result, config::PhotaraConfig, project};

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
}
