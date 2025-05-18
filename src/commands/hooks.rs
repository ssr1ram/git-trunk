use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, exit};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Manage Git hooks for git-trunk")]
pub struct HooksArgs {
    #[arg(long, help = "Force installation of hooks, overwriting existing hooks")]
    force: bool,
}

#[allow(dead_code)]
pub fn run(args: &HooksArgs) {
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

    // Step 2: Check if we are in a Git repository
    println!("\u{1F418} Step 2: Checking if inside a Git repository");
    let git_check_output = Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output();
    if git_check_output
        .map(|output| !output.status.success())
        .unwrap_or(true)
    {
        eprintln!("\u{1F418} Error: hooks can only be invoked inside a git repo");
        exit(1);
    }
    println!("\u{1F418} Step 2: Confirmed inside a Git repository");

    // Step 3: Define hooks directory
    println!("\u{1F418} Step 3: Setting up hooks directory");
    let hooks_dir = Path::new(&repo_root).join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir).unwrap_or_else(|e| {
        eprintln!("\u{1F418} Error: Failed to create hooks directory: {}", e);
        exit(1);
    });
    println!("\u{1F418} Step 3: Hooks directory ready at {:?}", hooks_dir);

    // Step 4: Prompt for post-commit hook
    let post_commit_path = hooks_dir.join("post-commit");
    let install_post_commit = if post_commit_path.exists() && !args.force {
        println!("\u{1F418} Step 4: post-commit hook already exists");
        print!("\u{1F418} Overwrite existing post-commit hook? [y/N]: ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read user input");
        let input = input.trim().to_lowercase();
        input == "y" || input == "yes"
    } else {
        println!("\u{1F418} Step 4: No post-commit hook found or --force specified");
        print!("\u{1F418} Install post-commit hook to auto-sync .trunk after commits? [y/N]: ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read user input");
        let input = input.trim().to_lowercase();
        input == "y" || input == "yes" || args.force
    };

    if install_post_commit {
        println!("\u{1F418} Step 4: Creating post-commit hook");
        let post_commit_content = r#"#!/bin/sh
# Post-commit hook to auto-sync .trunk changes
git trunk sync --force
"#;
        let mut post_commit_file = File::create(&post_commit_path)
            .unwrap_or_else(|e| {
                eprintln!("\u{1F418} Error: Failed to create post-commit hook: {}", e);
                exit(1);
            });
        writeln!(post_commit_file, "{}", post_commit_content)
            .expect("Failed to write post-commit hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&post_commit_path, fs::Permissions::from_mode(0o755))
                .unwrap_or_else(|e| {
                    eprintln!("\u{1F418} Error: Failed to set executable permissions on post-commit hook: {}", e);
                    exit(1);
                });
        }
        println!("\u{1F418} Step 4: Post-commit hook installed");
    } else {
        println!("\u{1F418} Step 4: Skipped post-commit hook installation");
    }

    // Step 5: Prompt for pre-push hook
    let pre_push_path = hooks_dir.join("pre-push");
    let install_pre_push = if pre_push_path.exists() && !args.force {
        println!("\u{1F418} Step 5: pre-push hook already exists");
        print!("\u{1F418} Overwrite existing pre-push hook? [y/N]: ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read user input");
        let input = input.trim().to_lowercase();
        input == "y" || input == "yes"
    } else {
        println!("\u{1F418} Step 5: No pre-push hook found or --force specified");
        print!("\u{1F418} Install pre-push hook to push refs/trunk/main with main? [y/N]: ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read user input");
        let input = input.trim().to_lowercase();
        input == "y" || input == "yes" || args.force
    };

    if install_pre_push {
        println!("\u{1F418} Step 5: Creating pre-push hook");
        let pre_push_content = r#"#!/bin/sh
# Pre-push hook to ensure refs/trunk/main is pushed
remote="$1"
url="$2"

while read local_ref local_sha remote_ref remote_sha
do
    if [ "$local_ref" = "refs/heads/main" ]; then
        git push "$remote" refs/trunk/main:refs/trunk/main
    fi
done
exit 0
"#;
        let mut pre_push_file = File::create(&pre_push_path)
            .unwrap_or_else(|e| {
                eprintln!("\u{1F418} Error: Failed to create pre-push hook: {}", e);
                exit(1);
            });
        writeln!(pre_push_file, "{}", pre_push_content)
            .expect("Failed to write pre-push hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&pre_push_path, fs::Permissions::from_mode(0o755))
                .unwrap_or_else(|e| {
                    eprintln!("\u{1F418} Error: Failed to set executable permissions on pre-push hook: {}", e);
                    exit(1);
                });
        }
        println!("\u{1F418} Step 5: Pre-push hook installed");
    } else {
        println!("\u{1F418} Step 5: Skipped pre-push hook installation");
    }

    println!("\u{1F418} Git hooks configuration completed");
}