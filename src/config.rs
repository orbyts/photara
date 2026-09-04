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
lightroom_inbox = "~/Pictures/Photara/Inbox"
default_catalog = "Lr_Photara"
default_creator = "Suhail"
default_author_code = "SUHAIL"
default_copyright = "@suhail"
default_country = "United States"
default_iso_country_code = "US"
proof_provider = "pixieset"
delivery_provider = "cloudinary"
templates_root = "$DROPBOX/Pictures/Photara/Templates"
templates_cache = "~/Library/Caches/photara/templates"

[layouts.defaults]
full_frame = "full-frame@1"
stacked_two = "stacked-two@1"
stacked_three = "stacked-three@2"
continuous_panorama = "continuous-panorama@1"
dynamic_range_comparison = "dynamic-range-comparison@2"
edit_comparison = "edit-comparison@1"
"#;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Settings {
    pub images_root: PathBuf,
    pub projects_root: PathBuf,
    #[serde(default = "default_lightroom_inbox")]
    pub lightroom_inbox: PathBuf,
    pub default_catalog: String,
    #[serde(default)]
    pub default_creator: Option<String>,
    #[serde(default = "default_author_code")]
    pub default_author_code: String,
    #[serde(default)]
    pub default_copyright: Option<String>,
    pub default_country: String,
    pub default_iso_country_code: String,
    pub proof_provider: String,
    pub delivery_provider: String,
    #[serde(default = "default_templates_root")]
    pub templates_root: PathBuf,
    #[serde(default = "default_templates_cache")]
    pub templates_cache: PathBuf,
    #[serde(default)]
    pub layouts: LayoutConfiguration,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LayoutConfiguration {
    #[serde(default)]
    pub defaults: LayoutDefaults,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LayoutDefaults {
    #[serde(default = "default_full_frame_template")]
    pub full_frame: String,
    #[serde(default = "default_stacked_two_template")]
    pub stacked_two: String,
    #[serde(default = "default_stacked_three_template")]
    pub stacked_three: String,
    #[serde(default = "default_continuous_panorama_template")]
    pub continuous_panorama: String,
    #[serde(default = "default_dynamic_range_comparison_template")]
    pub dynamic_range_comparison: String,
    #[serde(default = "default_edit_comparison_template")]
    pub edit_comparison: String,
}

impl Default for LayoutDefaults {
    fn default() -> Self {
        Self {
            full_frame: default_full_frame_template(),
            stacked_two: default_stacked_two_template(),
            stacked_three: default_stacked_three_template(),
            continuous_panorama: default_continuous_panorama_template(),
            dynamic_range_comparison: default_dynamic_range_comparison_template(),
            edit_comparison: default_edit_comparison_template(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Person {
    pub display_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub social: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Location {
    pub display_name: String,
    pub sublocation: String,
    pub city: String,
    pub state: String,
    pub country: String,
    pub iso_country_code: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
        let mut settings: Settings =
            toml::from_str(&settings_text).map_err(|source| PhotaraError::Toml {
                path: settings_path,
                source,
            })?;
        settings.apply_environment()?;

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
        if !self.settings.lightroom_inbox.is_absolute() {
            return Err(PhotaraError::Configuration(
                "lightroom_inbox must resolve to an absolute path".into(),
            ));
        }
        if !self.settings.templates_root.is_absolute() {
            return Err(PhotaraError::Configuration(
                "templates_root must resolve to an absolute path".into(),
            ));
        }
        if !self.settings.templates_cache.is_absolute() {
            return Err(PhotaraError::Configuration(
                "templates_cache must resolve to an absolute path".into(),
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
        validate_author_code(&self.settings.default_author_code)?;
        crate::layout::TemplateRef::parse(&self.settings.layouts.defaults.full_frame)?;
        crate::layout::TemplateRef::parse(&self.settings.layouts.defaults.stacked_two)?;
        crate::layout::TemplateRef::parse(&self.settings.layouts.defaults.stacked_three)?;
        crate::layout::TemplateRef::parse(&self.settings.layouts.defaults.continuous_panorama)?;
        crate::layout::TemplateRef::parse(
            &self.settings.layouts.defaults.dynamic_range_comparison,
        )?;
        crate::layout::TemplateRef::parse(&self.settings.layouts.defaults.edit_comparison)?;
        for (name, value) in [
            ("default_creator", &self.settings.default_creator),
            ("default_copyright", &self.settings.default_copyright),
        ] {
            if value.as_ref().is_some_and(|value| value.trim().is_empty()) {
                return Err(PhotaraError::Configuration(format!(
                    "{name} must be omitted or non-empty"
                )));
            }
        }
        validate_registry("person", &self.people)?;
        validate_registry("location", &self.locations)?;
        validate_registry("scene", &self.scenes)?;
        for person in self.people.values() {
            validate_person(person)?;
        }
        for location in self.locations.values() {
            validate_location(location)?;
        }
        for scene in self.scenes.values() {
            validate_scene(scene)?;
        }
        Ok(())
    }

    pub fn add_person(&mut self, slug: String, person: Person, replace: bool) -> Result<()> {
        validate_entry("person", &slug, &person.display_name)?;
        validate_person(&person)?;
        insert(&mut self.people, &slug, person, replace)?;
        write_yaml_atomic(self.root.join("config/people.yml"), &self.people)
    }

    pub fn add_location(&mut self, slug: String, location: Location, replace: bool) -> Result<()> {
        validate_entry("location", &slug, &location.display_name)?;
        validate_location(&location)?;
        insert(&mut self.locations, &slug, location, replace)?;
        write_yaml_atomic(self.root.join("config/locations.yml"), &self.locations)
    }

    pub fn add_scene(&mut self, slug: String, scene: Scene, replace: bool) -> Result<()> {
        validate_entry("scene", &slug, &scene.display_name)?;
        validate_scene(&scene)?;
        insert(&mut self.scenes, &slug, scene, replace)?;
        write_yaml_atomic(self.root.join("config/scenes.yml"), &self.scenes)
    }
}

fn default_author_code() -> String {
    "SUHAIL".into()
}

fn default_full_frame_template() -> String {
    "full-frame@1".into()
}

fn default_stacked_two_template() -> String {
    "stacked-two@1".into()
}

fn default_stacked_three_template() -> String {
    "stacked-three@2".into()
}

fn default_continuous_panorama_template() -> String {
    "continuous-panorama@1".into()
}

fn default_dynamic_range_comparison_template() -> String {
    "dynamic-range-comparison@2".into()
}

fn default_edit_comparison_template() -> String {
    "edit-comparison@1".into()
}

fn default_lightroom_inbox() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"))
        .join("Pictures/Photara/Inbox")
}

fn default_templates_root() -> PathBuf {
    env::var_os("DROPBOX")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("~"))
        .join("Pictures/Photara/Templates")
}

fn default_templates_cache() -> PathBuf {
    env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Caches")))
        .unwrap_or_else(|| PathBuf::from("~/.cache"))
        .join("photara/templates")
}

fn validate_author_code(value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(PhotaraError::Configuration(
            "default_author_code must contain only uppercase ASCII letters, digits, and hyphens"
                .into(),
        ));
    }
    Ok(())
}

impl Settings {
    fn apply_environment(&mut self) -> Result<()> {
        if let Some(value) = env::var_os("PHOTARA_IMAGES_ROOT") {
            self.images_root = PathBuf::from(value);
        }
        if let Some(value) = env::var_os("PHOTARA_PROJECTS_ROOT") {
            self.projects_root = PathBuf::from(value);
        }
        if let Some(value) = env::var_os("PHOTARA_LIGHTROOM_INBOX") {
            self.lightroom_inbox = PathBuf::from(value);
        }
        if let Some(value) = env::var_os("PHOTARA_TEMPLATES_ROOT") {
            self.templates_root = PathBuf::from(value);
        }
        if let Some(value) = env::var_os("PHOTARA_TEMPLATES_CACHE") {
            self.templates_cache = PathBuf::from(value);
        }
        self.lightroom_inbox = expand_home(&self.lightroom_inbox)?;
        self.templates_root = expand_environment(&expand_home(&self.templates_root)?)?;
        self.templates_cache = expand_environment(&expand_home(&self.templates_cache)?)?;
        Ok(())
    }
}

fn expand_environment(path: &Path) -> Result<PathBuf> {
    let value = path.to_string_lossy();
    if !value.starts_with('$') {
        return Ok(path.to_path_buf());
    }
    let remainder = &value[1..];
    let boundary = remainder.find('/').unwrap_or(remainder.len());
    let name = &remainder[..boundary];
    let root = env::var_os(name).ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "{} uses ${name}, but {name} is not configured",
            path.display()
        ))
    })?;
    let suffix = remainder[boundary..].trim_start_matches('/');
    Ok(PathBuf::from(root).join(suffix))
}

fn expand_home(path: &Path) -> Result<PathBuf> {
    let mut components = path.components();
    if components
        .next()
        .is_some_and(|part| part.as_os_str() == "~")
    {
        let home = env::var_os("HOME").ok_or_else(|| {
            PhotaraError::Configuration("lightroom_inbox uses ~ but HOME is not configured".into())
        })?;
        Ok(PathBuf::from(home).join(components.as_path()))
    } else {
        Ok(path.to_path_buf())
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

fn write_yaml_atomic<T: Serialize>(path: PathBuf, entries: &T) -> Result<()> {
    let contents = serde_yaml::to_string(entries).map_err(|source| PhotaraError::Yaml {
        path: path.clone(),
        source,
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| PhotaraError::Configuration("registry path has no filename".into()))?;
    let temporary = path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|source| PhotaraError::filesystem("create file", &temporary, source))?;
    file.write_all(contents.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|source| PhotaraError::filesystem("write file", &temporary, source))?;
    fs::rename(&temporary, &path)
        .map_err(|source| PhotaraError::filesystem("replace file", path, source))
}

fn insert<T>(entries: &mut BTreeMap<String, T>, slug: &str, value: T, replace: bool) -> Result<()> {
    if entries.contains_key(slug) && !replace {
        return Err(PhotaraError::Configuration(format!(
            "registry entry {slug:?} already exists; pass --replace to update it"
        )));
    }
    entries.insert(slug.to_owned(), value);
    Ok(())
}

fn validate_entry(kind: &str, slug: &str, display_name: &str) -> Result<()> {
    validate_slug(slug).map_err(|message| {
        PhotaraError::Configuration(format!("invalid {kind} slug {slug:?}: {message}"))
    })?;
    if display_name.trim().is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "{kind} display name must not be empty"
        )));
    }
    Ok(())
}

