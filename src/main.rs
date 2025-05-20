use clap::{Parser, Subcommand};
use log::LevelFilter;
use env_logger::{Builder, Env};
use std::io::Write;

mod commands;
mod utils; // Added utils module

#[derive(Parser)]
#[command(author, version, about = "Git Trunk CLI for managing repository-wide documents", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(
        long, 
        short = 'v', 
        help = "Enable verbose output",
        global = true
    )]
    verbose: bool,

    #[arg(
        long,
        short = 'r',
        help = "Specify the remote repository",
        default_value = "origin",
        global = true
    )]
    remote: String,

    #[arg(
        long,
        short = 's',
        help = "Specify the trunk store name (e.g., main, blog, issues)",
        default_value = "main",
        global = true
    )]
    store: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes the git-trunk store in the current repository
    Init(commands::init::InitArgs),
    /// Commits changes from .trunk/<store> to the main repository's refs/trunk/<store>
    Commit(commands::commit::CommitArgs),
    /// Checkouts the trunk store from refs/trunk/<store> into .trunk/<store>
    Checkout(commands::checkout::CheckoutArgs),
    /// Pushes the objects from refs/trunk/<store> to the specified remote
    Push(commands::push::PushArgs),
    /// Manages Git hooks for a git-trunk store
    Hooks(commands::hooks::HooksArgs),
    /// Removes all traces of .trunk/<store> from the main repository's working directory
    Stegano(commands::stegano::SteganoArgs),
    /// Removes all traces of a git-trunk store, including .trunk/<store> and refs/trunk/<store> locally and remotely
    Delete(commands::delete::DeleteArgs),
    /// Displays information about the git-trunk setup and stores
    Info(commands::info::InfoArgs),
}

fn init_logger(verbose: bool) {
    let env = Env::default().filter_or("RUST_LOG", if verbose { "debug" } else { "info" });
    Builder::from_env(env)
        .format(|buf, record| {
            let level_style = match record.level() {
                log::Level::Error => "\x1B[31m❌\x1B[0m", // Red ❌ for errors
                log::Level::Info => "🐘",                // 🐘 for info
                log::Level::Debug => "🐘",               // 🐘 for debug
                _ => "",                                  // Others (not used)
            };
            writeln!(buf, "{} {}", level_style, record.args())
        })
        .filter(None, if verbose { LevelFilter::Debug } else { LevelFilter::Info })
        .init();
}

fn main() {
    let cli = Cli::parse();
    init_logger(cli.verbose);

    let remote_name = &cli.remote;
    let store_name = &cli.store;

    match cli.command {
        Commands::Init(args) => commands::init::run(&args, remote_name, store_name, cli.verbose),
        Commands::Commit(args) => commands::commit::run(&args, remote_name, store_name, cli.verbose),
        Commands::Checkout(args) => commands::checkout::run(&args, remote_name, store_name, cli.verbose),
        Commands::Push(args) => commands::push::run(&args, remote_name, store_name, cli.verbose),
        Commands::Hooks(args) => commands::hooks::run(&args, remote_name, store_name, cli.verbose),
        Commands::Stegano(args) => commands::stegano::run(&args, remote_name, store_name, cli.verbose),
        Commands::Delete(args) => commands::delete::run(&args, remote_name, store_name, cli.verbose),
        Commands::Info(args) => commands::info::run(&args, remote_name, store_name, cli.verbose),
    }
}