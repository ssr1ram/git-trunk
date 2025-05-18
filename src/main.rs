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
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => commands::init::init(),
    }
}