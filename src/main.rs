use std::collections::BTreeMap;

use clap::{Args, Parser, Subcommand, ValueEnum};
use photara::{
    Result,
    config::{Location, Person, PhotaraConfig, Scene, config_root},
    persistence,
    project::{self, NewProject, ProjectOrigin},
};
use serde::Serialize;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Health,
    Migrate,
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    People {
        #[command(subcommand)]
        command: PeopleCommand,
    },
    Locations {
        #[command(subcommand)]
        command: LocationsCommand,
    },
    Metadata {
        #[command(subcommand)]
        command: MetadataCommand,
    },
    Scenes {
        #[command(subcommand)]
        command: ScenesCommand,
    },
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Init,
    Validate,
}

#[derive(Debug, Subcommand)]
enum MetadataCommand {
    Plan { project: String },
}

#[derive(Debug, Subcommand)]
enum PeopleCommand {
    Add(PersonAdd),
    List(OutputArgs),
    Show { slug: String },
}

#[derive(Debug, Args)]
struct PersonAdd {
    slug: String,
    #[arg(long)]
    display_name: String,
    #[arg(long = "alias")]
    aliases: Vec<String>,
    #[arg(long = "role", required = true)]
    roles: Vec<String>,
    #[arg(long = "social", value_parser = parse_pair)]
    social: Vec<(String, String)>,
    #[arg(long)]
    replace: bool,
}

#[derive(Debug, Subcommand)]
enum LocationsCommand {
    Add(LocationAdd),
    List(OutputArgs),
    Show { slug: String },
}

#[derive(Debug, Args)]
struct LocationAdd {
    slug: String,
    #[arg(long)]
    display_name: String,
    #[arg(long)]
    sublocation: String,
    #[arg(long)]
    city: String,
    #[arg(long)]
    state: String,
    #[arg(long)]
    country: String,
    #[arg(long)]
    iso_country_code: String,
    #[arg(long)]
    replace: bool,
}

#[derive(Debug, Subcommand)]
enum ScenesCommand {
    Add(SceneAdd),
    List(OutputArgs),
    Show { slug: String },
}

#[derive(Debug, Args)]
struct SceneAdd {
    slug: String,
    #[arg(long)]
    display_name: String,
    #[arg(long)]
    description: Option<String>,
    #[arg(long)]
    replace: bool,
}

