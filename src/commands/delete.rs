use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, exit};
use clap::Parser;
use log::{debug, error, info};
use crate::utils::run_git_command;

#[derive(Parser, Debug)]
#[command(about = "Remove all traces of a git-trunk store, including .trunk/<store> and refs/trunk/<store> locally and remotely")]
pub struct DeleteArgs {}

pub fn run(_args: &DeleteArgs, remote_name: &str, store_name: &str, verbose: bool) {
    let trunk_ref_name = format!("refs/trunk/{}", store_name);
    let store_dir_relative_path = format!(".trunk/{}", store_name);

    // Step 1: Prompt user for confirmation
    debug!("➡️ Step 1: Prompting user for confirmation to delete store '{}'", store_name);
    print!("🐘︖ This will delete the local directory '{}', the local ref '{}', and the remote ref '{}' on remote '{}'. This operation is irreversible. Continue? [y/N]: ", store_dir_relative_path, trunk_ref_name, trunk_ref_name, remote_name);
    io::stdout().flush().expect("Failed to flush stdout");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read user input");
    let input = input.trim().to_lowercase();
    if input != "y" && input != "yes" {
        info!("🚫 Delete operation for store '{}' aborted by user", store_name);
        exit(0);
    }
    info!("✓ Step 1: User confirmed deletion for store '{}'", store_name);

    // Step 2: Check if we are in a Git repository
    debug!("➡️ Step 2: Checking if inside a Git repository");
    let git_check_output = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree"),
        verbose,
    );
    if git_check_output.map(|output| !output.status.success()).unwrap_or(true) {
        error!("❌ delete can only be invoked inside a git repo");
        exit(1);
    }
    info!("✓ Step 2: Confirmed inside a Git repository");

    // Step 3: Get repository root
    debug!("➡️ Step 3: Getting repository root");
    let repo_root_output = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("--show-toplevel"),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("❌ Failed to get git repository root: {}", e);
        exit(1);
    });
    let repo_root_str = String::from_utf8_lossy(&repo_root_output.stdout).trim().to_string();
    if repo_root_str.is_empty() {
        error!("❌ Git repository root is empty. Ensure you are in a valid Git repository.");
        exit(1);
    }
    let repo_root = Path::new(&repo_root_str);
    info!("✓ Step 3: Repository root found at {}", repo_root.display());

    // Step 4: Remove .trunk/<store_name> directory
    let trunk_store_dir = repo_root.join(&store_dir_relative_path);
    debug!("➡️ Step 4: Checking for {} directory", store_dir_relative_path);
    if trunk_store_dir.exists() {
        debug!("🗑️ Step 4: Removing {} directory for store '{}'", store_dir_relative_path, store_name);
        fs::remove_dir_all(&trunk_store_dir).unwrap_or_else(|e| {
            error!("❌ Failed to remove {} directory: {}", store_dir_relative_path, e);
            // Do not exit here, try to remove refs as well
        });
        info!("✓ Step 4: {} directory removed for store '{}'", store_dir_relative_path, store_name);
    } else {
        debug!("🚫 Step 4: No {} directory found for store '{}'", store_dir_relative_path, store_name);
        info!("= Step 4: No {} directory to remove for store '{}'", store_dir_relative_path, store_name);
    }
    
    // Step 4b: Check if .trunk parent directory is empty, if so, remove it
    let parent_trunk_dir = repo_root.join(".trunk");
    if parent_trunk_dir.exists() {
        match fs::read_dir(&parent_trunk_dir) {
            Ok(mut entries) => {
                if entries.next().is_none() { // Directory is empty
                    debug!("🗑️ Step 4b: .trunk directory is empty, removing it.");
                    if let Err(e) = fs::remove_dir(&parent_trunk_dir) {
                        error!("⚠️ Warning: Failed to remove empty .trunk directory at {}: {}", parent_trunk_dir.display(), e);
                    } else {
                        info!("✓ Step 4b: Empty .trunk directory removed.");
                    }
                } else {
                    debug!("ℹ️ Step 4b: .trunk directory is not empty, retaining it.");
                }
            },
            Err(e) => {
                error!("⚠️ Warning: Could not read .trunk directory contents at {}: {}", parent_trunk_dir.display(), e);
            }
        }
    }


    // Step 5: Delete local refs/trunk/<store_name>
    debug!("➡️ Step 5: Checking for local ref {}", trunk_ref_name);
    let local_ref_exists = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("--verify")
            .arg(&trunk_ref_name)
            .current_dir(repo_root),
        verbose,
    )
    .map(|output| output.status.success())
    .unwrap_or(false);

    if local_ref_exists {
        debug!("🗑️ Step 5: Deleting local ref {}", trunk_ref_name);
        let delete_status = run_git_command(
            Command::new("git")
                .arg("update-ref")
                .arg("-d")
                .arg(&trunk_ref_name)
                .current_dir(repo_root),
            verbose,
        )
        .unwrap_or_else(|e| {
            error!("❌ Failed to delete local ref {}: {}", trunk_ref_name, e);
            exit(1); // Critical if ref deletion fails but we said we would
        });
        if !delete_status.status.success() {
            error!("❌ Failed to delete local ref {}. It might not exist or another error occurred.", trunk_ref_name);
            // Continue to try remote deletion
        } else {
            info!("✓ Step 5: Local ref {} deleted", trunk_ref_name);
        }
    } else {
        debug!("🚫 Step 5: No local ref {} found for store '{}'", trunk_ref_name, store_name);
        info!("= Step 5: No local ref {} to delete for store '{}'", trunk_ref_name, store_name);
    }

    // Step 6: Delete remote refs/trunk/<store_name>
    debug!("➡️ Step 6: Checking for remote ref {} on remote '{}'", trunk_ref_name, remote_name);
    let remote_ref_check = run_git_command(
        Command::new("git")
            .arg("ls-remote")
            .arg(remote_name)
            .arg(&trunk_ref_name)
            .current_dir(repo_root),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("❌ Failed to check {} on remote '{}': {}", trunk_ref_name, remote_name, e);
        exit(1); // Critical if we can't check before trying to delete
    });

    if !remote_ref_check.stdout.is_empty() {
        debug!("🗑️ Step 6: Deleting remote ref {} on remote '{}'", trunk_ref_name, remote_name);
        let push_delete_status = run_git_command(
            Command::new("git")
                .arg("push")
                .arg(remote_name)
                .arg(format!(":{}", trunk_ref_name)) // Delete refspec
                .current_dir(repo_root),
            verbose,
        )
        .unwrap_or_else(|e| {
            error!("❌ Failed to delete remote ref {}: {}", trunk_ref_name, e);
            exit(1); // Critical
        });
        if !push_delete_status.status.success() {
            error!("❌ Failed to delete remote ref {} on remote '{}'. Check remote configuration and permissions.", trunk_ref_name, remote_name);
            // Don't exit, just report error
        } else {
             info!("✓ Step 6: Remote ref {} deleted on remote '{}'", trunk_ref_name, remote_name);
        }
    } else {
        debug!("🚫 Step 6: No remote ref {} found on remote '{}' for store '{}'", trunk_ref_name, remote_name, store_name);
        info!("= Step 6: No remote ref {} to delete on remote '{}' for store '{}'", trunk_ref_name, remote_name, store_name);
    }
    // Note: .gitignore entry for ".trunk" is not removed, as other stores might exist.

    info!("✅ Delete for store '{}' completed. Local directory (if existed), local ref (if existed), and remote ref (if existed) have been targeted for removal.", store_name);
}