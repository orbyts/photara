use std::{collections::BTreeMap, io::IsTerminal, path::PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};
use photara::{
    Result, adobe,
    cloud::{self, ProetusImport},
    cloud_collection,
    config::{Location, Person, PhotaraConfig, Scene, config_root},
    decision::{self, DecisionValue},
    delivery,
    layout::{self, PostPlatform},
    master, persistence,
    project::{self, NewProject, ProjectOrigin},
    publication,
    selection::{self, SelectionAction, SelectionCorrection, SelectionKind, SelectionSource},
    transfer, withdrawal,
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

struct TerminalMasterProgress {
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct HealthReport {
    schema_version: u32,
    status: &'static str,
    provider: String,
    server_version: String,
    latency_ms: u128,
}

#[derive(Debug, Serialize)]
struct ConfigValidationReport {
    schema_version: u32,
    status: &'static str,
    config_root: PathBuf,
    images_root: PathBuf,
    projects_root: PathBuf,
    lightroom_inbox: PathBuf,
    templates_root: PathBuf,
}

impl TerminalMasterProgress {
    fn new() -> Self {
        Self {
            enabled: std::io::stderr().is_terminal(),
        }
    }

    fn report(&self, event: master::MasterProgress) {
        if self.enabled {
            eprintln!("{}", format_master_progress(&event));
        }
    }
}

fn format_master_progress(event: &master::MasterProgress) -> String {
    if event.asset.is_empty() {
        format!("{}  {}/{}", event.stage, event.current, event.total)
    } else {
        format!(
            "{}  {}/{}  {}",
            event.stage, event.current, event.total, event.asset
        )
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    Health,
    Migrate,
    Cloud {
        #[command(subcommand)]
        command: CloudCommand,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Decisions {
        #[command(subcommand)]
        command: DecisionCommand,
    },
    Delivery {
        #[command(subcommand)]
        command: DeliveryCommand,
    },
    People {
        #[command(subcommand)]
        command: PeopleCommand,
    },
    Locations {
        #[command(subcommand)]
        command: LocationsCommand,
    },
    Layouts {
        #[command(subcommand)]
        command: LayoutCommand,
    },
    Metadata {
        #[command(subcommand)]
        command: MetadataCommand,
    },
    Masters {
        #[command(subcommand)]
        command: MasterCommand,
    },
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
    Scenes {
        #[command(subcommand)]
        command: ScenesCommand,
    },
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    Posts {
        #[command(subcommand)]
        command: PostCommand,
    },
    Selections {
        #[command(subcommand)]
        command: SelectionCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Init,
    Validate,
}

#[derive(Debug, Subcommand)]
enum DeliveryCommand {
    CloudinaryLogin {
        #[arg(long, default_value = "personal")]
        account: String,
        #[arg(long)]
        cloud_name: String,
    },
    CloudinaryProbe {
        #[arg(long, default_value = "personal")]
        account: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Prepare {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long, default_value = "personal")]
        account: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    UploadCanary {
        batch: uuid::Uuid,
        #[arg(long)]
        confirm: bool,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    UploadRemaining {
        batch: uuid::Uuid,
        #[arg(long)]
        confirm: bool,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    VerifyCanary {
        batch: uuid::Uuid,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Verify {
        batch: uuid::Uuid,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
}

#[derive(Debug, Subcommand)]
enum LayoutCommand {
    Install {
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Show {
        template: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    InstallReference {
        template: String,
        source: PathBuf,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
}

#[derive(Debug, Subcommand)]
enum PostCommand {
    Init {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    AddFullFrame {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long)]
        item: String,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        template: Option<String>,
        #[arg(long, value_enum, default_value = "crop")]
        fit: PlacementFit,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    AddStackedTwo {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long)]
        item: String,
        #[arg(long)]
        top: String,
        #[arg(long)]
        bottom: String,
        #[arg(long)]
        top_crop_from_item: Option<String>,
        #[arg(long)]
        bottom_crop_from_item: Option<String>,
        #[arg(long)]
        template: Option<String>,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    AddStackedThree {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long)]
        item: String,
        #[arg(long)]
        top: String,
        #[arg(long)]
        middle: String,
        #[arg(long)]
        bottom: String,
        #[arg(long)]
        template: Option<String>,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    AddGridFour {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long)]
        item: String,
        #[arg(long)]
        top_left: String,
        #[arg(long)]
        top_right: String,
        #[arg(long)]
        bottom_left: String,
        #[arg(long)]
        bottom_right: String,
        #[arg(long)]
        template: Option<String>,
        #[arg(long, value_enum, default_value = "crop")]
        fit: PlacementFit,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    SetFit {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long)]
        item: String,
        #[arg(long)]
        slot: Option<String>,
        #[arg(long, value_enum)]
        fit: PlacementFit,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    AddContinuousPanorama {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long)]
        item: String,
        #[arg(long)]
        asset: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    AddDynamicRangeComparison {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long)]
        item: String,
        #[arg(long)]
        top: String,
        #[arg(long)]
        bottom: String,
        #[arg(long)]
        template: Option<String>,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    AddEditComparison {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long)]
        item: String,
        #[arg(long)]
        top: String,
        #[arg(long)]
        bottom: String,
        #[arg(long)]
        template: Option<String>,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    PrepareEditComparisonSources {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Option<Platform>,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    VerifyEditComparisonSources {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    PreparePanoramaCrop {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long)]
        item: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    ApplyPanoramaCrop {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long)]
        item: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    PrepareAuthoring {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long, value_enum)]
        also_platform: Option<Platform>,
        #[arg(long)]
        item: Option<String>,
        #[arg(long)]
        slot: Option<String>,
        #[arg(long)]
        reauthor: bool,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    ApplyAuthoring {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    SetTransform {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long)]
        item: String,
        #[arg(long)]
        slot: Option<String>,
        #[arg(long, default_value_t = 0)]
        rotation_quarter_turns_cw: u8,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    ConfirmManualPublication {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long, default_value = "personal")]
        account: String,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        published_at: Option<chrono::DateTime<chrono::Utc>>,
        #[arg(long, default_value = "Operator confirmed manual publication")]
        note: String,
        #[arg(long)]
        confirm: bool,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Show {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Reorder {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long = "item", required = true)]
        items: Vec<String>,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Resolve {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    PrepareRender {
        project: String,
        post: String,
        #[arg(long, value_enum)]
        platform: Platform,
        /// Resolve and render exactly one editorial item for review/debugging.
        #[arg(long)]
        item: Option<String>,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
}

#[derive(Debug, Subcommand)]
enum DecisionCommand {
    Add(DecisionUpdateArgs),
    Remove(DecisionUpdateArgs),
    History {
        project: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Plan {
        project: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
}

#[derive(Debug, Args)]
struct DecisionUpdateArgs {
    project: String,
    #[arg(long = "original", required = true)]
    originals: Vec<std::path::PathBuf>,
    #[arg(long, value_enum, default_value = "json")]
    format: SerializationFormat,
}

#[derive(Debug, Subcommand)]
enum CloudCommand {
    AdobeLogin {
        #[arg(long, default_value = "personal")]
        account: String,
    },
    AdobeLogout {
        #[arg(long, default_value = "personal")]
        account: String,
    },
    AdobeInventory {
        #[arg(long, default_value = "personal")]
        account: String,
    },
    AdobeProbe,
    AdobeStatus {
        #[arg(long, default_value = "personal")]
        account: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    AdobeVerify {
        #[arg(long, default_value = "personal")]
        account: String,
    },
    CleanupBatch {
        batch: uuid::Uuid,
        #[arg(long)]
        confirm: bool,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    CollectionPlan {
        project: String,
        #[arg(long, default_value = "personal")]
        account: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    BeginWithdrawal {
        project: String,
        #[arg(long)]
        original: std::path::PathBuf,
        #[arg(long, default_value = "personal")]
        account: String,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        confirm: bool,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    ExportBatch {
        batch: uuid::Uuid,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    FinishExport {
        batch: uuid::Uuid,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    ImportProetus(ProetusImportArgs),
    PresencePlan {
        #[arg(long, default_value = "personal")]
        account: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    WithdrawalPlan {
        project: String,
        #[arg(long)]
        original: std::path::PathBuf,
        #[arg(long, default_value = "personal")]
        account: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    WithdrawalKeywords {
        project: String,
        #[arg(long = "original", required = true)]
        originals: Vec<std::path::PathBuf>,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    VerifyWithdrawal {
        withdrawal: uuid::Uuid,
        #[arg(long, default_value = "personal")]
        account: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    RecordExport {
        batch: uuid::Uuid,
        #[arg(long)]
        asset: uuid::Uuid,
        #[arg(long)]
        file: std::path::PathBuf,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    ReserveTransfer {
        project: String,
        #[arg(long, default_value = "personal")]
        account: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Status {
        #[arg(long, default_value = "personal")]
        account: String,
    },
    StorageAudit,
    SyncCollections {
        project: String,
        #[arg(long, default_value = "personal")]
        account: String,
        #[arg(long)]
        confirm: bool,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    TransferPlan {
        project: String,
        #[arg(long, default_value = "personal")]
        account: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    UploadPreflight {
        batch: uuid::Uuid,
        #[arg(long, default_value = "personal")]
        account: String,
    },
    UploadCanary {
        batch: uuid::Uuid,
        #[arg(long, default_value = "personal")]
        account: String,
    },
    UploadRemaining {
        batch: uuid::Uuid,
        #[arg(long, default_value = "personal")]
        account: String,
    },
    VerifyCanary {
        batch: uuid::Uuid,
        #[arg(long, default_value = "personal")]
        account: String,
    },
    VerifyBatch {
        batch: uuid::Uuid,
        #[arg(long, default_value = "personal")]
        account: String,
    },
}

#[derive(Debug, Args)]
struct ProetusImportArgs {
    #[arg(long)]
    database: std::path::PathBuf,
    #[arg(long, default_value = "personal")]
    account: String,
    #[arg(long)]
    confirmed_present: bool,
}

#[derive(Debug, Subcommand)]
enum MetadataCommand {
    Plan {
        project: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
}

#[derive(Debug, Subcommand)]
enum MasterCommand {
    InstallScripts {
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Prepare {
        project: String,
        #[arg(long)]
        canary: Option<String>,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Status {
        project: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Verify {
        project: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Promote {
        project: String,
        #[arg(long)]
        confirm: bool,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    CatalogPlan {
        project: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Checkpoint {
        project: String,
        #[arg(long)]
        asset: Option<String>,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    MarkReady {
        project: String,
        #[arg(long)]
        confirm: bool,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    PrepareFlattening {
        project: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    VerifyFlattening {
        project: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    RegisterFlattening {
        project: String,
        #[arg(long)]
        confirm: bool,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    RefreshFlattened {
        project: String,
        #[arg(long)]
        asset: String,
        #[arg(long, alias = "confirm")]
        r#override: bool,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
}

#[derive(Debug, Subcommand)]
enum PluginCommand {
    Install {
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Status {
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Uninstall {
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Context {
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
}

#[derive(Debug, Subcommand)]
enum SelectionCommand {
    ImportPixieset(PixiesetImport),
    Add {
        project: String,
        #[arg(long)]
        asset: String,
        #[arg(long = "to")]
        kind: SelectionKind,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Remove {
        project: String,
        #[arg(long)]
        asset: String,
        #[arg(long = "from")]
        kind: SelectionKind,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        cascade: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Status {
        project: String,
        #[arg(long)]
        asset: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    History {
        project: String,
        #[arg(long)]
        asset: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
    Plan {
        project: String,
        #[arg(long, value_enum, default_value = "json")]
        format: SerializationFormat,
    },
}

#[derive(Debug, Args)]
struct PixiesetImport {
    project: String,
    #[arg(long)]
    source_root: std::path::PathBuf,
    #[arg(long)]
    client_favorites: std::path::PathBuf,
    #[arg(long)]
    client_shortlist: std::path::PathBuf,
    #[arg(long)]
    hero: std::path::PathBuf,
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

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Platform {
    Instagram,
    Threads,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum PlacementFit {
    Fill,
    Contain,
    Crop,
}

impl PlacementFit {
    fn as_str(self) -> &'static str {
        match self {
            Self::Fill => "fill",
            Self::Contain => "contain",
            Self::Crop => "crop",
        }
    }
}

impl From<Platform> for PostPlatform {
    fn from(value: Platform) -> Self {
        match value {
            Platform::Instagram => Self::Instagram,
            Platform::Threads => Self::Threads,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum SerializationFormat {
    Json,
    Lua,
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
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .ok();

    match Cli::parse().command {
        Command::Health => health().await?,
        Command::Migrate => migrate().await?,
        Command::Cloud { command } => cloud_command(command).await?,
        Command::Config { command } => config(command)?,
        Command::Decisions { command } => decisions(command).await?,
        Command::Delivery { command } => delivery_command(command).await?,
        Command::People { command } => people(command)?,
        Command::Locations { command } => locations(command)?,
        Command::Layouts { command } => layouts(command)?,
        Command::Metadata { command } => metadata(command).await?,
        Command::Masters { command } => masters(command).await?,
        Command::Plugin { command } => plugin(command).await?,
        Command::Scenes { command } => scenes(command)?,
        Command::Project { command } => project(command).await?,
        Command::Posts { command } => posts(command).await?,
        Command::Selections { command } => selections(command).await?,
    }
    Ok(())
}

async fn delivery_command(command: DeliveryCommand) -> Result<()> {
    match command {
        DeliveryCommand::CloudinaryLogin {
            account,
            cloud_name,
        } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&delivery::login(&account, &cloud_name).await?)?
            );
        }
        DeliveryCommand::CloudinaryProbe { account, format } => {
            print_serialized(&delivery::probe(&account).await?, format)?;
        }
        DeliveryCommand::Prepare {
            project: slug,
            post,
            platform,
            account,
            format,
        } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let database = persistence::connect_development().await?;
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &delivery::prepare(
                    &database,
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    &account,
                )
                .await?,
                format,
            )?;
            database.close().await;
        }
        DeliveryCommand::UploadCanary {
            batch,
            confirm,
            format,
        } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let database = persistence::connect_development().await?;
            print_serialized(
                &delivery::upload_canary(&database, &config, batch, confirm).await?,
                format,
            )?;
            database.close().await;
        }
        DeliveryCommand::UploadRemaining {
            batch,
            confirm,
            format,
        } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let database = persistence::connect_development().await?;
            print_serialized(
                &delivery::upload_remaining(&database, &config, batch, confirm).await?,
                format,
            )?;
            database.close().await;
        }
        DeliveryCommand::VerifyCanary { batch, format } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let database = persistence::connect_development().await?;
            print_serialized(
                &delivery::verify_canary(&database, &config, batch).await?,
                format,
            )?;
            database.close().await;
        }
        DeliveryCommand::Verify { batch, format } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let database = persistence::connect_development().await?;
            print_serialized(&delivery::verify(&database, &config, batch).await?, format)?;
            database.close().await;
        }
    }
    Ok(())
}

async fn decisions(command: DecisionCommand) -> Result<()> {
    let config = PhotaraConfig::discover()?;
    config.validate()?;
    let database = persistence::connect_development().await?;
    match command {
        DecisionCommand::Add(arguments) => {
            let project = project::find(&database, &arguments.project)
                .await?
                .ok_or_else(|| {
                    photara::PhotaraError::Configuration(format!(
                        "project {:?} was not found",
                        arguments.project
                    ))
                })?;
            print_serialized(
                &decision::update(
                    &database,
                    &config,
                    &project,
                    DecisionValue::Selected,
                    &arguments.originals,
                )
                .await?,
                arguments.format,
            )?;
        }
        DecisionCommand::Remove(arguments) => {
            let project = project::find(&database, &arguments.project)
                .await?
                .ok_or_else(|| {
                    photara::PhotaraError::Configuration(format!(
                        "project {:?} was not found",
                        arguments.project
                    ))
                })?;
            print_serialized(
                &decision::update(
                    &database,
                    &config,
                    &project,
                    DecisionValue::Rejected,
                    &arguments.originals,
                )
                .await?,
                arguments.format,
            )?;
        }
        DecisionCommand::History {
            project: slug,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(&decision::history(&database, &project).await?, format)?;
        }
        DecisionCommand::Plan {
            project: slug,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(&decision::plan(&database, &project).await?, format)?;
        }
    }
    database.close().await;
    Ok(())
}

async fn masters(command: MasterCommand) -> Result<()> {
    match command {
        MasterCommand::InstallScripts { format } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            print_serialized(&master::install_photoshop_scripts(&config)?, format)?;
        }
        MasterCommand::Prepare {
            project: slug,
            canary,
            format,
        } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let database = persistence::connect_development().await?;
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &master::prepare(&database, &config, &project, canary.as_deref()).await?,
                format,
            )?;
            database.close().await;
        }
        MasterCommand::Status { project, format } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            print_serialized(&master::status(&config, &project)?, format)?;
        }
        MasterCommand::Verify { project, format } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            print_serialized(&master::verify(&config, &project)?, format)?;
        }
        MasterCommand::Promote {
            project: slug,
            confirm,
            format,
        } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let database = persistence::connect_development().await?;
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &master::promote(&database, &config, &project, confirm).await?,
                format,
            )?;
            database.close().await;
        }
        MasterCommand::CatalogPlan {
            project: slug,
            format,
        } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let database = persistence::connect_development().await?;
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &master::catalog_plan(&database, &config, &project).await?,
                format,
            )?;
            database.close().await;
        }
        MasterCommand::Checkpoint {
            project: slug,
            asset,
            format,
        } => {
            master_checkpoint(&slug, false, true, asset.as_deref(), format).await?;
        }
        MasterCommand::MarkReady {
            project: slug,
            confirm,
            format,
        } => {
            master_checkpoint(&slug, true, confirm, None, format).await?;
        }
        MasterCommand::PrepareFlattening {
            project: slug,
            format,
        } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let database = persistence::connect_development().await?;
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            let progress = TerminalMasterProgress::new();
            let reporter = |event| progress.report(event);
            print_serialized(
                &master::prepare_flattening(&database, &config, &project, Some(&reporter)).await?,
                format,
            )?;
            database.close().await;
        }
        MasterCommand::VerifyFlattening { project, format } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let progress = TerminalMasterProgress::new();
            let reporter = |event| progress.report(event);
            print_serialized(
                &master::verify_flattening(&config, &project, Some(&reporter))?,
                format,
            )?;
        }
        MasterCommand::RegisterFlattening {
            project: slug,
            confirm,
            format,
        } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let database = persistence::connect_development().await?;
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            let progress = TerminalMasterProgress::new();
            let reporter = |event| progress.report(event);
            print_serialized(
                &master::register_flattening(
                    &database,
                    &config,
                    &project,
                    confirm,
                    Some(&reporter),
                )
                .await?,
                format,
            )?;
            database.close().await;
        }
        MasterCommand::RefreshFlattened {
            project: slug,
            asset,
            r#override,
            format,
        } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let database = persistence::connect_development().await?;
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &master::refresh_flattened(&database, &config, &project, &asset, r#override)
                    .await?,
                format,
            )?;
            database.close().await;
        }
    }
    Ok(())
}

async fn master_checkpoint(
    slug: &str,
    ready: bool,
    confirmed: bool,
    asset: Option<&str>,
    format: SerializationFormat,
) -> Result<()> {
    let config = PhotaraConfig::discover()?;
    config.validate()?;
    let database = persistence::connect_development().await?;
    let project = project::find(&database, slug).await?.ok_or_else(|| {
        photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
    })?;
    let progress = TerminalMasterProgress::new();
    let reporter = |event| progress.report(event);
    print_serialized(
        &master::checkpoint(
            &database,
            &config,
            &project,
            ready,
            confirmed,
            asset,
            Some(&reporter),
        )
        .await?,
        format,
    )?;
    database.close().await;
    Ok(())
}

async fn cloud_command(command: CloudCommand) -> Result<()> {
    let command = match command {
        CloudCommand::AdobeLogout { account } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&adobe::logout(&account)?)?
            );
            return Ok(());
        }
        CloudCommand::AdobeProbe => {
            let report = adobe::probe().await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            return Ok(());
        }
        CloudCommand::AdobeStatus { account, format } => {
            print_serialized(&adobe::status(&account)?, format)?;
            return Ok(());
        }
        command => command,
    };
    let database = persistence::connect_development().await?;
    match command {
        CloudCommand::AdobeLogin { account } => {
            let report = adobe::login(&account).await?;
            cloud::register_remote_catalog(&database, &account, &report.catalog_id).await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CloudCommand::AdobeInventory { account } => {
            let inventory = adobe::inventory(&account).await?;
            cloud::register_remote_catalog(&database, &account, &inventory.catalog_id).await?;
            let report = cloud::record_adobe_inventory(&database, &account, &inventory).await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CloudCommand::AdobeLogout { .. }
        | CloudCommand::AdobeProbe
        | CloudCommand::AdobeStatus { .. } => unreachable!(),
        CloudCommand::AdobeVerify { account } => {
            let report = adobe::verify(&account).await?;
            cloud::register_remote_catalog(&database, &account, &report.catalog_id).await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CloudCommand::BeginWithdrawal {
            project: slug,
            original,
            account,
            reason,
            confirm,
            format,
        } => {
            if !confirm {
                return Err(photara::PhotaraError::Configuration(
                    "begin-withdrawal records a Cloud deletion intent; inspect withdrawal-plan, then retry with --confirm".into(),
                ));
            }
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &withdrawal::begin(&database, &project, &account, &original, reason.as_deref())
                    .await?,
                format,
            )?;
        }
        CloudCommand::CleanupBatch {
            batch,
            confirm,
            format,
        } => {
            print_serialized(
                &transfer::cleanup_batch(&database, batch, confirm).await?,
                format,
            )?;
        }
        CloudCommand::CollectionPlan {
            project: slug,
            account,
            format,
        } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &cloud_collection::plan(&database, &config, &project, &account).await?,
                format,
            )?;
        }
        CloudCommand::ExportBatch { batch, format } => {
            print_serialized(&transfer::begin_export(&database, batch).await?, format)?;
        }
        CloudCommand::FinishExport { batch, format } => {
            print_serialized(&transfer::finish_export(&database, batch).await?, format)?;
        }
        CloudCommand::ImportProetus(arguments) => {
            let report = cloud::import_proetus_evidence(
                &database,
                &ProetusImport {
                    database_path: arguments.database,
                    account_label: arguments.account,
                    confirmed_present: arguments.confirmed_present,
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CloudCommand::PresencePlan { account, format } => {
            print_serialized(&cloud::presence_plan(&database, &account).await?, format)?;
        }
        CloudCommand::WithdrawalPlan {
            project: slug,
            original,
            account,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &withdrawal::plan(&database, &project, &account, &original).await?,
                format,
            )?;
        }
        CloudCommand::WithdrawalKeywords {
            project: slug,
            originals,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &withdrawal::keyword_plan(&database, &project, &originals).await?,
                format,
            )?;
        }
        CloudCommand::RecordExport {
            batch,
            asset,
            file,
            format,
        } => {
            print_serialized(
                &transfer::record_export(&database, batch, asset, &file).await?,
                format,
            )?;
        }
        CloudCommand::ReserveTransfer {
            project: slug,
            account,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &transfer::reserve(&database, &project, &account).await?,
                format,
            )?;
        }
        CloudCommand::Status { account } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&cloud::status(&database, &account).await?)?
            );
        }
        CloudCommand::StorageAudit => {
            println!(
                "{}",
                serde_json::to_string_pretty(&cloud::storage_audit(&database).await?)?
            );
        }
        CloudCommand::SyncCollections {
            project: slug,
            account,
            confirm,
            format,
        } => {
            if !confirm {
                return Err(photara::PhotaraError::Configuration(
                    "sync-collections creates or updates Lightroom Cloud albums; inspect collection-plan, then retry with --confirm".into(),
                ));
            }
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            let inventory = adobe::inventory(&account).await?;
            cloud::register_remote_catalog(&database, &account, &inventory.catalog_id).await?;
            cloud::record_adobe_inventory(&database, &account, &inventory).await?;
            let plan = cloud_collection::plan(&database, &config, &project, &account).await?;
            let provider = adobe::sync_collections(&account, &plan).await?;
            if provider.verified_membership_count != plan.album_membership_count {
                return Err(photara::PhotaraError::Configuration(
                    "Adobe verified a different number of album memberships than Photara planned"
                        .into(),
                ));
            }
            let ledger = cloud_collection::record_sync(&database, &plan).await?;
            print_serialized(
                &serde_json::json!({
                    "provider": provider,
                    "ledger": ledger,
                    "paths": plan.nodes,
                }),
                format,
            )?;
        }
        CloudCommand::TransferPlan {
            project: slug,
            account,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &transfer::plan(&database, &project, &account).await?,
                format,
            )?;
        }
        CloudCommand::UploadPreflight { batch, account } => {
            let requirements = transfer::upload_requirements(&database, batch).await?;
            let adobe = adobe::upload_preflight(&account, requirements.required_bytes).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "batch": requirements,
                    "adobe": adobe,
                }))?
            );
        }
        CloudCommand::UploadCanary { batch, account } => {
            let requirements = transfer::upload_requirements(&database, batch).await?;
            let preflight = adobe::upload_preflight(&account, requirements.required_bytes).await?;
            if !preflight.ready {
                return Err(photara::PhotaraError::Configuration(
                    "Adobe upload preflight did not pass".into(),
                ));
            }
            let canary = transfer::prepare_canary_upload(&database, batch, &account).await?;
            let upload = adobe::upload_asset(
                &account,
                &canary.remote_asset_id,
                &canary.filename,
                &canary.sha256,
                canary.byte_size,
                canary.capture_date,
                &canary.staged_path,
            )
            .await?;
            transfer::mark_canary_uploaded(&database, &canary).await?;
            tokio::time::sleep(std::time::Duration::from_secs(8)).await;
            let inventory = adobe::inventory(&account).await?;
            cloud::register_remote_catalog(&database, &account, &inventory.catalog_id).await?;
            let inventory_report =
                cloud::record_adobe_inventory(&database, &account, &inventory).await?;
            let verification = transfer::verify_canary(&database, batch).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "upload": upload,
                    "inventory": inventory_report,
                    "verification": verification,
                }))?
            );
        }
        CloudCommand::VerifyCanary { batch, account } => {
            let inventory = adobe::inventory(&account).await?;
            cloud::register_remote_catalog(&database, &account, &inventory.catalog_id).await?;
            let inventory_report =
                cloud::record_adobe_inventory(&database, &account, &inventory).await?;
            let verification = transfer::verify_canary(&database, batch).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "inventory": inventory_report,
                    "verification": verification,
                }))?
            );
        }
        CloudCommand::UploadRemaining { batch, account } => {
            let requirements = transfer::upload_requirements(&database, batch).await?;
            let preflight = adobe::upload_preflight(&account, requirements.required_bytes).await?;
            if !preflight.ready {
                return Err(photara::PhotaraError::Configuration(
                    "Adobe upload preflight did not pass".into(),
                ));
            }
            let mut uploads = Vec::with_capacity(requirements.upload_count);
            for _ in 0..requirements.upload_count {
                let item = transfer::prepare_canary_upload(&database, batch, &account).await?;
                let upload = adobe::upload_asset(
                    &account,
                    &item.remote_asset_id,
                    &item.filename,
                    &item.sha256,
                    item.byte_size,
                    item.capture_date,
                    &item.staged_path,
                )
                .await?;
                transfer::mark_canary_uploaded(&database, &item).await?;
                uploads.push(upload);
            }
            transfer::begin_batch_verification(&database, batch).await?;
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let inventory = adobe::inventory(&account).await?;
            cloud::register_remote_catalog(&database, &account, &inventory.catalog_id).await?;
            let inventory_report =
                cloud::record_adobe_inventory(&database, &account, &inventory).await?;
            let verification = transfer::verify_uploaded_batch(&database, batch).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "uploads": uploads,
                    "inventory": inventory_report,
                    "verification": verification,
                }))?
            );
        }
        CloudCommand::VerifyBatch { batch, account } => {
            let inventory = adobe::inventory(&account).await?;
            cloud::register_remote_catalog(&database, &account, &inventory.catalog_id).await?;
            let inventory_report =
                cloud::record_adobe_inventory(&database, &account, &inventory).await?;
            let verification = transfer::verify_uploaded_batch(&database, batch).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "inventory": inventory_report,
                    "verification": verification,
                }))?
            );
        }
        CloudCommand::VerifyWithdrawal {
            withdrawal: withdrawal_id,
            account,
            format,
        } => {
            let inventory = adobe::inventory(&account).await?;
            cloud::register_remote_catalog(&database, &account, &inventory.catalog_id).await?;
            cloud::record_adobe_inventory(&database, &account, &inventory).await?;
            print_serialized(
                &withdrawal::verify(&database, withdrawal_id, &account).await?,
                format,
            )?;
        }
    }
    database.close().await;
    Ok(())
}

