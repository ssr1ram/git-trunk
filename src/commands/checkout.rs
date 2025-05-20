use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, exit};
use clap::Parser;
use log::{debug, error, info};
use crate::utils::run_git_command;

#[derive(Parser, Debug)]
#[command(about = "Checkout a trunk store from refs/trunk/<store> into .trunk/<store>")]
pub struct CheckoutArgs {
    #[arg(long, help = "Force cloning, overwriting existing .trunk/<store> directory")]
    force: bool,
}

pub fn run(args: &CheckoutArgs, remote_name: &str, store_name: &str, verbose: bool) {
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
    let repo_root_str = String::from_utf8_lossy(&repo_root_output.stdout).trim().to_string();
    if repo_root_str.is_empty() {
        error!("❌ Git repository root is empty. Ensure you are in a valid Git repository.");
        exit(1);
    }
    let repo_root = Path::new(&repo_root_str);
    info!("✓ Step 1: Repository root found at {}", repo_root.display());

    let trunk_ref_name = format!("refs/trunk/{}", store_name);
    let store_dir_relative_path = format!(".trunk/{}", store_name);
    let trunk_store_dir = repo_root.join(&store_dir_relative_path);

    // Step 2: Check if refs/trunk/<store_name> exists locally
    debug!("➡️ Step 2: Checking if {} exists locally", trunk_ref_name);
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
        info!("✓ Step 2: {} found locally", trunk_ref_name);
    } else {
        info!("🚫 Step 2: {} not found locally for store '{}'", trunk_ref_name, store_name);

        // Step 3: Check if refs/trunk/<store_name> exists on the remote
        debug!("➡️ Step 3: Checking if {} exists on remote '{}'", trunk_ref_name, remote_name);
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
            exit(1);
        });
        if !remote_ref_check.status.success() || remote_ref_check.stdout.is_empty() {
            error!("❌ {} for store '{}' does not exist on the remote '{}'. Ensure it was pushed with `git trunk push --store {} --remote {}`.", trunk_ref_name, store_name, remote_name, store_name, remote_name);
            exit(1);
        }
        info!("✓ Step 3: {} found on remote '{}'", trunk_ref_name, remote_name);

        // Step 4: Fetch refs/trunk/<store_name> from remote
        debug!("📥 Step 4: Fetching {} from remote '{}'", trunk_ref_name, remote_name);
        let fetch_refspec = format!("{}:{}", trunk_ref_name, trunk_ref_name);
        let fetch_status = run_git_command(
            Command::new("git")
                .arg("fetch")
                .arg(remote_name)
                .arg(&fetch_refspec)
                .current_dir(repo_root),
            verbose,
        )
        .unwrap_or_else(|e| {
            error!("❌ Failed to fetch {} from remote '{}': {}", trunk_ref_name, remote_name, e);
            exit(1);
        })
        .status;
        if !fetch_status.success() {
            error!("❌ Failed to fetch {} from remote '{}'. Check remote configuration and network connectivity.", trunk_ref_name, remote_name);
            exit(1);
        }
        info!("✓ Step 4: Successfully fetched {} from remote '{}'", trunk_ref_name, remote_name);
    }

    // Step 5: Verify refs/trunk/<store_name> exists locally after fetch attempt
    debug!("🔍 Step 5: Verifying {} exists locally for store '{}'", trunk_ref_name, store_name);
    let final_ref_check = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("--verify")
            .arg(&trunk_ref_name)
            .current_dir(repo_root),
        verbose,
    );
    if final_ref_check.map(|output| !output.status.success()).unwrap_or(true) {
        error!("❌ {} for store '{}' is still missing after attempting to fetch. Ensure it was pushed to the remote.", trunk_ref_name, store_name);
        exit(1);
    }
    info!("✓ Step 5: {} verified locally for store '{}'", trunk_ref_name, store_name);

    // Step 6: Ensure .trunk is in .gitignore (parent directory)
    debug!("➡️ Step 6: Checking .gitignore for .trunk entry");
    let gitignore_path = repo_root.join(".gitignore");
    // This logic is identical to init, could be refactored if desired
    let mut gitignore_content = String::new();
    let mut gitignore_needs_update = false;
    if gitignore_path.exists() {
        let mut gitignore_file = File::open(&gitignore_path).unwrap_or_else(|e| {
            error!("❌ Failed to read .gitignore: {}", e); exit(1);
        });
        gitignore_file.read_to_string(&mut gitignore_content).expect("Failed to read .gitignore content");
        if !gitignore_content.lines().any(|line| line.trim() == ".trunk") {
            gitignore_needs_update = true;
        }
    } else {
        gitignore_needs_update = true;
    }
    if gitignore_needs_update {
        debug!("✨ Step 6: Adding .trunk to .gitignore");
        let mut gitignore_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .append(true)
            .open(&gitignore_path)
            .unwrap_or_else(|e| {
                error!("❌ Failed to open .gitignore: {}", e);
                exit(1);
            });

        // Check if the file is non-empty and doesn't end with a newline
        let mut contents = String::new();
        gitignore_file.read_to_string(&mut contents).unwrap_or_else(|e| {
            error!("❌ Failed to read .gitignore: {}", e);
            exit(1);
        });
        if !contents.is_empty() && !contents.ends_with('\n') {
            writeln!(gitignore_file, "").expect("Failed to write newline to .gitignore");
        }

        writeln!(gitignore_file, ".trunk").expect("Failed to write .trunk to .gitignore");
        info!("✓ Step 6: Added .trunk to .gitignore");
    } else {
        debug!("= Step 6: .trunk already in .gitignore");
        info!("= Step 6: .trunk already in .gitignore");
    }

    // Step 7: Create .trunk parent directory if it doesn't exist
    let parent_trunk_dir = repo_root.join(".trunk");
    if !parent_trunk_dir.exists() {
        debug!("✨ Step 7a: Creating parent .trunk directory");
        fs::create_dir(&parent_trunk_dir).unwrap_or_else(|e| {
            error!("❌ Failed to create .trunk parent directory: {}", e);
            exit(1);
        });
        info!("✓ Step 7a: .trunk parent directory created at {:?}", parent_trunk_dir);
    }
    
    // Step 8: Check if .trunk/<store_name> exists
    debug!("➡️ Step 8: Checking if {} directory exists for store '{}'", store_dir_relative_path, store_name);
    if trunk_store_dir.exists() {
        let should_overwrite = if args.force {
            info!("🚀 Step 8: {} exists, --force specified, will overwrite for store '{}'", store_dir_relative_path, store_name);
            true
        } else {
            debug!("📍 Step 8: {} directory exists for store '{}'", store_dir_relative_path, store_name);
            print!("🐘︖ Overwrite existing {} directory for store '{}'? [y/N]: ", store_dir_relative_path, store_name);
            io::stdout().flush().expect("Failed to flush stdout");
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("Failed to read user input");
            let input = input.trim().to_lowercase();
            if input == "y" || input == "yes" {
                debug!("👍 Step 8: User confirmed overwrite for store '{}'", store_name);
                true
            } else {
                info!("🚫 Step 8: Checkout for store '{}' aborted by user", store_name);
                exit(0);
            }
        };
        if should_overwrite {
            debug!("🗑️ Step 8: Removing existing {} directory for store '{}'", store_dir_relative_path, store_name);
            fs::remove_dir_all(&trunk_store_dir).unwrap_or_else(|e| {
                error!("❌ Failed to remove existing {} directory: {}", store_dir_relative_path, e);
                exit(1);
            });
            info!("✓ Step 8: Existing {} directory removed for store '{}'", store_dir_relative_path, store_name);
        }
    } else {
        debug!("∉ Step 8: {} directory does not exist for store '{}'", store_dir_relative_path, store_name);
    }

    // Step 9: Create .trunk/<store_name> directory
    debug!("✨ Step 9: Creating {} directory for store '{}'", store_dir_relative_path, store_name);
    fs::create_dir_all(&trunk_store_dir).unwrap_or_else(|e| { // create_dir_all for parent .trunk too
        error!("❌ Failed to create {} directory: {}", store_dir_relative_path, e);
        exit(1);
    });
    info!("✓ Step 9: {} directory created for store '{}'", store_dir_relative_path, store_name);

    // Step 10: Initialize Git repository in .trunk/<store_name>
    debug!("⚙️ Step 10: Initializing Git repository in {}", store_dir_relative_path);
    run_git_command(Command::new("git").arg("init").current_dir(&trunk_store_dir), verbose)
        .and_then(|out| if !out.status.success() { Err(io::Error::new(io::ErrorKind::Other, "git init failed")) } else { Ok(()) })
        .unwrap_or_else(|e| { error!("❌ Failed to run git init in {}: {}", store_dir_relative_path, e); exit(1); });
    info!("✓ Step 10: Git repository initialized in {}", store_dir_relative_path);

    // Step 11: Fetch history from main repo's refs/trunk/<store_name> into a temporary ref in .trunk/<store_name>
    let temp_store_ref = "refs/temp/trunk_store_data";
    debug!("📥 Step 11: Fetching {} from main repo into {} temporary ref '{}'", trunk_ref_name, store_dir_relative_path, temp_store_ref);
    run_git_command(
        Command::new("git")
            .arg("fetch")
            .arg(repo_root.as_os_str()) // Path to main repository
            .arg(format!("{}:{}", trunk_ref_name, temp_store_ref))
            .current_dir(&trunk_store_dir),
        verbose,
    )
    .and_then(|out| if !out.status.success() { Err(io::Error::new(io::ErrorKind::Other, "git fetch failed")) } else { Ok(()) })
    .unwrap_or_else(|e| { error!("❌ Failed to fetch {} into {}: {}", trunk_ref_name, store_dir_relative_path, e); exit(1); });
    info!("✓ Step 11: Successfully fetched {} into temporary ref in {}", trunk_ref_name, store_dir_relative_path);

    // Step 12: Get the fetched commit hash from the temporary ref
    debug!("🔑 Step 12: Getting fetched commit hash from {} in {}", temp_store_ref, store_dir_relative_path);
    let commit_hash_output = run_git_command(
        Command::new("git").arg("rev-parse").arg(temp_store_ref).current_dir(&trunk_store_dir),
        verbose,
    ).unwrap_or_else(|e| { error!("❌ Failed to get fetched commit hash from {}: {}", temp_store_ref, e); exit(1); });
    if !commit_hash_output.status.success() { error!("❌ {} not found after fetch in {}", temp_store_ref, store_dir_relative_path); exit(1); }
    let commit_hash = String::from_utf8_lossy(&commit_hash_output.stdout).trim().to_string();
    info!("✓ Step 12: Fetched commit hash for store '{}': {}", store_name, commit_hash);

    // Step 13: Reset main branch in .trunk/<store_name> to the fetched commit
    debug!("🔄 Step 13: Resetting {} main branch to fetched commit {}", store_dir_relative_path, commit_hash);
    run_git_command(Command::new("git").arg("reset").arg("--hard").arg(&commit_hash).current_dir(&trunk_store_dir), verbose)
        .and_then(|out| if !out.status.success() { Err(io::Error::new(io::ErrorKind::Other, "git reset failed")) } else { Ok(()) })
        .unwrap_or_else(|e| { error!("❌ Failed to reset {} to fetched commit: {}", store_dir_relative_path, e); exit(1); });
    info!("✓ Step 13: Main branch in {} reset to commit {}", store_dir_relative_path, commit_hash);

    // Step 14: Update main branch ref in .trunk/<store_name> (git reset --hard might not update HEAD if not on a branch yet)
    debug!("🔄 Step 14: Updating refs/heads/main in {}", store_dir_relative_path);
    run_git_command(Command::new("git").arg("update-ref").arg("refs/heads/main").arg(&commit_hash).current_dir(&trunk_store_dir), verbose)
        .and_then(|out| if !out.status.success() { Err(io::Error::new(io::ErrorKind::Other, "git update-ref failed")) } else { Ok(()) })
        .unwrap_or_else(|e| { error!("❌ Failed to update refs/heads/main in {}: {}", store_dir_relative_path, e); exit(1); });
    info!("✓ Step 14: refs/heads/main updated in {}", store_dir_relative_path);
    
    // Step 14b: Ensure .trunk/<store_name> is on the main branch
    debug!("⤵️ Step 14b: Ensuring {} is on the main branch", store_dir_relative_path);
    run_git_command(Command::new("git").arg("checkout").arg("main").current_dir(&trunk_store_dir), verbose)
        .and_then(|out| if !out.status.success() { Err(io::Error::new(io::ErrorKind::Other, "git checkout main failed")) } else { Ok(()) })
        .unwrap_or_else(|e| { error!("❌ Failed to checkout main in {}: {}", store_dir_relative_path, e); exit(1); });


    // Step 15: Clean up temporary ref in .trunk/<store_name>
    debug!("🧹 Step 15: Cleaning up temporary ref {} in {}", temp_store_ref, store_dir_relative_path);
    if let Err(e) = run_git_command(Command::new("git").arg("update-ref").arg("-d").arg(temp_store_ref).current_dir(&trunk_store_dir), verbose) {
        error!("⚠️ Warning: Failed to delete temporary ref {} in {}: {}", temp_store_ref, store_dir_relative_path, e);
    }
    info!("✓ Step 15: Temporary ref cleaned up in {}", store_dir_relative_path);

    info!("✅ Trunk store '{}' checked out successfully into {}", store_name, store_dir_relative_path);
}