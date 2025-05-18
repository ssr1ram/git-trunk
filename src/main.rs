use clap::{Parser, Subcommand};
use log::LevelFilter;
use env_logger::{Builder, Env};
use std::io::Write;

mod commands;

#[derive(Parser)]
#[command(author, version, about = "Git Trunk CLI for managing repository-wide documents", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(long, short = 'v', help = "Enable verbose output")]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes the git-trunk in the current repository
    Init(commands::init::InitArgs),
    /// Commits changes from .trunk to the main repository
    Commit(commands::commit::CommitArgs),
    /// Checkouts the trunk from refs/trunk/main into .trunk
    Checkout(commands::checkout::CheckoutArgs),
    /// Pushes the objects from refs/trunk/main to remote (default origin)
    Push(commands::push::PushArgs),
    /// Manages Git hooks for git-trunk
    Hooks(commands::hooks::HooksArgs),
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

    match cli.command {
        Commands::Init(args) => commands::init::run(&args, cli.verbose),
        Commands::Commit(args) => commands::commit::run(&args, cli.verbose),
        Commands::Checkout(args) => commands::checkout::run(&args, cli.verbose),
        Commands::Push(args) => commands::push::run(&args, cli.verbose),
        Commands::Hooks(args) => commands::hooks::run(&args, cli.verbose),
    }
}