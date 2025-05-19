use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, exit, Stdio};
use clap::Parser;
use log::{debug, error, info};

#[derive(Parser, Debug)]
#[command(about = "Checkout the trunk from refs/trunk/main into .trunk")]
pub struct CheckoutArgs {
    #[arg(long, help = "Force cloning, overwriting existing .trunk directory")]
    force: bool,
}

pub fn run(args: &CheckoutArgs, verbose: bool) {
    // Step 1: Get repository root
    debug!("Step 1: Getting repository root");
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
    info!("✓ Step 1: Repository root found at {}", repo_root);

    // Step 2: Check if refs/trunk/main exists locally
    debug!("Step 2: Checking if refs/trunk/main exists locally");
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
        info!("✓ Step 2: refs/trunk/main found locally");
    } else {
        info!("✓ Step 2: refs/trunk/main not found locally");

        // Step 3: Check if refs/trunk/main exists on the remote
        debug!("Step 3: Checking if refs/trunk/main exists on remote (origin)");
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
        info!("✓ Step 3: refs/trunk/main found on remote (origin)");

        // Step 4: Fetch refs/trunk/main from origin
        debug!("Step 4: Fetching refs/trunk/main from origin");
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
        info!("✓ Step 4: Successfully fetched refs/trunk/main");
    }

    // Step 5: Verify refs/trunk/main exists locally after fetch
    debug!("Step 5: Verifying refs/trunk/main exists locally");
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
    info!("✓ Step 5: refs/trunk/main verified locally");

    // Step 6: Ensure .trunk is in .gitignore
    debug!("Step 6: Checking .gitignore for .trunk entry");
    let gitignore_path = Path::new(&repo_root).join(".gitignore");
    let mut gitignore_content = String::new();
    let mut gitignore_needs_update = false;

    if gitignore_path.exists() {
        let mut gitignore_file = File::open(&gitignore_path).unwrap_or_else(|e| {
            error!("Failed to read .gitignore: {}", e);
            exit(1);
        });
        gitignore_file
            .read_to_string(&mut gitignore_content)
            .expect("Failed to read .gitignore content");
        if !gitignore_content.lines().any(|line| line.trim() == ".trunk") {
            gitignore_needs_update = true;
        }
    } else {
        gitignore_needs_update = true;
    }

    if gitignore_needs_update {
        debug!("Step 6: Adding .trunk to .gitignore");
        let mut gitignore_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore_path)
            .unwrap_or_else(|e| {
                error!("Failed to open .gitignore: {}", e);
                exit(1);
            });
        writeln!(gitignore_file, ".trunk").expect("Failed to write .trunk to .gitignore");
        info!("✓ Step 6: Added .trunk to .gitignore");
    } else {
        debug!("Step 6: .trunk already in .gitignore");
        info!("= Step 6: .trunk already in .gitignore");
    }

    // Step 7: Check if .trunk exists
    debug!("Step 7: Checking if .trunk directory exists");
    let trunk_dir = Path::new(&repo_root).join(".trunk");
    if trunk_dir.exists() {
        let should_overwrite = if args.force {
            info!("Step 7: .trunk exists, --force specified, will overwrite");
            true
        } else {
            debug!("Step 7: .trunk directory exists");
            print!("🐘︖ Overwrite existing .trunk directory? [y/N]: ");
            io::stdout().flush().expect("Failed to flush stdout");

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read user input");
            let input = input.trim().to_lowercase();
            if input == "y" || input == "yes" {
                debug!("Step 7: User confirmed overwrite");
                true
            } else {
                info!("Step 7: Checkout aborted by user");
                exit(0);
            }
        };

        if should_overwrite {
            debug!("Step 7: Removing existing .trunk directory");
            fs::remove_dir_all(&trunk_dir).unwrap_or_else(|e| {
                error!("Failed to remove existing .trunk directory: {}", e);
                exit(1);
            });
            info!("✓ Step 7: Existing .trunk directory removed");
        }
    } else {
        debug!("∉ Step 7: .trunk directory does not exist");
    }

    // Step 8: Create .trunk directory
    debug!("Step 8: Creating .trunk directory");
    fs::create_dir(&trunk_dir).unwrap_or_else(|e| {
        error!("Failed to create .trunk directory: {}", e);
        exit(1);
    });
    info!("✓ Step 8: .trunk directory created");

    // Step 9: Initialize Git repository in .trunk
    debug!("Step 9: Initializing Git repository in .trunk");
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
    info!("✓ Step 9: Git repository initialized in .trunk");

    // Step 10: Fetch history from refs/trunk/main into a temporary ref
    debug!("Step 10: Fetching refs/trunk/main into .trunk temporary ref");
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
    info!("✓ Step 10: Successfully fetched refs/trunk/main into temporary ref");

    // Step 11: Get the fetched commit hash
    debug!("Step 11: Getting fetched commit hash");
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
    info!("✓ Step 11: Fetched commit hash: {}", commit_hash);

    // Step 12: Reset main branch to the fetched commit
    debug!("Step 12: Resetting .trunk main branch to fetched commit");
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
    info!("✓ Step 12: Main branch reset to commit {}", commit_hash);

    // Step 13: Update main branch ref
    debug!("Step 13: Updating refs/heads/main in .trunk");
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
    info!("✓ Step 13: refs/heads/main updated");

    // Step 14: Clean up temporary ref
    debug!("Step 14: Cleaning up temporary ref refs/temp/trunk");
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
    info!("✓ Step 14: Temporary ref cleaned up");

    info!("✅ Trunk checkout successfully");
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