async fn selections(command: SelectionCommand) -> Result<()> {
    let config = PhotaraConfig::discover()?;
    config.validate()?;
    let database = persistence::connect_development().await?;
    match command {
        SelectionCommand::ImportPixieset(arguments) => {
            let project = project::find(&database, &arguments.project)
                .await?
                .ok_or_else(|| {
                    photara::PhotaraError::Configuration(format!(
                        "project {:?} was not found",
                        arguments.project
                    ))
                })?;
            let sources = [
                SelectionSource {
                    kind: SelectionKind::ClientFavorite,
                    path: arguments.client_favorites,
                },
                SelectionSource {
                    kind: SelectionKind::ClientShortlist,
                    path: arguments.client_shortlist,
                },
                SelectionSource {
                    kind: SelectionKind::Hero,
                    path: arguments.hero,
                },
            ];
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &selection::import_pixieset(
                        &database,
                        &project,
                        &arguments.source_root,
                        &sources,
                    )
                    .await?
                )?
            );
        }
        SelectionCommand::Add {
            project: slug,
            asset,
            kind,
            reason,
            dry_run,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &selection::correct(
                    &database,
                    &project,
                    SelectionCorrection {
                        asset_reference: &asset,
                        kind,
                        action: SelectionAction::Add,
                        reason: &reason,
                        cascade: false,
                        dry_run,
                    },
                )
                .await?,
                format,
            )?;
        }
        SelectionCommand::Remove {
            project: slug,
            asset,
            kind,
            reason,
            cascade,
            dry_run,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &selection::correct(
                    &database,
                    &project,
                    SelectionCorrection {
                        asset_reference: &asset,
                        kind,
                        action: SelectionAction::Remove,
                        reason: &reason,
                        cascade,
                        dry_run,
                    },
                )
                .await?,
                format,
            )?;
        }
        SelectionCommand::Status {
            project: slug,
            asset,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &selection::status(&database, &project, &asset).await?,
                format,
            )?;
        }
        SelectionCommand::History {
            project: slug,
            asset,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &selection::history(&database, &project, &asset).await?,
                format,
            )?;
        }
        SelectionCommand::Plan {
            project: slug,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(&selection::plan(&database, &project).await?, format)?;
        }
    }
    database.close().await;
    Ok(())
}

