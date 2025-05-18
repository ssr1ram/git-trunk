use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, exit};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Clone the trunk from refs/trunk/main into .trunk")]
pub struct CloneArgs {
    #[arg(long, help = "Force cloning, overwriting existing .trunk directory")]
    force: bool,
}

#[allow(dead_code)]
pub fn run(args: &CloneArgs) {
    // Step 1: Get repository root
    let repo_root_output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output();
    let repo_root_output = repo_root_output.unwrap_or_else(|e| {
        eprintln!("\u{274C} Failed to get git repository root: {}", e);
        exit(1);
    });
    let repo_root_temp = String::from_utf8_lossy(&repo_root_output.stdout);
    let repo_root = repo_root_temp.trim().to_string();

    // Step 2: Check if refs/trunk/main exists locally
    let local_ref_check = Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg("refs/trunk/main")
        .current_dir(&repo_root)
        .output();
    let local_ref_exists = local_ref_check
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !local_ref_exists {
        // Step 3: Check if refs/trunk/main exists on the remote
        let remote_ref_check = Command::new("git")
            .arg("ls-remote")
            .arg("origin")
            .arg("refs/trunk/main")
            .current_dir(&repo_root)
            .output();
        let remote_ref_check = remote_ref_check.unwrap_or_else(|e| {
            eprintln!("\u{274C} Failed to check refs/trunk/main on remote: {}", e);
            exit(1);
        });
        if !remote_ref_check.status.success() || remote_ref_check.stdout.is_empty() {
            eprintln!("\u{274C} refs/trunk/main does not exist in the repository or on the remote (origin). Ensure it was pushed with `git trunk push`.");
            exit(1);
        }

        // Step 4: Fetch refs/trunk/main from origin
        let fetch_status = Command::new("git")
            .arg("fetch")
            .arg("origin")
            .arg("refs/trunk/main:refs/trunk/main")
            .current_dir(&repo_root)
            .status();
        let fetch_status = fetch_status.unwrap_or_else(|e| {
            eprintln!("\u{274C} Failed to fetch refs/trunk/main from origin: {}", e);
            exit(1);
        });
        if !fetch_status.success() {
            eprintln!("\u{274C} Failed to fetch refs/trunk/main from origin. Check remote configuration and network connectivity.");
            exit(1);
        }
    }

    // Step 5: Verify refs/trunk/main exists locally after fetch (if needed)
    let final_ref_check = Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg("refs/trunk/main")
        .current_dir(&repo_root)
        .output();
    if final_ref_check
        .map(|output| !output.status.success())
        .unwrap_or(true)
    {
        eprintln!("\u{274C} refs/trunk/main is still missing after attempting to fetch. Ensure it was pushed to the remote.");
        exit(1);
    }

    // Step 6: Check if .trunk exists
    let trunk_dir = Path::new(&repo_root).join(".trunk");
    if trunk_dir.exists() {
        let should_overwrite = if args.force {
            true
        } else {
            println!("The .trunk directory already exists.");
            print!("Overwrite existing .trunk directory? [y/N]: ");
            io::stdout().flush().expect("Failed to flush stdout");

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read user input");
            let input = input.trim().to_lowercase();
            input == "y" || input == "yes"
        };

        if !should_overwrite {
            println!("Clone aborted by user.");
            exit(0);
        }

        // Remove existing .trunk directory
        fs::remove_dir_all(&trunk_dir).unwrap_or_else(|e| {
            eprintln!("\u{274C} Failed to remove existing .trunk directory: {}", e);
            exit(1);
        });
    }

    // Step 7: Create .trunk directory
    fs::create_dir(&trunk_dir).unwrap_or_else(|e| {
        eprintln!("\u{274C} Failed to create .trunk directory: {}", e);
        exit(1);
    });

    // Step 8: Initialize Git repository in .trunk
    let init_status = Command::new("git")
        .arg("init")
        .current_dir(&trunk_dir)
        .status();
    let init_status = init_status.unwrap_or_else(|e| {
        eprintln!("\u{274C} Failed to run git init in .trunk: {}", e);
        exit(1);
    });
    if !init_status.success() {
        eprintln!("\u{274C} git init failed in .trunk");
        exit(1);
    }

    // Step 9: Fetch history from refs/trunk/main into a temporary ref
    let fetch_status = Command::new("git")
        .arg("fetch")
        .arg(&repo_root)
        .arg("refs/trunk/main:refs/temp/trunk")
        .current_dir(&trunk_dir)
        .status();
    let fetch_status = fetch_status.unwrap_or_else(|e| {
        eprintln!("\u{274C} Failed to fetch refs/trunk/main into .trunk: {}", e);
        exit(1);
    });
    if !fetch_status.success() {
        eprintln!("\u{274C} git fetch failed for refs/trunk/main");
        exit(1);
    }

    // Step 10: Get the fetched commit hash
    let commit_hash_output = Command::new("git")
        .arg("rev-parse")
        .arg("refs/temp/trunk")
        .current_dir(&trunk_dir)
        .output();
    let commit_hash_output = commit_hash_output.unwrap_or_else(|e| {
        eprintln!("\u{274C} Failed to get fetched commit hash: {}", e);
        exit(1);
    });
    if !commit_hash_output.status.success() {
        eprintln!("\u{274C} refs/temp/trunk not found after fetch");
        exit(1);
    }
    let commit_hash = String::from_utf8_lossy(&commit_hash_output.stdout)
        .trim()
        .to_string();

    // Step 11: Reset main branch to the fetched commit
    let reset_status = Command::new("git")
        .arg("reset")
        .arg("--hard")
        .arg(&commit_hash)
        .current_dir(&trunk_dir)
        .status();
    let reset_status = reset_status.unwrap_or_else(|e| {
        eprintln!("\u{274C} Failed to reset .trunk to fetched commit: {}", e);
        exit(1);
    });
    if !reset_status.success() {
        eprintln!("\u{274C} git reset failed in .trunk");
        exit(1);
    }

    // Step 12: Update main branch ref
    let update_ref_status = Command::new("git")
        .arg("update-ref")
        .arg("refs/heads/main")
        .arg(&commit_hash)
        .current_dir(&trunk_dir)
        .status();
    let update_ref_status = update_ref_status.unwrap_or_else(|e| {
        eprintln!("\u{274C} Failed to update refs/heads/main in .trunk: {}", e);
        exit(1);
    });
    if !update_ref_status.success() {
        eprintln!("\u{274C} git update-ref failed for refs/heads/main");
        exit(1);
    }

    // Step 13: Clean up temporary ref
    if let Err(e) = Command::new("git")
        .arg("update-ref")
        .arg("-d")
        .arg("refs/temp/trunk")
        .current_dir(&trunk_dir)
        .status()
    {
        eprintln!("Warning: Failed to delete temporary ref refs/temp/trunk: {}", e);
        // Non-critical, continue
    }

    println!("Trunk cloned successfully.");
}