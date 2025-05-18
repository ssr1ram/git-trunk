use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, exit};

#[allow(dead_code)]
pub fn sync() {
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

    let trunk_dir = Path::new(&repo_root).join(".trunk");
    if !trunk_dir.exists() {
        eprintln!("\u{274C} .trunk directory not found. Run `git trunk init` first.");
        exit(1);
    }

    // Step 2: Check if .trunk has files to be staged
    let status_output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(&trunk_dir)
        .output();
    let status_output = status_output.unwrap_or_else(|e| {
        eprintln!("\u{274C} Failed to run git status in .trunk: {}", e);
        exit(1);
    });

    let status = String::from_utf8_lossy(&status_output.stdout);
    if status.is_empty() {
        println!("No changes to stage in .trunk.");
    } else {
        // Step 3: Ask user to stage all files
        println!("Changes detected in .trunk:\n{}", status);
        print!("Stage all files? [y/N]: ");
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read user input");
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Sync aborted by user.");
            exit(0);
        }

        // Stage all files
        let stage_status = Command::new("git")
            .arg("add")
            .arg("-A")
            .current_dir(&trunk_dir)
            .status();
        stage_status.unwrap_or_else(|e| {
            eprintln!("\u{274C} Failed to run git add in .trunk: {}", e);
            exit(1);
        });

        // Step 4: Commit staged files
        let commit_status = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("Sync trunk changes")
            .current_dir(&trunk_dir)
            .status();
        let commit_status = commit_status.unwrap_or_else(|e| {
            eprintln!("\u{274C} Failed to run git commit in .trunk: {}", e);
            exit(1);
        });

        if !commit_status.success() {
            println!("No changes to commit in .trunk.");
        }
    }

    // Step 5: Get the latest commit hash from .trunk
    let commit_hash_output = Command::new("git")
        .arg("rev-parse")
        .arg("main")
        .current_dir(&trunk_dir)
        .output();
    let commit_hash_output = commit_hash_output.unwrap_or_else(|e| {
        eprintln!("\u{274C} Failed to get .trunk main commit hash: {}", e);
        exit(1);
    });
    let commit_hash = String::from_utf8_lossy(&commit_hash_output.stdout)
        .trim()
        .to_string();

    // Step 6: Fetch objects from .trunk to main repo
    let fetch_status = Command::new("git")
        .arg("-C")
        .arg(&repo_root)
        .arg("fetch")
        .arg(&trunk_dir)
        .arg("main:trunk-temp")
        .status();
    fetch_status.unwrap_or_else(|e| {
        eprintln!("\u{274C} Failed to fetch objects from .trunk: {}", e);
        exit(1);
    });

    // Step 7: Update refs/trunk/main
    let ref_exists = Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg("refs/trunk/main")
        .current_dir(&repo_root)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    let update_ref_status = Command::new("git")
        .arg("update-ref")
        .arg("refs/trunk/main")
        .arg(&commit_hash)
        .current_dir(&repo_root)
        .status();
    update_ref_status.unwrap_or_else(|e| {
        eprintln!("\u{274C} Failed to update refs/trunk/main: {}", e);
        exit(1);
    });

    // Clean up temporary branch
    Command::new("git")
        .arg("branch")
        .arg("-D")
        .arg("trunk-temp")
        .current_dir(&repo_root)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to delete temporary branch trunk-temp: {}", e);
            exit(1);
        });

    if ref_exists {
        println!("Updated refs/trunk/main to commit {}.", commit_hash);
    } else {
        println!("Created refs/trunk/main at commit {}.", commit_hash);
    }

    println!("Trunk synced successfully.");
}