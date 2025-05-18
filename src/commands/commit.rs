use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, exit, Stdio};
use clap::Parser;
use log::{debug, error, info};

#[derive(Parser, Debug)]
#[command(about = "Commit changes from .trunk to the main repository")]
pub struct CommitArgs {
    #[arg(long, help = "Skip interactive prompts and stage all changes")]
    force: bool,
}

pub fn run(args: &CommitArgs, verbose: bool) {
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

    // Step 2: Check if .trunk exists
    debug!("Step 2: Checking for .trunk directory");
    let trunk_dir = Path::new(&repo_root).join(".trunk");
    if !trunk_dir.exists() {
        error!(".trunk directory not found. Run `git trunk init` first.");
        exit(1);
    }
    info!("✓ Step 2: .trunk directory found");

    // Step 3: Check if .trunk has files to be staged
    debug!("Step 3: Checking for changes in .trunk");
    let status_output = run_git_command(
        Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&trunk_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to run git status in .trunk: {}", e);
        exit(1);
    });

    let status = String::from_utf8_lossy(&status_output.stdout);
    if status.is_empty() {
        info!("= Step 3: No changes to stage in .trunk");
    } else {
        // Step 4: Ask user to stage all files (unless --force)
        let should_stage = if args.force {
            debug!("Step 4: --force specified, staging all changes");
            true
        } else {
            info!("≠ Step 4: Changes detected in .trunk:\n{}", status);
            print!("🐘︖ Stage all files? [y/N]: ");
            io::stdout().flush().expect("Failed to flush stdout");

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read user input");
            let input = input.trim().to_lowercase();
            if input == "y" || input == "yes" {
                debug!("Step 4: User confirmed staging");
                true
            } else {
                info!("Step 4: Commit aborted by user");
                exit(0);
            }
        };

        if should_stage {
            // Stage all files
            debug!("Step 4: Staging all files in .trunk");
            let stage_status = run_git_command(
                Command::new("git")
                    .arg("add")
                    .arg("-A")
                    .current_dir(&trunk_dir),
                verbose,
            )
            .unwrap_or_else(|e| {
                error!("Failed to run git add in .trunk: {}", e);
                exit(1);
            })
            .status;
            if !stage_status.success() {
                error!("git add failed in .trunk");
                exit(1);
            }
            info!("✓ Step 4: Files staged");

            // Step 5: Commit staged files
            debug!("Step 5: Committing staged changes");
            let commit_status = run_git_command(
                Command::new("git")
                    .arg("commit")
                    .arg("-m")
                    .arg("Commit trunk changes")
                    .current_dir(&trunk_dir),
                verbose,
            )
            .unwrap_or_else(|e| {
                error!("Failed to run git commit in .trunk: {}", e);
                exit(1);
            })
            .status;

            if !commit_status.success() {
                info!("= Step 5: No changes to commit in .trunk");
            } else {
                info!("✓ Step 5: Changes committed");
            }
        }
    }

    // Step 6: Get the latest commit hash from .trunk
    debug!("Step 6: Getting latest commit hash from .trunk");
    let commit_hash_output = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("main")
            .current_dir(&trunk_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to get .trunk main commit hash: {}", e);
        exit(1);
    });
    let commit_hash = String::from_utf8_lossy(&commit_hash_output.stdout).trim().to_string();
    debug!("Step 6: Commit hash: {}", commit_hash);

    // Step 7: Fetch objects from .trunk to main repo
    debug!("Step 7: Fetching objects from .trunk to main repository");
    let fetch_status = run_git_command(
        Command::new("git")
            .arg("-C")
            .arg(&repo_root)
            .arg("fetch")
            .arg(&trunk_dir)
            .arg("main:trunk-temp"),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to fetch objects from .trunk: {}", e);
        exit(1);
    })
    .status;
    if !fetch_status.success() {
        error!("git fetch failed from .trunk");
        exit(1);
    }
    info!("✓ Step 7: Objects fetched");

    // Step 8: Update refs/trunk/main
    debug!("Step 8: Checking if refs/trunk/main exists");
    let ref_exists = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("--verify")
            .arg("refs/trunk/main")
            .current_dir(&repo_root),
        verbose,
    )
    .map(|output| output.status.success())
    .unwrap_or(false);

    debug!("Step 8: Updating refs/trunk/main");
    let update_ref_status = run_git_command(
        Command::new("git")
            .arg("update-ref")
            .arg("refs/trunk/main")
            .arg(&commit_hash)
            .current_dir(&repo_root),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to update refs/trunk/main: {}", e);
        exit(1);
    })
    .status;
    if !update_ref_status.success() {
        error!("git update-ref failed for refs/trunk/main");
        exit(1);
    }

    // Step 9: Clean up temporary branch
    debug!("Step 9: Cleaning up temporary branch trunk-temp");
    let cleanup_status = run_git_command(
        Command::new("git")
            .arg("branch")
            .arg("-D")
            .arg("trunk-temp")
            .current_dir(&repo_root),
        verbose,
    );
    if let Err(e) = cleanup_status {
        error!("Warning: Failed to delete temporary branch trunk-temp: {}", e);
        // Non-critical, continue
    }

    if ref_exists {
        info!("✓ Step 8: Updated refs/trunk/main to commit {}", commit_hash);
    } else {
        info!("✓ Step 8: Created refs/trunk/main at commit {}", commit_hash);
    }

    info!("✅ Trunk commited successfully");
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