async fn metadata(command: MetadataCommand) -> Result<()> {
    let config = PhotaraConfig::discover()?;
    config.validate()?;
    let database = persistence::connect_development().await?;
    match command {
        MetadataCommand::Plan {
            project: slug,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(&photara::metadata::plan(&config, &project)?, format)?;
        }
    }
    database.close().await;
    Ok(())
}

async fn plugin(command: PluginCommand) -> Result<()> {
    match command {
        PluginCommand::Install { format } => {
            print_serialized(&photara::plugin::install_lightroom_plugin()?, format)?;
            return Ok(());
        }
        PluginCommand::Status { format } => {
            print_serialized(&photara::plugin::lightroom_status()?, format)?;
            return Ok(());
        }
        PluginCommand::Uninstall { format } => {
            print_serialized(&photara::plugin::uninstall_lightroom_plugin()?, format)?;
            return Ok(());
        }
        PluginCommand::Context { .. } => {}
    }
    let config = PhotaraConfig::discover()?;
    config.validate()?;
    let database = persistence::connect_development().await?;
    match command {
        PluginCommand::Context { format } => {
            print_serialized(&photara::plugin::context(&database, &config).await?, format)?;
        }
        PluginCommand::Install { .. }
        | PluginCommand::Status { .. }
        | PluginCommand::Uninstall { .. } => unreachable!(),
    }
    database.close().await;
    Ok(())
}

