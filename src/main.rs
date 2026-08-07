use clap::{Args, Parser, Subcommand, ValueEnum};
use photara::{
    Result,
    config::{PhotaraConfig, config_root},
    persistence,
    project::{self, NewProject, ProjectOrigin},
};
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
enum ProjectCommand {
    Init(ProjectInit),
    Show { slug: String },
}

#[derive(Debug, Args)]
struct ProjectInit {
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
        Command::Project { command } => project(command).await?,
    }
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

async fn project(command: ProjectCommand) -> Result<()> {
    let config = PhotaraConfig::discover()?;
    config.validate()?;
    let database = persistence::connect_development().await?;

    match command {
        ProjectCommand::Init(arguments) => {
            let record = project::initialize(
                &database,
                &config,
                NewProject {
                    slug: arguments.slug,
                    display_name: arguments.display_name,
                    scene: arguments.scene,
                    location: arguments.location,
                    people: arguments.people,
                    origin: arguments.origin.into(),
                },
            )
            .await?;
            info!(project.id = %record.id, project.slug = %record.slug, "project initialized");
        }
        ProjectCommand::Show { slug } => match project::find(&database, &slug).await? {
            Some(project) => println!("{project:#?}"),
            None => {
                return Err(photara::PhotaraError::Configuration(format!(
                    "project {slug:?} was not found"
                )));
            }
        },
    }

    database.close().await;
    Ok(())
}
