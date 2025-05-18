use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(author, version, about = "Git Trunk CLI for managing repository-wide documents", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes the git-trunk in the current repository
    Init(commands::init::InitArgs),
    /// Syncs changes from .trunk to the main repository
    Sync(commands::sync::SyncArgs),
    /// Clones the trunk from refs/trunk/main into .trunk
    Clone(commands::clone::CloneArgs),
    /// Pushes the objects from refs/trunk/main to remote (default origin)
    Push(commands::push::PushArgs),
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => commands::init::run(&args),
        Commands::Sync(args) => commands::sync::run(&args),
        Commands::Clone(args) => commands::clone::run(&args),
        Commands::Push(args) => commands::push::run(&args),
    }
}