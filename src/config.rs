use std::{
    collections::BTreeMap,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{PhotaraError, Result};

const SETTINGS: &str = r#"images_root = "/Volumes/whisk/Pictures/images"
projects_root = "/Volumes/whisk/Pictures/projects"
default_catalog = "Lr_Photara"
default_country = "United States"
default_iso_country_code = "US"
proof_provider = "pixieset"
delivery_provider = "cloudinary"
"#;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Settings {
    pub images_root: PathBuf,
    pub projects_root: PathBuf,
    pub default_catalog: String,
    pub default_country: String,
    pub default_iso_country_code: String,
    pub proof_provider: String,
    pub delivery_provider: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Person {
    pub display_name: String,
    #[serde(default)]
    pub instagram: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Location {
    pub display_name: String,
    pub sublocation: String,
    pub city: String,
    pub state: String,
    pub country: String,
    pub iso_country_code: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Scene {
    pub display_name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PhotaraConfig {
    pub root: PathBuf,
    pub settings: Settings,
    pub people: BTreeMap<String, Person>,
    pub locations: BTreeMap<String, Location>,
    pub scenes: BTreeMap<String, Scene>,
}

impl PhotaraConfig {
    pub fn discover() -> Result<Self> {
        Self::load(config_root()?)
    }

    pub fn load(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let config = root.join("config");
        let settings_path = config.join("photara.toml");
        let settings_text = read(&settings_path)?;
        let settings = toml::from_str(&settings_text).map_err(|source| PhotaraError::Toml {
            path: settings_path,
            source,
        })?;

        Ok(Self {
            root,
            settings,
            people: read_yaml(config.join("people.yml"))?,
            locations: read_yaml(config.join("locations.yml"))?,
            scenes: read_yaml(config.join("scenes.yml"))?,
        })
    }

    pub fn initialize(root: impl Into<PathBuf>) -> Result<PathBuf> {
        let root = root.into();
        let config = root.join("config");
        for directory in [
            config.clone(),
            root.join("cache"),
            root.join("schemas"),
            root.join("templates"),
        ] {
            fs::create_dir_all(&directory).map_err(|source| {
                PhotaraError::filesystem("create directory", directory, source)
            })?;
        }

        write_new(config.join("photara.toml"), SETTINGS)?;
        write_new(config.join("people.yml"), "{}\n")?;
        write_new(config.join("locations.yml"), "{}\n")?;
        write_new(config.join("scenes.yml"), "{}\n")?;
        Ok(root)
    }

    pub fn validate(&self) -> Result<()> {
        if !self.settings.images_root.is_absolute() {
            return Err(PhotaraError::Configuration(
                "images_root must be an absolute path".into(),
            ));
        }
        if !self.settings.projects_root.is_absolute() {
            return Err(PhotaraError::Configuration(
                "projects_root must be an absolute path".into(),
            ));
        }
        if self.settings.images_root == self.settings.projects_root {
            return Err(PhotaraError::Configuration(
                "images_root and projects_root must be different".into(),
            ));
        }
        if self.settings.default_catalog.trim().is_empty() {
            return Err(PhotaraError::Configuration(
                "default_catalog must not be empty".into(),
            ));
        }
        validate_registry("person", &self.people)?;
        validate_registry("location", &self.locations)?;
        validate_registry("scene", &self.scenes)?;
        Ok(())
    }
}

pub fn config_root() -> Result<PathBuf> {
    if let Some(root) = env::var_os("PHOTARA_CONFIG_ROOT") {
        return Ok(PathBuf::from(root));
    }
    env::var_os("XDG_CONFIG_HOME")
        .map(|root| PathBuf::from(root).join("photara"))
        .ok_or_else(|| {
            PhotaraError::Configuration("set PHOTARA_CONFIG_ROOT or XDG_CONFIG_HOME".into())
        })
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|source| PhotaraError::filesystem("read file", path, source))
}

fn read_yaml<T: DeserializeOwned>(path: PathBuf) -> Result<T> {
    let text = read(&path)?;
    serde_yaml::from_str(&text).map_err(|source| PhotaraError::Yaml { path, source })
}

fn write_new(path: PathBuf, contents: &str) -> Result<()> {
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path);
    match file {
        Ok(mut file) => file
            .write_all(contents.as_bytes())
            .map_err(|source| PhotaraError::filesystem("write file", path, source)),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(source) => Err(PhotaraError::filesystem("create file", path, source)),
    }
}

fn validate_registry<T>(kind: &str, entries: &BTreeMap<String, T>) -> Result<()> {
    for slug in entries.keys() {
        validate_slug(slug).map_err(|message| {
            PhotaraError::Configuration(format!("invalid {kind} slug {slug:?}: {message}"))
        })?;
    }
    Ok(())
}

pub(crate) fn validate_slug(slug: &str) -> std::result::Result<(), &'static str> {
    if slug.is_empty() {
        return Err("must not be empty");
    }
    if slug.starts_with('-') || slug.ends_with('-') {
        return Err("must not start or end with a hyphen");
    }
    if !slug
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err("must contain only lowercase ASCII letters, digits, and hyphens");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialization_is_idempotent_and_does_not_overwrite() {
        let temporary = tempfile::tempdir().unwrap();
        PhotaraConfig::initialize(temporary.path()).unwrap();
        let settings = temporary.path().join("config/photara.toml");
        fs::write(&settings, "custom = true\n").unwrap();
        PhotaraConfig::initialize(temporary.path()).unwrap();
        assert_eq!(fs::read_to_string(settings).unwrap(), "custom = true\n");
    }

    #[test]
    fn rejects_noncanonical_slugs() {
        assert!(validate_slug("red-meridian").is_ok());
        assert!(validate_slug("Red Meridian").is_err());
        assert!(validate_slug("red_meridian").is_err());
    }
}