fn validate_person(person: &Person) -> Result<()> {
    if person.display_name.trim().is_empty() {
        return Err(PhotaraError::Configuration(
            "person display name must not be empty".into(),
        ));
    }
    if person.roles.is_empty() {
        return Err(PhotaraError::Configuration(
            "a person must have at least one role".into(),
        ));
    }
    for role in &person.roles {
        validate_slug(role).map_err(|message| {
            PhotaraError::Configuration(format!("invalid person role {role:?}: {message}"))
        })?;
    }
    for (platform, handle) in &person.social {
        validate_slug(platform).map_err(|message| {
            PhotaraError::Configuration(format!("invalid social platform {platform:?}: {message}"))
        })?;
        if handle.trim().is_empty() {
            return Err(PhotaraError::Configuration(format!(
                "social handle for {platform:?} must not be empty"
            )));
        }
    }
    Ok(())
}

fn validate_location(location: &Location) -> Result<()> {
    if location.display_name.trim().is_empty() || location.sublocation.trim().is_empty() {
        return Err(PhotaraError::Configuration(
            "location display name and sublocation must not be empty".into(),
        ));
    }
    let code = location.iso_country_code.as_bytes();
    if code.len() != 2 || !code.iter().all(u8::is_ascii_uppercase) {
        return Err(PhotaraError::Configuration(
            "location ISO country code must be two uppercase ASCII letters".into(),
        ));
    }
    Ok(())
}

fn validate_scene(scene: &Scene) -> Result<()> {
    if scene.display_name.trim().is_empty() {
        return Err(PhotaraError::Configuration(
            "scene display name must not be empty".into(),
        ));
    }
    Ok(())
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

    #[test]
    fn registry_write_is_sorted_and_round_trips_social_profiles() {
        let temporary = tempfile::tempdir().unwrap();
        PhotaraConfig::initialize(temporary.path()).unwrap();
        let mut config = PhotaraConfig::load(temporary.path()).unwrap();
        config
            .add_person(
                "trinity-woodward".into(),
                Person {
                    display_name: "Trinity Woodward".into(),
                    aliases: vec!["Trin".into(), "Trinity".into()],
                    roles: vec!["model".into()],
                    social: BTreeMap::from([
                        ("instagram".into(), "@theetr1n1ty".into()),
                        ("threads".into(), "@theetr1n1ty".into()),
                    ]),
                },
                false,
            )
            .unwrap();
        let loaded = PhotaraConfig::load(temporary.path()).unwrap();
        assert_eq!(
            loaded.people["trinity-woodward"],
            config.people["trinity-woodward"]
        );
    }
}
