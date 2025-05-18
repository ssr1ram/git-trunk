use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, exit};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Initialize the .trunk directory")]
pub struct InitArgs {
    #[arg(long, help = "Force initialization, overwriting existing .trunk directory")]
    force: bool,
}

pub fn run(args: &InitArgs) {
    // Step 1: Check if we are in a Git repository
    let git_check_output = Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output();

    if git_check_output
        .map(|output| !output.status.success())
        .unwrap_or(true)
    {
        eprintln!("\u{274C} init can only be invoked inside a git repo");
        exit(1);
    }

    // Step 2: Get repository root
    let repo_root_output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .expect("Failed to get git repository root");
    
    let repo_root_temp = String::from_utf8_lossy(&repo_root_output.stdout);
    let repo_root = repo_root_temp.trim().to_string();

    // Step 3: Ensure .trunk is in .gitignore
    let gitignore_path = Path::new(&repo_root).join(".gitignore");
    let mut gitignore_content = String::new();
    let mut gitignore_needs_update = false;

    if gitignore_path.exists() {
        let mut gitignore_file = File::open(&gitignore_path).expect("Failed to read .gitignore");
        gitignore_file
            .read_to_string(&mut gitignore_content)
            .expect("Failed to read .gitignore content");
        if !gitignore_content.lines().any(|line| line.trim() == ".trunk") {
            gitignore_needs_update = true;
        }
    } else {
        gitignore_needs_update = true;
    }

    if gitignore_needs_update {
        let mut gitignore_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore_path)
            .expect("Failed to open .gitignore");
        writeln!(gitignore_file, ".trunk").expect("Failed to write .trunk to .gitignore");
        println!("Added .trunk to .gitignore");
    }

    // Step 4: Create .trunk directory
    let trunk_dir = Path::new(&repo_root).join(".trunk");
    if trunk_dir.exists() {
        if args.force {
            fs::remove_dir_all(&trunk_dir).expect("Failed to remove existing .trunk directory");
        } else {
            println!("Trunk is already initialized in this repository.");
            return;
        }
    }
    fs::create_dir(&trunk_dir).expect("Failed to create .trunk directory");

    // Step 5: Create .trunk/readme.md
    let readme_path = trunk_dir.join("readme.md");
    let mut readme_file = File::create(&readme_path).expect("Failed to create readme.md");
    writeln!(
        readme_file,
        "# Trunk Documents\n\nThis directory stores repository-wide documents managed by git-trunk."
    )
    .expect("Failed to write to readme.md");

    // Step 6: Initialize Git in .trunk
    Command::new("git")
        .arg("init")
        .current_dir(&trunk_dir)
        .status()
        .expect("Failed to run git init in .trunk");

    Command::new("git")
        .arg("add")
        .arg("-A")
        .current_dir(&trunk_dir)
        .status()
        .expect("Failed to run git add in .trunk");

    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("Initial trunk commit")
        .current_dir(&trunk_dir)
        .status()
        .expect("Failed to run git commit in .trunk");

    // Step 7: Create pre-push hook
    let hooks_dir = Path::new(&repo_root).join(".git").join("hooks");
    let pre_push_path = hooks_dir.join("pre-push");
    let hook_content = r#"#!/bin/sh
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

    fs::create_dir_all(&hooks_dir).expect("Failed to create hooks directory");
    let mut pre_push_file = File::create(&pre_push_path).expect("Failed to create pre-push hook");
    writeln!(pre_push_file, "{}", hook_content).expect("Failed to write pre-push hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&pre_push_path, fs::Permissions::from_mode(0o755))
            .expect("Failed to set executable permissions on pre-push hook");
    }

    println!("Trunk initialized successfully.");
}