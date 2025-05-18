use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, exit, Stdio};
use clap::Parser;
use log::{debug, error, info};

#[derive(Parser, Debug)]
#[command(about = "Manage Git hooks for git-trunk")]
pub struct HooksArgs {
    #[arg(long, help = "Force installation of hooks, overwriting existing hooks")]
    force: bool,
}

pub fn run(args: &HooksArgs, verbose: bool) {
    // Step 1: Get repository root
    info!("Step 1: Getting repository root");
    let repo_root_output = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("--show-toplevel"),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to get git repository root: {}", e);
        exit(1);
    });
    let repo_root = String::from_utf8_lossy(&repo_root_output.stdout).trim().to_string();
    if repo_root.is_empty() {
        error!("Git repository root is empty. Ensure you are in a valid Git repository.");
        exit(1);
    }
    info!("Step 1: Repository root found at {}", repo_root);

    // Step 2: Check if we are in a Git repository
    info!("Step 2: Checking if inside a Git repository");
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
        error!("hooks can only be invoked inside a git repo");
        exit(1);
    }
    info!("✓ Step 2: Confirmed inside a Git repository");

    // Step 3: Define hooks directory
    info!("Step 3: Setting up hooks directory");
    let hooks_dir = Path::new(&repo_root).join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir).unwrap_or_else(|e| {
        error!("Failed to create hooks directory: {}", e);
        exit(1);
    });
    info!("✓ Step 3: Hooks directory ready at {:?}", hooks_dir);

    // Step 4: Prompt for post-commit hook
    let post_commit_path = hooks_dir.join("post-commit");
    let install_post_commit = if post_commit_path.exists() && !args.force {
        info!("Step 4: post-commit hook already exists");
        print!("🐘 Overwrite existing post-commit hook? [y/N]: ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read user input");
        let input = input.trim().to_lowercase();
        input == "y" || input == "yes"
    } else {
        info!("Step 4: No post-commit hook found or --force specified");
        print!("🐘 Install post-commit hook to auto-sync .trunk after commits? [y/N]: ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read user input");
        let input = input.trim().to_lowercase();
        input == "y" || input == "yes" || args.force
    };

    if install_post_commit {
        info!("Step 4: Creating post-commit hook");
        let post_commit_content = r#"#!/bin/sh
# Post-commit hook to auto-sync .trunk changes
git trunk sync --force
"#;
        let mut post_commit_file = File::create(&post_commit_path).unwrap_or_else(|e| {
            error!("Failed to create post-commit hook: {}", e);
            exit(1);
        });
        writeln!(post_commit_file, "{}", post_commit_content)
            .expect("Failed to write post-commit hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&post_commit_path, fs::Permissions::from_mode(0o755))
                .unwrap_or_else(|e| {
                    error!("Failed to set executable permissions on post-commit hook: {}", e);
                    exit(1);
                });
        }
        info!("✓ Step 4: Post-commit hook installed");
    } else {
        info!("Step 4: Skipped post-commit hook installation");
    }

    // Step 5: Prompt for pre-push hook
    let pre_push_path = hooks_dir.join("pre-push");
    let install_pre_push = if pre_push_path.exists() && !args.force {
        info!("Step 5: pre-push hook already exists");
        print!("🐘 Overwrite existing pre-push hook? [y/N]: ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read user input");
        let input = input.trim().to_lowercase();
        input == "y" || input == "yes"
    } else {
        info!("Step 5: No pre-push hook found or --force specified");
        print!("🐘 Install pre-push hook to push refs/trunk/main with main? [y/N]: ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read user input");
        let input = input.trim().to_lowercase();
        input == "y" || input == "yes" || args.force
    };

    if install_pre_push {
        info!("Step 5: Creating pre-push hook");
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
        let mut pre_push_file = File::create(&pre_push_path).unwrap_or_else(|e| {
            error!("Failed to create pre-push hook: {}", e);
            exit(1);
        });
        writeln!(pre_push_file, "{}", pre_push_content)
            .expect("Failed to write pre-push hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&pre_push_path, fs::Permissions::from_mode(0o755))
                .unwrap_or_else(|e| {
                    error!("Failed to set executable permissions on pre-push hook: {}", e);
                    exit(1);
                });
        }
        info!("✓ Step 5: Pre-push hook installed");
    } else {
        info!("Step 5: Skipped pre-push hook installation");
    }

    info!("✓ Git hooks configuration completed");
}

fn run_git_command(command: &mut Command, verbose: bool) -> io::Result<std::process::Output> {
    // Check if git is available
    let git_check = Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if git_check.is_err() || !git_check.unwrap().success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Git executable not found or failed to execute",
        ));
    }

    // Always capture stdout, suppress stderr in non-verbose mode
    if !verbose {
        command.stderr(Stdio::null());
    }
    let output = command.output()?;
    if verbose {
        if !output.stdout.is_empty() {
            debug!("Git stdout: {}", String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            debug!("Git stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
    }
    Ok(output)
}