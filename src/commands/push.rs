use clap::Parser;
use std::io;
use std::process::{Command, exit, Stdio};
use log::{debug, error, info};

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

pub fn run(args: &PushArgs, verbose: bool) {
    // Step 1: Verify that refs/trunk/main exists
    info!("Step 1: Checking if refs/trunk/main exists locally");
    let show_ref = run_git_command(
        Command::new("git")
            .args(["show-ref", "--quiet", "refs/trunk/main"]),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to check refs/trunk/main: {}", e);
        exit(1);
    });

    if !show_ref.status.success() {
        error!("refs/trunk/main does not exist in the repository");
        exit(1);
    }
    info!("✅ Step 1: refs/trunk/main found locally");

    // Step 2: Push refs/trunk/main to the remote
    info!("Step 2: Pushing refs/trunk/main to remote '{}'", args.remote);
    let push = run_git_command(
        Command::new("git")
            .args([
                "push",
                &args.remote,
                "refs/trunk/main:refs/trunk/main",
            ]),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to execute git push: {}", e);
        exit(1);
    })
    .status;

    if !push.success() {
        error!("Failed to push refs/trunk/main to remote '{}'", args.remote);
        exit(1);
    }

    info!("✅ Step 2: Successfully pushed refs/trunk/main to {}", args.remote);
}

fn run_git_command(command: &mut Command, verbose: bool) -> io::Result<std::process::Output> {
    // Check if git is available
    let git_check = Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if git_check.is_err() || !git_check.unwrap().success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Git executable not found or failed to execute",
        ));
    }

    // Always capture stdout, suppress stderr in non-verbose mode
    if !verbose {
        command.stderr(Stdio::null());
    }
    let output = command.output()?;
    if verbose {
        if !output.stdout.is_empty() {
            debug!("Git stdout: {}", String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            debug!("Git stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
    }
    Ok(output)
}