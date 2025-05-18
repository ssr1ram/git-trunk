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
    // Step 1: Verify that refs/trunk/main exists
    println!("\u{1F418} Step 1: Checking if refs/trunk/main exists locally");
    let show_ref = Command::new("git")
        .args(["show-ref", "--quiet", "refs/trunk/main"])
        .status()
        .unwrap_or_else(|e| {
            eprintln!("\u{1F418} Error: Failed to check refs/trunk/main: {}", e);
            std::process::exit(1);
        });

    if !show_ref.success() {
        eprintln!("\u{1F418} Error: refs/trunk/main does not exist in the repository");
        std::process::exit(1);
    }
    println!("\u{1F418} Step 1: refs/trunk/main found locally");

    // Step 2: Push refs/trunk/main to the remote
    println!("\u{1F418} Step 2: Pushing refs/trunk/main to remote '{}'", args.remote);
    let push = Command::new("git")
        .args([
            "push",
            &args.remote,
            "refs/trunk/main:refs/trunk/main",
        ])
        .status()
        .unwrap_or_else(|e| {
            eprintln!("\u{1F418} Error: Failed to execute git push: {}", e);
            std::process::exit(1);
        });

    if !push.success() {
        eprintln!(
            "\u{1F418} Error: Failed to push refs/trunk/main to remote '{}'",
            args.remote
        );
        std::process::exit(1);
    }

    println!("\u{1F418} Step 2: Successfully pushed refs/trunk/main to {}", args.remote);
}