fn print_serialized<T: Serialize>(value: &T, format: SerializationFormat) -> Result<()> {
    match format {
        SerializationFormat::Json => println!("{}", serde_json::to_string_pretty(value)?),
        SerializationFormat::Lua => print!("{}", photara::plugin::to_lua(value)?),
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
    println!(
        "{}",
        serde_json::to_string_pretty(&HealthReport {
            schema_version: 1,
            status: "healthy",
            provider: database.provider().to_string(),
            server_version: report.server_version,
            latency_ms: report.latency.as_millis(),
        })?
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
            println!(
                "{}",
                serde_json::to_string_pretty(&ConfigValidationReport {
                    schema_version: 1,
                    status: "valid",
                    config_root: config.root,
                    images_root: config.settings.images_root,
                    projects_root: config.settings.projects_root,
                    lightroom_inbox: config.settings.lightroom_inbox,
                    templates_root: config.settings.templates_root,
                })?
            );
        }
    }
    Ok(())
}

fn layouts(command: LayoutCommand) -> Result<()> {
    match command {
        LayoutCommand::Install { format } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            print_serialized(
                &layout::install_builtin_templates(&config.settings.templates_root)?,
                format,
            )?;
        }
        LayoutCommand::Show { template, format } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            print_serialized(&layout::load_template(&config, &template)?, format)?;
        }
        LayoutCommand::InstallReference {
            template,
            source,
            format,
        } => {
            let config = PhotaraConfig::discover()?;
            config.validate()?;
            print_serialized(
                &layout::install_template_reference(&config, &template, &source)?,
                format,
            )?;
        }
    }
    Ok(())
}

