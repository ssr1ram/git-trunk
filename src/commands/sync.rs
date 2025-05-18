use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, exit};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Sync changes from .trunk to the main repository")]
pub struct SyncArgs {
    #[arg(long, help = "Skip interactive prompts and stage all changes")]
    force: bool,
}

#[allow(dead_code)]
pub fn run(args: &SyncArgs) {
    // Step 1: Get repository root
    println!("\u{1F418} Step 1: Getting repository root");
    let repo_root_output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output();
    let repo_root_output = repo_root_output.unwrap_or_else(|e| {
        eprintln!("\u{1F418} Error: Failed to get git repository root: {}", e);
        exit(1);
    });
    let repo_root_temp = String::from_utf8_lossy(&repo_root_output.stdout);
    let repo_root = repo_root_temp.trim().to_string();
    println!("\u{1F418} Step 1: Repository root found at {}", repo_root);

    // Step 2: Check if .trunk exists
    println!("\u{1F418} Step 2: Checking for .trunk directory");
    let trunk_dir = Path::new(&repo_root).join(".trunk");
    if !trunk_dir.exists() {
        eprintln!("\u{1F418} Error: .trunk directory not found. Run `git trunk init` first.");
        exit(1);
    }
    println!("\u{1F418} Step 2: .trunk directory found");

    // Step 3: Check if .trunk has files to be staged
    println!("\u{1F418} Step 3: Checking for changes in .trunk");
    let status_output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(&trunk_dir)
        .output();
    let status_output = status_output.unwrap_or_else(|e| {
        eprintln!("\u{1F418} Error: Failed to run git status in .trunk: {}", e);
        exit(1);
    });

    let status = String::from_utf8_lossy(&status_output.stdout);
    if status.is_empty() {
        println!("\u{1F418} Step 3: No changes to stage in .trunk");
    } else {
        // Step 4: Ask user to stage all files (unless --force)
        let should_stage = if args.force {
            println!("\u{1F418} Step 4: --force specified, staging all changes");
            true
        } else {
            println!("\u{1F418} Step 4: Changes detected in .trunk:\n{}", status);
            print!("\u{1F418} Stage all files? [y/N]: ");
            io::stdout().flush().expect("Failed to flush stdout");

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read user input");
            let input = input.trim().to_lowercase();
            if input == "y" || input == "yes" {
                println!("\u{1F418} Step 4: User confirmed staging");
                true
            } else {
                println!("\u{1F418} Step 4: Sync aborted by user");
                exit(0);
            }
        };

        if should_stage {
            // Stage all files
            println!("\u{1F418} Step 4: Staging all files in .trunk");
            let stage_status = Command::new("git")
                .arg("add")
                .arg("-A")
                .current_dir(&trunk_dir)
                .status();
            stage_status.unwrap_or_else(|e| {
                eprintln!("\u{1F418} Error: Failed to run git add in .trunk: {}", e);
                exit(1);
            });
            println!("\u{1F418} Step 4: Files staged");

            // Step 5: Commit staged files
            println!("\u{1F418} Step 5: Committing staged changes");
            let commit_status = Command::new("git")
                .arg("commit")
                .arg("-m")
                .arg("Sync trunk changes")
                .current_dir(&trunk_dir)
                .status();
            let commit_status = commit_status.unwrap_or_else(|e| {
                eprintln!("\u{1F418} Error: Failed to run git commit in .trunk: {}", e);
                exit(1);
            });

            if !commit_status.success() {
                println!("\u{1F418} Step 5: No changes to commit in .trunk");
            } else {
                println!("\u{1F418} Step 5: Changes committed");
            }
        }
    }

    // Step 6: Get the latest commit hash from .trunk
    println!("\u{1F418} Step 6: Getting latest commit hash from .trunk");
    let commit_hash_output = Command::new("git")
        .arg("rev-parse")
        .arg("main")
        .current_dir(&trunk_dir)
        .output();
    let commit_hash_output = commit_hash_output.unwrap_or_else(|e| {
        eprintln!("\u{1F418} Error: Failed to get .trunk main commit hash: {}", e);
        exit(1);
    });
    let commit_hash = String::from_utf8_lossy(&commit_hash_output.stdout)
        .trim()
        .to_string();
    println!("\u{1F418} Step 6: Commit hash: {}", commit_hash);

    // Step 7: Fetch objects from .trunk to main repo
    println!("\u{1F418} Step 7: Fetching objects from .trunk to main repository");
    let fetch_status = Command::new("git")
        .arg("-C")
        .arg(&repo_root)
        .arg("fetch")
        .arg(&trunk_dir)
        .arg("main:trunk-temp")
        .status();
    fetch_status.unwrap_or_else(|e| {
        eprintln!("\u{1F418} Error: Failed to fetch objects from .trunk: {}", e);
        exit(1);
    });
    println!("\u{1F418} Step 7: Objects fetched");

    // Step 8: Update refs/trunk/main
    println!("\u{1F418} Step 8: Checking if refs/trunk/main exists");
    let ref_exists = Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg("refs/trunk/main")
        .current_dir(&repo_root)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    println!("\u{1F418} Step 8: Updating refs/trunk/main");
    let update_ref_status = Command::new("git")
        .arg("update-ref")
        .arg("refs/trunk/main")
        .arg(&commit_hash)
        .current_dir(&repo_root)
        .status();
    update_ref_status.unwrap_or_else(|e| {
        eprintln!("\u{1F418} Error: Failed to update refs/trunk/main: {}", e);
        exit(1);
    });

    // Step 9: Clean up temporary branch
    println!("\u{1F418} Step 9: Cleaning up temporary branch trunk-temp");
    Command::new("git")
        .arg("branch")
        .arg("-D")
        .arg("trunk-temp")
        .current_dir(&repo_root)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("\u{1F418} Warning: Failed to delete temporary branch trunk-temp: {}", e);
            exit(1);
        });

    if ref_exists {
        println!("\u{1F418} Step 8: Updated refs/trunk/main to commit {}", commit_hash);
    } else {
        println!("\u{1F418} Step 8: Created refs/trunk/main at commit {}", commit_hash);
    }

    println!("\u{1F418} Trunk synced successfully");
}