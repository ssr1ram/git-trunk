use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, exit, Stdio};
use clap::Parser;
use log::{debug, error, info};

#[derive(Parser, Debug)]
#[command(about = "Initialize the .trunk directory")]
pub struct InitArgs {
    #[arg(long, help = "Force initialization, overwriting existing .trunk directory")]
    force: bool,
}

pub fn run(args: &InitArgs, verbose: bool) {
    // Step 1: Check if we are in a Git repository
    info!("Step 1: Checking if inside a Git repository");
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
        error!("init can only be invoked inside a git repo");
        exit(1);
    }
    info!("✓ Step 1: Confirmed inside a Git repository");

    // Step 2: Get repository root
    info!("Step 2: Getting repository root");
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
    info!("Step 2: Repository root found at {}", repo_root);

    // Step 3: Ensure .trunk is in .gitignore
    info!("Step 3: Checking .gitignore for .trunk entry");
    let gitignore_path = Path::new(&repo_root).join(".gitignore");
    let mut gitignore_content = String::new();
    let mut gitignore_needs_update = false;

    if gitignore_path.exists() {
        let mut gitignore_file = File::open(&gitignore_path).unwrap_or_else(|e| {
            error!("Failed to read .gitignore: {}", e);
            exit(1);
        });
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
        info!("Step 3: Adding .trunk to .gitignore");
        let mut gitignore_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore_path)
            .unwrap_or_else(|e| {
                error!("Failed to open .gitignore: {}", e);
                exit(1);
            });
        writeln!(gitignore_file, ".trunk").expect("Failed to write .trunk to .gitignore");
        info!("✓ Step 3: Added .trunk to .gitignore");
    } else {
        info!("Step 3: .trunk already in .gitignore");
    }

    // Step 4: Create .trunk directory
    info!("Step 4: Checking for .trunk directory");
    let trunk_dir = Path::new(&repo_root).join(".trunk");
    if trunk_dir.exists() {
        if args.force {
            info!("Step 4: .trunk exists, --force specified, removing existing directory");
            fs::remove_dir_all(&trunk_dir).unwrap_or_else(|e| {
                error!("Failed to remove existing .trunk directory: {}", e);
                exit(1);
            });
            info!("✓ Step 4: Existing .trunk directory removed");
        } else {
            info!("Step 4: Trunk is already initialized in this repository");
            return;
        }
    }
    info!("Step 4: Creating .trunk directory");
    fs::create_dir(&trunk_dir).unwrap_or_else(|e| {
        error!("Failed to create .trunk directory: {}", e);
        exit(1);
    });
    info!("✓ Step 4: .trunk directory created");

    // Step 5: Create .trunk/readme.md
    info!("Step 5: Creating .trunk/readme.md");
    let readme_path = trunk_dir.join("readme.md");
    let mut readme_file = File::create(&readme_path).unwrap_or_else(|e| {
        error!("Failed to create readme.md: {}", e);
        exit(1);
    });
    writeln!(
        readme_file,
        "# Trunk Documents\n\nThis directory stores repository-wide documents managed by git-trunk."
    )
    .expect("Failed to write to readme.md");
    info!("✓ Step 5: Created .trunk/readme.md");

    // Step 6: Initialize Git in .trunk
    info!("Step 6: Initializing Git repository in .trunk");
    let init_status = run_git_command(
        Command::new("git")
            .arg("init")
            .current_dir(&trunk_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to run git init in .trunk: {}", e);
        exit(1);
    })
    .status;
    if !init_status.success() {
        error!("git init failed in .trunk");
        exit(1);
    }
    info!("✓ Step 6: Git repository initialized");

    // Step 7: Stage files in .trunk
    info!("Step 7: Staging files in .trunk");
    let stage_status = run_git_command(
        Command::new("git")
            .arg("add")
            .arg("-A")
            .current_dir(&trunk_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to run git add in .trunk: {}", e);
        exit(1);
    })
    .status;
    if !stage_status.success() {
        error!("git add failed in .trunk");
        exit(1);
    }
    info!("✓ Step 7: Files staged");

    // Step 8: Commit files in .trunk
    info!("Step 8: Committing initial trunk changes");
    let commit_status = run_git_command(
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("Initial trunk commit")
            .current_dir(&trunk_dir),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("Failed to run git commit in .trunk: {}", e);
        exit(1);
    })
    .status;
    if !commit_status.success() {
        error!("git commit failed in .trunk");
        exit(1);
    }
    info!("✓ Step 8: Initial commit created");

    info!("✓ Trunk initialized successfully");
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