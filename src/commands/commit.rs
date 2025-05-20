use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, exit};
use clap::Parser;
use log::{debug, error, info};
use crate::utils::run_git_command;

#[derive(Parser, Debug)]
#[command(about = "Commit changes from .trunk/<store> to the main repository's refs/trunk/<store>")]
pub struct CommitArgs {
    #[arg(long, help = "Skip interactive prompts and stage all changes")]
    force: bool,
    #[arg(short = 'm', long, help = "Commit message")]
    message: Option<String>,
}

pub fn run(args: &CommitArgs, _remote_name: &str, store_name: &str, verbose: bool) {
    // Step 1: Get repository root
    debug!("➡️ Step 1: Getting repository root");
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
    let repo_root = String::from_utf8_lossy(&repo_root_output.stdout).trim().to_string();
    if repo_root.is_empty() {
        error!("❌ Git repository root is empty. Ensure you are in a valid Git repository.");
        exit(1);
    }
    info!("✓ Step 1: Repository root found at {}", repo_root);

    let store_dir_path_str = format!(".trunk/{}", store_name);
    let trunk_store_dir = Path::new(&repo_root).join(&store_dir_path_str);
    let trunk_ref_name = format!("refs/trunk/{}", store_name);

    // Step 2: Check if .trunk/<store_name> exists
    debug!("➡️ Step 2: Checking for {} directory", store_dir_path_str);
    if !trunk_store_dir.exists() {
        error!("❌ {} directory not found for store '{}'. Run `git trunk init --store {}` first.", store_dir_path_str, store_name, store_name);
        exit(1);
    }
    info!("✓ Step 2: {} directory found", store_dir_path_str);

    // Step 3: Check if .trunk/<store_name> has files to be staged
    debug!("➡️ Step 3: Checking for changes in {}", store_dir_path_str);
    let status_output = run_git_command(
        Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&trunk_store_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("❌ Failed to run git status in {}: {}", store_dir_path_str, e);
        exit(1);
    });

    let status = String::from_utf8_lossy(&status_output.stdout);
    if status.is_empty() {
        info!("= Step 3: No changes to stage in {}", store_dir_path_str);
    } else {
        // Step 4: Ask user to stage all files (unless --force)
        let should_stage = if args.force {
            debug!("🚀 Step 4: --force specified, staging all changes in {}", store_dir_path_str);
            true
        } else {
            info!("≠ Step 4: Changes detected in {}:\n{}", store_dir_path_str, status);
            print!("🐘︖ Stage all files for store '{}'? [y/N]: ", store_name);
            io::stdout().flush().expect("Failed to flush stdout");

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read user input");
            let input = input.trim().to_lowercase();
            if input == "y" || input == "yes" {
                debug!("👍 Step 4: User confirmed staging for store '{}'", store_name);
                true
            } else {
                info!("🚫 Step 4: Commit for store '{}' aborted by user", store_name);
                exit(0);
            }
        };

        if should_stage {
            // Stage all files
            debug!("➕ Step 4: Staging all files in {}", store_dir_path_str);
            let stage_status = run_git_command(
                Command::new("git")
                    .arg("add")
                    .arg("-A")
                    .current_dir(&trunk_store_dir),
                verbose,
            )
            .unwrap_or_else(|e| {
                error!("❌ Failed to run git add in {}: {}", store_dir_path_str, e);
                exit(1);
            })
            .status;
            if !stage_status.success() {
                error!("❌ git add failed in {}", store_dir_path_str);
                exit(1);
            }
            info!("✓ Step 4: Files staged in {}", store_dir_path_str);

            // Step 5: Commit staged files
            debug!("💾 Step 5: Committing staged changes for store '{}'", store_name);
            let commit_message = args.message.clone().unwrap_or_else(|| format!("Commit trunk changes for store '{}'", store_name));
            let commit_status = run_git_command(
                Command::new("git")
                    .arg("commit")
                    .arg("-m")
                    .arg(&commit_message)
                    .current_dir(&trunk_store_dir),
                verbose,
            )
            .unwrap_or_else(|e| {
                error!("❌ Failed to run git commit in {}: {}", store_dir_path_str, e);
                exit(1);
            })
            .status;

            if !commit_status.success() {
                // This can happen if git add -A results in no actual changes to commit (e.g., only .gitignored files changed status)
                // or if there were no staged changes after all.
                info!("= Step 5: No changes to commit in {} (or commit failed)", store_dir_path_str);
            } else {
                info!("✓ Step 5: Changes committed in {}", store_dir_path_str);
            }
        }
    }

    // Step 6: Get the latest commit hash from .trunk/<store_name>
    debug!("🔑 Step 6: Getting latest commit hash from {}'s main branch", store_dir_path_str);
    let commit_hash_output = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("main") // Assumes 'main' is the branch in the store's repo
            .current_dir(&trunk_store_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("❌ Failed to get {} main commit hash: {}", store_dir_path_str, e);
        exit(1);
    });
    let commit_hash = String::from_utf8_lossy(&commit_hash_output.stdout).trim().to_string();
    if commit_hash.is_empty() {
        error!("❌ Failed to get commit hash from {}. It might be empty or not have commits on 'main'.", store_dir_path_str);
        exit(1);
    }
    debug!("🔑 Step 6: Commit hash for store '{}': {}", store_name, commit_hash);

    // Step 7: Fetch objects from .trunk/<store_name> to main repo
    let temp_branch_name = format!("trunk-temp-{}", store_name);
    debug!("📥 Step 7: Fetching objects from {} into temporary branch '{}' in main repository", store_dir_path_str, temp_branch_name);
    let fetch_status = run_git_command(
        Command::new("git")
            .arg("-C")
            .arg(&repo_root)
            .arg("fetch")
            .arg(&trunk_store_dir)
            .arg(format!("main:{}", temp_branch_name)), // Fetch main from store repo to temp branch
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("❌ Failed to fetch objects from {}: {}", store_dir_path_str, e);
        exit(1);
    })
    .status;
    if !fetch_status.success() {
        error!("❌ git fetch failed from {}", store_dir_path_str);
        exit(1);
    }
    info!("✓ Step 7: Objects fetched from store '{}'", store_name);

    // Step 8: Update refs/trunk/<store_name>
    debug!("➡️ Step 8: Checking if {} exists", trunk_ref_name);
    let ref_exists = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("--verify")
            .arg(&trunk_ref_name)
            .current_dir(&repo_root),
        verbose,
    )
    .map(|output| output.status.success())
    .unwrap_or(false);

    debug!("🔄 Step 8: Updating {} to commit {}", trunk_ref_name, commit_hash);
    let update_ref_status = run_git_command(
        Command::new("git")
            .arg("update-ref")
            .arg(&trunk_ref_name)
            .arg(&commit_hash)
            .current_dir(&repo_root),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("❌ Failed to update {}: {}", trunk_ref_name, e);
        exit(1);
    })
    .status;
    if !update_ref_status.success() {
        error!("❌ git update-ref failed for {}", trunk_ref_name);
        exit(1);
    }

    // Step 9: Clean up temporary branch
    debug!("🧹 Step 9: Cleaning up temporary branch {}", temp_branch_name);
    let cleanup_status = run_git_command(
        Command::new("git")
            .arg("branch")
            .arg("-D")
            .arg(&temp_branch_name)
            .current_dir(&repo_root),
        verbose,
    );
    // Log warning if cleanup fails, but don't exit
    if cleanup_status.is_err() || (cleanup_status.is_ok() && !cleanup_status.as_ref().unwrap().status.success()){
        error!("⚠️ Warning: Failed to delete temporary branch {}. You may need to delete it manually: git branch -D {}", temp_branch_name, temp_branch_name);
    }


    if ref_exists {
        info!("✓ Step 8 & 9: Updated {} to commit {}", trunk_ref_name, commit_hash);
    } else {
        info!("✓ Step 8 & 9: Created {} at commit {}", trunk_ref_name, commit_hash);
    }

    info!("✅ Trunk store '{}' committed successfully to {}", store_name, trunk_ref_name);
}