async fn posts(command: PostCommand) -> Result<()> {
    let config = PhotaraConfig::discover()?;
    config.validate()?;
    let database = persistence::connect_development().await?;
    match command {
        PostCommand::Init {
            project: slug,
            post,
            platform,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::initialize_post(&config, &project, &post, platform.into())?,
                format,
            )?;
        }
        PostCommand::AddFullFrame {
            project: slug,
            post,
            platform,
            item,
            asset,
            template,
            fit,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::add_full_frame(
                    &database,
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    &item,
                    &asset,
                    fit.as_str(),
                    template,
                )
                .await?,
                format,
            )?;
        }
        PostCommand::AddStackedTwo {
            project: slug,
            post,
            platform,
            item,
            top,
            bottom,
            top_crop_from_item,
            bottom_crop_from_item,
            template,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::add_stacked_two(
                    &database,
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    &item,
                    &top,
                    &bottom,
                    top_crop_from_item.as_deref(),
                    bottom_crop_from_item.as_deref(),
                    template,
                )
                .await?,
                format,
            )?;
        }
        PostCommand::AddStackedThree {
            project: slug,
            post,
            platform,
            item,
            top,
            middle,
            bottom,
            template,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::add_stacked_three(
                    &database,
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    &item,
                    &top,
                    &middle,
                    &bottom,
                    template,
                )
                .await?,
                format,
            )?;
        }
        PostCommand::AddGridFour {
            project: slug,
            post,
            platform,
            item,
            top_left,
            top_right,
            bottom_left,
            bottom_right,
            template,
            fit,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::add_grid_four(
                    &database,
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    &item,
                    &top_left,
                    &top_right,
                    &bottom_left,
                    &bottom_right,
                    fit.as_str(),
                    template,
                )
                .await?,
                format,
            )?;
        }
        PostCommand::SetFit {
            project: slug,
            post,
            platform,
            item,
            slot,
            fit,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::set_item_fit(
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    &item,
                    slot.as_deref(),
                    fit.as_str(),
                )?,
                format,
            )?;
        }
        PostCommand::AddContinuousPanorama {
            project: slug,
            post,
            platform,
            item,
            asset,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::add_continuous_panorama(
                    &database,
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    &item,
                    &asset,
                )
                .await?,
                format,
            )?;
        }
        PostCommand::AddDynamicRangeComparison {
            project: slug,
            post,
            platform,
            item,
            top,
            bottom,
            template,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::add_dynamic_range_comparison(
                    &database,
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    &item,
                    &top,
                    &bottom,
                    template.as_deref(),
                )
                .await?,
                format,
            )?;
        }
        PostCommand::AddEditComparison {
            project: slug,
            post,
            platform,
            item,
            top,
            bottom,
            template,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::add_edit_comparison(
                    &database,
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    &item,
                    &top,
                    &bottom,
                    template.as_deref(),
                )
                .await?,
                format,
            )?;
        }
        PostCommand::PrepareEditComparisonSources {
            project: slug,
            post,
            platform,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::prepare_edit_comparison_sources(
                    &database,
                    &config,
                    &project,
                    &post,
                    platform.map(Into::into),
                )
                .await?,
                format,
            )?;
        }
        PostCommand::VerifyEditComparisonSources {
            project: slug,
            post,
            platform,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::verify_edit_comparison_sources(&config, &project, &post, platform.into())?,
                format,
            )?;
        }
        PostCommand::PreparePanoramaCrop {
            project: slug,
            post,
            platform,
            item,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::prepare_panorama_crop(
                    &database,
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    &item,
                )
                .await?,
                format,
            )?;
        }
        PostCommand::ApplyPanoramaCrop {
            project: slug,
            post,
            platform,
            item,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::apply_panorama_crop(&config, &project, &post, platform.into(), &item)?,
                format,
            )?;
        }
        PostCommand::PrepareAuthoring {
            project: slug,
            post,
            platform,
            also_platform,
            item,
            slot,
            reauthor,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::prepare_dual_platform_authoring_session(
                    &database,
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    also_platform.map(Into::into),
                    item.as_deref(),
                    slot.as_deref(),
                    reauthor,
                )
                .await?,
                format,
            )?;
        }
        PostCommand::ApplyAuthoring {
            project: slug,
            post,
            platform,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::apply_dual_platform_authoring(&config, &project, &post, platform.into())?,
                format,
            )?;
        }
        PostCommand::SetTransform {
            project: slug,
            post,
            platform,
            item,
            slot,
            rotation_quarter_turns_cw,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::set_item_transform(
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    &item,
                    slot.as_deref(),
                    layout::PlacementTransform {
                        crop: None,
                        rotation_quarter_turns_cw,
                    },
                )?,
                format,
            )?;
        }
        PostCommand::ConfirmManualPublication {
            project: slug,
            post,
            platform,
            account,
            url,
            published_at,
            note,
            confirm,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &publication::confirm_manual(
                    &database,
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    publication::ManualPublicationInput {
                        account_label: &account,
                        external_url: url.as_deref(),
                        published_at,
                        evidence_note: &note,
                        confirmed: confirm,
                    },
                )
                .await?,
                format,
            )?;
        }
        PostCommand::Show {
            project: slug,
            post,
            platform,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::show_post(&config, &project, &post, platform.into())?,
                format,
            )?;
        }
        PostCommand::Reorder {
            project: slug,
            post,
            platform,
            items,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::reorder_post(&config, &project, &post, platform.into(), &items)?,
                format,
            )?;
        }
        PostCommand::Resolve {
            project: slug,
            post,
            platform,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::resolve_post(&database, &config, &project, &post, platform.into()).await?,
                format,
            )?;
        }
        PostCommand::PrepareRender {
            project: slug,
            post,
            platform,
            item,
            format,
        } => {
            let project = project::find(&database, &slug).await?.ok_or_else(|| {
                photara::PhotaraError::Configuration(format!("project {slug:?} was not found"))
            })?;
            print_serialized(
                &layout::prepare_render_item(
                    &database,
                    &config,
                    &project,
                    &post,
                    platform.into(),
                    item.as_deref(),
                )
                .await?,
                format,
            )?;
        }
    }
    database.close().await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_progress_is_a_separate_human_readable_line() {
        let event = master::MasterProgress {
            stage: "Hashing layered masters",
            current: 14,
            total: 20,
            asset: "_SUH5235.PSB".into(),
        };
        assert_eq!(
            format_master_progress(&event),
            "Hashing layered masters  14/20  _SUH5235.PSB"
        );
        assert!(serde_json::to_string(&serde_json::json!({"ok": true})).is_ok());
    }
}
