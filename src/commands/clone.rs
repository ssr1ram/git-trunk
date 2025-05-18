use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, exit, Stdio};
use clap::Parser;
use log::{debug, error, info};

#[derive(Parser, Debug)]
#[command(about = "Clone the trunk from refs/trunk/main into .trunk")]
pub struct CloneArgs {
    #[arg(long, help = "Force cloning, overwriting existing .trunk directory")]
    force: bool,
}

pub fn run(args: &CloneArgs, verbose: bool) {
    // Step 1: Get repository root
    info!("Step 1: Getting repository root");
    let repo_root_output = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("--show-toplevel"),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to get git repository root: {}", e);
        exit(1);
    });
    let repo_root = String::from_utf8_lossy(&repo_root_output.stdout).trim().to_string();
    if repo_root.is_empty() {
        error!("Git repository root is empty. Ensure you are in a valid Git repository.");
        exit(1);
    }
    info!("Step 1: Repository root found at {}", repo_root);

    // Step 2: Check if refs/trunk/main exists locally
    info!("Step 2: Checking if refs/trunk/main exists locally");
    let local_ref_exists = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("--verify")
            .arg("refs/trunk/main")
            .current_dir(&repo_root),
        verbose,
    )
    .map(|output| output.status.success())
    .unwrap_or(false);
    if local_ref_exists {
        info!("Step 2: refs/trunk/main found locally");
    } else {
        info!("Step 2: refs/trunk/main not found locally");

        // Step 3: Check if refs/trunk/main exists on the remote
        info!("Step 3: Checking if refs/trunk/main exists on remote (origin)");
        let remote_ref_check = run_git_command(
            Command::new("git")
                .arg("ls-remote")
                .arg("origin")
                .arg("refs/trunk/main")
                .current_dir(&repo_root),
            verbose,
        )
        .unwrap_or_else(|e| {
            error!("Failed to check refs/trunk/main on remote: {}", e);
            exit(1);
        });
        if !remote_ref_check.status.success() || remote_ref_check.stdout.is_empty() {
            error!("refs/trunk/main does not exist in the repository or on the remote (origin). Ensure it was pushed with `git trunk push`.");
            exit(1);
        }
        info!("Step 3: refs/trunk/main found on remote (origin)");

        // Step 4: Fetch refs/trunk/main from origin
        info!("Step 4: Fetching refs/trunk/main from origin");
        let fetch_status = run_git_command(
            Command::new("git")
                .arg("fetch")
                .arg("origin")
                .arg("refs/trunk/main:refs/trunk/main")
                .current_dir(&repo_root),
            verbose,
        )
        .unwrap_or_else(|e| {
            error!("Failed to fetch refs/trunk/main from origin: {}", e);
            exit(1);
        })
        .status;
        if !fetch_status.success() {
            error!("Failed to fetch refs/trunk/main from origin. Check remote configuration and network connectivity.");
            exit(1);
        }
        info!("✅ Step 4: Successfully fetched refs/trunk/main");
    }

    // Step 5: Verify refs/trunk/main exists locally after fetch
    info!("Step 5: Verifying refs/trunk/main exists locally");
    let final_ref_check = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("--verify")
            .arg("refs/trunk/main")
            .current_dir(&repo_root),
        verbose,
    );
    if final_ref_check
        .map(|output| !output.status.success())
        .unwrap_or(true)
    {
        error!("refs/trunk/main is still missing after attempting to fetch. Ensure it was pushed to the remote.");
        exit(1);
    }
    info!("✅ Step 5: refs/trunk/main verified locally");

    // Step 6: Check if .trunk exists
    info!("Step 6: Checking if .trunk directory exists");
    let trunk_dir = Path::new(&repo_root).join(".trunk");
    if trunk_dir.exists() {
        let should_overwrite = if args.force {
            info!("Step 6: .trunk exists, --force specified, will overwrite");
            true
        } else {
            info!("Step 6: .trunk directory exists");
            print!("🐘 Overwrite existing .trunk directory? [y/N]: ");
            io::stdout().flush().expect("Failed to flush stdout");

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read user input");
            let input = input.trim().to_lowercase();
            if input == "y" || input == "yes" {
                info!("Step 6: User confirmed overwrite");
                true
            } else {
                info!("Step 6: Clone aborted by user");
                exit(0);
            }
        };

        if should_overwrite {
            info!("Step 6: Removing existing .trunk directory");
            fs::remove_dir_all(&trunk_dir).unwrap_or_else(|e| {
                error!("Failed to remove existing .trunk directory: {}", e);
                exit(1);
            });
            info!("✅ Step 6: Existing .trunk directory removed");
        }
    } else {
        info!("Step 6: .trunk directory does not exist");
    }

    // Step 7: Create .trunk directory
    info!("Step 7: Creating .trunk directory");
    fs::create_dir(&trunk_dir).unwrap_or_else(|e| {
        error!("Failed to create .trunk directory: {}", e);
        exit(1);
    });
    info!("✅ Step 7: .trunk directory created");

    // Step 8: Initialize Git repository in .trunk
    info!("Step 8: Initializing Git repository in .trunk");
    let init_status = run_git_command(
        Command::new("git")
            .arg("init")
            .current_dir(&trunk_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to run git init in .trunk: {}", e);
        exit(1);
    })
    .status;
    if !init_status.success() {
        error!("git init failed in .trunk");
        exit(1);
    }
    info!("✅ Step 8: Git repository initialized in .trunk");

    // Step 9: Fetch history from refs/trunk/main into a temporary ref
    info!("Step 9: Fetching refs/trunk/main into .trunk temporary ref");
    let fetch_status = run_git_command(
        Command::new("git")
            .arg("fetch")
            .arg(&repo_root)
            .arg("refs/trunk/main:refs/temp/trunk")
            .current_dir(&trunk_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to fetch refs/trunk/main into .trunk: {}", e);
        exit(1);
    })
    .status;
    if !fetch_status.success() {
        error!("git fetch failed for refs/trunk/main");
        exit(1);
    }
    info!("✅ Step 9: Successfully fetched refs/trunk/main into temporary ref");

    // Step 10: Get the fetched commit hash
    info!("Step 10: Getting fetched commit hash");
    let commit_hash_output = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("refs/temp/trunk")
            .current_dir(&trunk_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to get fetched commit hash: {}", e);
        exit(1);
    });
    if !commit_hash_output.status.success() {
        error!("refs/temp/trunk not found after fetch");
        exit(1);
    }
    let commit_hash = String::from_utf8_lossy(&commit_hash_output.stdout).trim().to_string();
    info!("Step 10: Fetched commit hash: {}", commit_hash);

    // Step 11: Reset main branch to the fetched commit
    info!("Step 11: Resetting .trunk main branch to fetched commit");
    let reset_status = run_git_command(
        Command::new("git")
            .arg("reset")
            .arg("--hard")
            .arg(&commit_hash)
            .current_dir(&trunk_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to reset .trunk to fetched commit: {}", e);
        exit(1);
    })
    .status;
    if !reset_status.success() {
        error!("git reset failed in .trunk");
        exit(1);
    }
    info!("✅ Step 11: Main branch reset to commit {}", commit_hash);

    // Step 12: Update main branch ref
    info!("Step 12: Updating refs/heads/main in .trunk");
    let update_ref_status = run_git_command(
        Command::new("git")
            .arg("update-ref")
            .arg("refs/heads/main")
            .arg(&commit_hash)
            .current_dir(&trunk_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to update refs/heads/main in .trunk: {}", e);
        exit(1);
    })
    .status;
    if !update_ref_status.success() {
        error!("git update-ref failed for refs/heads/main");
        exit(1);
    }
    info!("✅ Step 12: refs/heads/main updated");

    // Step 13: Clean up temporary ref
    info!("Step 13: Cleaning up temporary ref refs/temp/trunk");
    if let Err(e) = run_git_command(
        Command::new("git")
            .arg("update-ref")
            .arg("-d")
            .arg("refs/temp/trunk")
            .current_dir(&trunk_dir),
        verbose,
    ) {
        error!("Warning: Failed to delete temporary ref refs/temp/trunk: {}", e);
        // Non-critical, continue
    }
    info!("✅ Step 13: Temporary ref cleaned up");

    info!("✅ Trunk cloned successfully");
}

// Helper function to run Git commands and handle output
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