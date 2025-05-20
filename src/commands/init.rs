use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::{Command, exit};
use clap::Parser;
use log::{debug, error, info};
use crate::utils::{run_git_command, ensure_trunk_in_gitignore};

#[derive(Parser, Debug)]
#[command(about = "Initialize a .trunk/<store> directory")]
pub struct InitArgs {
    #[arg(long, help = "Force initialization, overwriting existing .trunk/<store> directory")]
    force: bool,
}

pub fn run(args: &InitArgs, _remote_name: &str, store_name: &str, verbose: bool) {
    // Step 1: Check if we are in a Git repository
    debug!("➡️ Step 1: Checking if inside a Git repository");
    let git_check_output = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree"),
        verbose,
    );
    if git_check_output
        .map(|output| !output.status.success())
        .unwrap_or(true)
    {
        error!("❌ init can only be invoked inside a git repo");
        exit(1);
    }
    info!("✓ Step 1: Confirmed inside a Git repository");

    // Step 2: Get repository root
    debug!("➡️ Step 2: Getting repository root");
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
    info!("✓ Step 2: Repository root found at {}", repo_root.display());

    // Step 3: Ensure .trunk is in .gitignore (parent directory)
    debug!("➡️ Step 3: Ensuring .trunk is in .gitignore");
    if let Err(e) = ensure_trunk_in_gitignore(repo_root, "Step 3") {
        error!("❌ Failed to update .gitignore for Step 3: {}", e);
        exit(1);
    }
    // Detailed info/debug for Step 3 (added/already exists) is handled by ensure_trunk_in_gitignore
    
    // Step 4: Create .trunk parent directory if it doesn't exist
    let parent_trunk_dir = repo_root.join(".trunk");
    if !parent_trunk_dir.exists() {
        debug!("✨ Step 4a: Creating parent .trunk directory");
        fs::create_dir(&parent_trunk_dir).unwrap_or_else(|e| {
            error!("❌ Failed to create .trunk parent directory: {}", e);
            exit(1);
        });
        info!("✓ Step 4a: .trunk parent directory created at {:?}", parent_trunk_dir);
    }


    // Step 5: Create .trunk/<store_name> directory
    let store_dir_name = format!(".trunk/{}", store_name);
    debug!("➡️ Step 5: Checking for {} directory", store_dir_name);
    let trunk_store_dir = Path::new(&repo_root).join(&store_dir_name);
    if trunk_store_dir.exists() {
        if args.force {
            debug!("🗑️ Step 5: {} exists, --force specified, removing existing directory", store_dir_name);
            fs::remove_dir_all(&trunk_store_dir).unwrap_or_else(|e| {
                error!("❌ Failed to remove existing {} directory: {}", store_dir_name, e);
                exit(1);
            });
            info!("✓ Step 5: Existing {} directory removed", store_dir_name);
        } else {
            info!("= Step 5: Trunk store '{}' is already initialized in this repository at {}", store_name, store_dir_name);
            return;
        }
    }
    debug!("✨ Step 5: Creating {} directory", store_dir_name);
    fs::create_dir(&trunk_store_dir).unwrap_or_else(|e| {
        error!("❌ Failed to create {} directory: {}", store_dir_name, e);
        exit(1);
    });
    info!("✓ Step 5: {} directory created", store_dir_name);

    // Step 6: Create .trunk/<store_name>/readme.md
    debug!("✨ Step 6: Creating {}/readme.md", store_dir_name);
    let readme_path = trunk_store_dir.join("readme.md");
    let mut readme_file = File::create(&readme_path).unwrap_or_else(|e| {
        error!("❌ Failed to create readme.md in {}: {}", store_dir_name, e);
        exit(1);
    });
    writeln!(
        readme_file,
        "# Trunk Documents for Store: {}\n\nThis directory stores repository-wide documents for the '{}' store, managed by git-trunk.",
        store_name, store_name
    )
    .expect("Failed to write to readme.md");
    info!("✓ Step 6: Created {}/readme.md", store_dir_name);

    // Step 7: Initialize Git in .trunk/<store_name>
    debug!("⚙️ Step 7: Initializing Git repository in {}", store_dir_name);
    let init_status = run_git_command(
        Command::new("git")
            .arg("init")
            .current_dir(&trunk_store_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("❌ Failed to run git init in {}: {}", store_dir_name, e);
        exit(1);
    })
    .status;
    if !init_status.success() {
        error!("❌ git init failed in {}", store_dir_name);
        exit(1);
    }
    info!("✓ Step 7: Git repository initialized in {}", store_dir_name);

    // Step 8: Stage files in .trunk/<store_name>
    debug!("➕ Step 8: Staging files in {}", store_dir_name);
    let stage_status = run_git_command(
        Command::new("git")
            .arg("add")
            .arg("-A")
            .current_dir(&trunk_store_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("❌ Failed to run git add in {}: {}", store_dir_name, e);
        exit(1);
    })
    .status;
    if !stage_status.success() {
        error!("❌ git add failed in {}", store_dir_name);
        exit(1);
    }
    info!("✓ Step 8: Files staged in {}", store_dir_name);

    // Step 9: Commit files in .trunk/<store_name>
    debug!("💾 Step 9: Committing initial changes for store '{}'", store_name);
    let commit_message = format!("Initial commit for store '{}'", store_name);
    let commit_status = run_git_command(
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(&commit_message)
            .current_dir(&trunk_store_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("❌ Failed to run git commit in {}: {}", store_dir_name, e);
        exit(1);
    })
    .status;
    if !commit_status.success() {
        error!("❌ git commit failed in {}", store_dir_name);
        exit(1);
    }
    info!("✓ Step 9: Initial commit created for store '{}'", store_name);

    info!("✅ Trunk store '{}' initialized successfully at {}", store_name, store_dir_name);
}