#[derive(Debug, Args)]
struct OutputArgs {
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum ProjectCommand {
    Init(ProjectArguments),
    Configure(ProjectArguments),
    Show { slug: String },
}

#[derive(Debug, Args)]
struct ProjectArguments {
    slug: String,
    #[arg(long)]
    display_name: String,
    #[arg(long)]
    scene: String,
    #[arg(long)]
    location: String,
    #[arg(long = "person")]
    people: Vec<String>,
    #[arg(long, value_enum, default_value = "native")]
    origin: Origin,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Origin {
    Native,
    Proetus,
    Adopted,
}

impl From<Origin> for ProjectOrigin {
    fn from(value: Origin) -> Self {
        match value {
            Origin::Native => Self::Native,
            Origin::Proetus => Self::Proetus,
            Origin::Adopted => Self::Adopted,
        }
    }
}

impl From<ProjectArguments> for NewProject {
    fn from(arguments: ProjectArguments) -> Self {
        Self {
            slug: arguments.slug,
            display_name: arguments.display_name,
            scene: arguments.scene,
            location: arguments.location,
            people: arguments.people,
            origin: arguments.origin.into(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .ok();

    match Cli::parse().command {
        Command::Health => health().await?,
        Command::Migrate => migrate().await?,
        Command::Config { command } => config(command)?,
        Command::People { command } => people(command)?,
        Command::Locations { command } => locations(command)?,
        Command::Metadata { command } => metadata(command).await?,
        Command::Scenes { command } => scenes(command)?,
        Command::Project { command } => project(command).await?,
    }
    Ok(())
}

async fn metadata(command: MetadataCommand) -> Result<()> {
    let config = PhotaraConfig::discover()?;
    config.validate()?;
    let database = persistence::connect_development().await?;
    match command {
        MetadataCommand::Plan { project: slug } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            println!(
                "{}",
                serde_json::to_string_pretty(&photara::metadata::plan(&config, &project)?)?
            );
        }
    }
    database.close().await;
    Ok(())
}

async fn health() -> Result<()> {
    let database = persistence::connect_development().await?;
    let report = database.health().await?;
    info!(
        provider = %database.provider(),
        server_version = %report.server_version,
        latency_ms = report.latency.as_millis(),
        "Photara database is healthy"
    );
    database.close().await;
    Ok(())
}

async fn migrate() -> Result<()> {
    let database = persistence::connect_development().await?;
    let report = persistence::migrate(&database).await?;
    info!(
        available = report.available,
        "Photara migrations are current"
    );
    database.close().await;
    Ok(())
}

fn config(command: ConfigCommand) -> Result<()> {
    match command {
        ConfigCommand::Init => {
            let root = PhotaraConfig::initialize(config_root()?)?;
            info!(path = %root.display(), "Photara configuration initialized");
        }
        ConfigCommand::Validate => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            info!(path = %config.root.display(), "Photara configuration is valid");
        }
    }
    Ok(())
}

fn people(command: PeopleCommand) -> Result<()> {
    let mut config = PhotaraConfig::discover()?;
    match command {
        PeopleCommand::Add(arguments) => {
            let slug = arguments.slug;
            config.add_person(
                slug.clone(),
                Person {
                    display_name: arguments.display_name,
                    aliases: arguments.aliases,
                    roles: arguments.roles,
                    social: arguments.social.into_iter().collect(),
                },
                arguments.replace,
            )?;
            info!(%slug, "person registry entry saved");
        }
        PeopleCommand::List(output) => list(&config.people, output.json)?,
        PeopleCommand::Show { slug } => show(&config.people, &slug)?,
    }
    Ok(())
}

fn locations(command: LocationsCommand) -> Result<()> {
    let mut config = PhotaraConfig::discover()?;
    match command {
        LocationsCommand::Add(arguments) => {
            let slug = arguments.slug;
            config.add_location(
                slug.clone(),
                Location {
                    display_name: arguments.display_name,
                    sublocation: arguments.sublocation,
                    city: arguments.city,
                    state: arguments.state,
                    country: arguments.country,
                    iso_country_code: arguments.iso_country_code,
                },
                arguments.replace,
            )?;
            info!(%slug, "location registry entry saved");
        }
        LocationsCommand::List(output) => list(&config.locations, output.json)?,
        LocationsCommand::Show { slug } => show(&config.locations, &slug)?,
    }
    Ok(())
}

fn scenes(command: ScenesCommand) -> Result<()> {
    let mut config = PhotaraConfig::discover()?;
    match command {
        ScenesCommand::Add(arguments) => {
            let slug = arguments.slug;
            config.add_scene(
                slug.clone(),
                Scene {
                    display_name: arguments.display_name,
                    description: arguments.description,
                },
                arguments.replace,
            )?;
            info!(%slug, "scene registry entry saved");
        }
        ScenesCommand::List(output) => list(&config.scenes, output.json)?,
        ScenesCommand::Show { slug } => show(&config.scenes, &slug)?,
    }
    Ok(())
}

async fn project(command: ProjectCommand) -> Result<()> {
    let config = PhotaraConfig::discover()?;
    config.validate()?;
    let database = persistence::connect_development().await?;

    let record = match command {
        ProjectCommand::Init(arguments) => {
            Some(project::initialize(&database, &config, arguments.into()).await?)
        }
        ProjectCommand::Configure(arguments) => {
            Some(project::reconfigure(&database, &config, arguments.into()).await?)
        }
        ProjectCommand::Show { slug } => match project::find(&database, &slug).await? {
            Some(project) => {
                println!("{project:#?}");
                None
            }
            None => {
                return Err(photara::PhotaraError::Configuration(format!(
                    "project {slug:?} was not found"
                )));
            }
        },
    };

    if let Some(record) = record {
        info!(project.id = %record.id, project.slug = %record.slug, "project saved");
    }
    database.close().await;
    Ok(())
}

fn list<T: Serialize>(entries: &BTreeMap<String, T>, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(entries)?);
    } else {
        for slug in entries.keys() {
            println!("{slug}");
        }
    }
    Ok(())
}

fn show<T: Serialize>(entries: &BTreeMap<String, T>, slug: &str) -> Result<()> {
    let entry = entries.get(slug).ok_or_else(|| {
        photara::PhotaraError::Configuration(format!("registry entry {slug:?} was not found"))
    })?;
    println!("{}", serde_json::to_string_pretty(entry)?);
    Ok(())
}

fn parse_pair(value: &str) -> std::result::Result<(String, String), String> {
    let (key, value) = value
        .split_once('=')
        .ok_or_else(|| "expected PLATFORM=HANDLE".to_owned())?;
    if key.is_empty() || value.is_empty() {
        return Err("platform and handle must not be empty".into());
    }
    Ok((key.to_owned(), value.to_owned()))
}
