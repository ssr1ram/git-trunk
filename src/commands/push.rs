use clap::Parser;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(about = "Push refs/trunk/main to the specified remote")]
pub struct PushArgs {
    #[arg(
        long,
        default_value = "origin",
        help = "Remote to push refs/trunk/main to"
    )]
    remote: String,
}

#[allow(dead_code)]
pub fn run(args: &PushArgs) {
    // Verify that refs/trunk/main exists
    let show_ref = Command::new("git")
        .args(["show-ref", "--quiet", "refs/trunk/main"])
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Error checking refs/trunk/main: {}", e);
            std::process::exit(1);
        });

    if !show_ref.success() {
        eprintln!("Error: refs/trunk/main does not exist in the repository");
        std::process::exit(1);
    }

    // Push refs/trunk/main to the remote
    let push = Command::new("git")
        .args([
            "push",
            &args.remote,
            "refs/trunk/main:refs/trunk/main",
        ])
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Error executing git push: {}", e);
            std::process::exit(1);
        });

    if !push.success() {
        eprintln!(
            "Error: Failed to push refs/trunk/main to remote '{}'",
            args.remote
        );
        std::process::exit(1);
    }

    println!("Successfully pushed refs/trunk/main to {}", args.remote);
}