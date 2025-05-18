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
    Init,
    /// Syncs changes from .trunk to the main repository
    Sync,
    /// Clones the trunk from refs/trunk/main into .trunk
    Clone,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => commands::init::init(),
        Commands::Sync => commands::sync::sync(),
        Commands::Clone => commands::clone::clone(),
    }
}