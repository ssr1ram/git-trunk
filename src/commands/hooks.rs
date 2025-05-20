use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, exit};
use clap::Parser;
use log::{debug, error, info};
use crate::utils::run_git_command;

#[derive(Parser, Debug)]
#[command(about = "Manage Git hooks for a specific git-trunk store")]
pub struct HooksArgs {
    #[arg(long, help = "Force installation of hooks, overwriting existing hooks")]
    force: bool,
}

pub fn run(args: &HooksArgs, _remote_name: &str, store_name: &str, verbose: bool) {
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

    // Step 2: Check if we are in a Git repository
    debug!("➡️ Step 2: Checking if inside a Git repository");
    // This check is somewhat redundant given Step 1, but kept for consistency
    let git_check_output = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree"),
        verbose,
    );
    if git_check_output.map(|output| !output.status.success()).unwrap_or(true) {
        error!("❌ hooks can only be invoked inside a git repo");
        exit(1);
    }
    info!("✓ Step 2: Confirmed inside a Git repository");

    // Step 3: Define hooks directory
    debug!("⚙️ Step 3: Setting up hooks directory");
    let hooks_dir = repo_root.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir).unwrap_or_else(|e| {
        error!("❌ Failed to create hooks directory: {}", e);
        exit(1);
    });
    info!("✓ Step 3: Hooks directory ready at {:?}", hooks_dir.display());

    let trunk_ref_name = format!("refs/trunk/{}", store_name);

    // Step 4: Prompt for post-commit hook
    let post_commit_path = hooks_dir.join("post-commit");
    let install_post_commit = if post_commit_path.exists() && !args.force {
        debug!("📍 Step 4: post-commit hook already exists");
        print!("🐘 Overwrite existing post-commit hook? [y/N]: ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read user input");
        input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes"
    } else {
        debug!("🚫 Step 4: No post-commit hook found or --force specified for store '{}'", store_name);
        print!("🐘 Install post-commit hook to auto-commit .trunk/{} after main repo commits? [y/N]: ", store_name);
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read user input");
        input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" || args.force
    };

    if install_post_commit {
        debug!("✨ Step 4: Creating post-commit hook for store '{}'", store_name);
        let post_commit_content = format!(r#"#!/bin/sh
# Post-commit hook to auto-commit .trunk/{} changes
# This hook is managed by git-trunk.
echo "Git Trunk: Running post-commit hook for store '{}'..."
git trunk commit --force --store {}
if [ $? -eq 0 ]; then
    echo "Git Trunk: Store '{}' committed successfully."
else
    echo "Git Trunk: Warning - Failed to commit store '{}'." >&2
fi
"#, store_name, store_name, store_name, store_name, store_name);
        let mut post_commit_file = File::create(&post_commit_path).unwrap_or_else(|e| {
            error!("❌ Failed to create post-commit hook: {}", e);
            exit(1);
        });
        writeln!(post_commit_file, "{}", post_commit_content).expect("Failed to write post-commit hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&post_commit_path, fs::Permissions::from_mode(0o755)).unwrap_or_else(|e| {
                error!("❌ Failed to set executable permissions on post-commit hook: {}", e);
                // Non-critical for Windows, but log it.
            });
        }
        info!("✓ Step 4: Post-commit hook for store '{}' installed", store_name);
    } else {
        info!("= Step 4: Skipped post-commit hook installation for store '{}'", store_name);
    }

    // Step 5: Prompt for pre-push hook
    let pre_push_path = hooks_dir.join("pre-push");
    let install_pre_push = if pre_push_path.exists() && !args.force {
        debug!("📍 Step 5: pre-push hook already exists");
        print!("🐘 Overwrite existing pre-push hook? [y/N]: ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read user input");
        input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes"
    } else {
        debug!("🚫 Step 5: No pre-push hook found or --force specified for store '{}'", store_name);
        print!("🐘 Install pre-push hook to push {} with main branch pushes? [y/N]: ", trunk_ref_name);
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read user input");
        input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" || args.force
    };

    if install_pre_push {
        debug!("✨ Step 5: Creating pre-push hook for store '{}' (ref: {})", store_name, trunk_ref_name);
        let pre_push_content = format!(r#"#!/bin/sh
# Pre-push hook to ensure {} is pushed when main branch is pushed.
# This hook is managed by git-trunk.
remote_name="$1"
# remote_url="$2" # Not used in this script

# Read stdin to get refs being pushed
while read local_ref local_sha remote_ref remote_sha
do
    # Check if the main working branch (e.g., main, master) is being pushed
    # Adjust "refs/heads/main" if your main branch has a different name
    if [ "$local_ref" = "refs/heads/main" ] || [ "$local_ref" = "refs/heads/master" ]; then
        echo "Git Trunk: Main branch is being pushed to '$remote_name'."
        echo "Git Trunk: Ensuring {} for store '{}' is also pushed."
        # Attempt to push the trunk ref for the specific store
        # Use the remote name provided to the pre-push hook by Git
        git push "$remote_name" {}:{}
        if [ $? -eq 0 ]; then
            echo "Git Trunk: {} pushed successfully to '$remote_name'."
        else
            echo "Git Trunk: Warning - Failed to push {} to '$remote_name'." >&2
            echo "Git Trunk: You might need to push it manually: git trunk push --store {} --remote $remote_name" >&2
        fi
        # We don't want to block the main push if trunk push fails, so we don't exit 1 here.
        # The user will see the warning.
    fi
done

exit 0 # Always exit 0 to not block the push, warnings are printed to stderr
"#, trunk_ref_name, trunk_ref_name, store_name, trunk_ref_name, trunk_ref_name, trunk_ref_name, trunk_ref_name, store_name);
        let mut pre_push_file = File::create(&pre_push_path).unwrap_or_else(|e| {
            error!("❌ Failed to create pre-push hook: {}", e);
            exit(1);
        });
        writeln!(pre_push_file, "{}", pre_push_content).expect("Failed to write pre-push hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&pre_push_path, fs::Permissions::from_mode(0o755)).unwrap_or_else(|e| {
                error!("❌ Failed to set executable permissions on pre-push hook: {}", e);
            });
        }
        info!("✓ Step 5: Pre-push hook for store '{}' (ref: {}) installed", store_name, trunk_ref_name);
    } else {
        info!("= Step 5: Skipped pre-push hook installation for store '{}'", store_name);
    }

    info!("✅ Trunk hooks configuration for store '{}' completed", store_name);
}