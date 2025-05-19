use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, exit};
use clap::Parser;
use log::{debug, error, info};

#[derive(Parser, Debug)]
#[command(about = "Remove all traces of .trunk from the main repository")]
pub struct SteganoArgs {}

pub fn run(_args: &SteganoArgs, verbose: bool) {
    // Step 1: Check if we are in a Git repository
    debug!("Step 1: Checking if inside a Git repository");
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
        error!("stegano can only be invoked inside a git repo");
        exit(1);
    }
    info!("✓ Step 1: Confirmed inside a Git repository");

    // Step 2: Get repository root
    debug!("Step 2: Getting repository root");
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
    info!("✓ Step 2: Repository root found at {}", repo_root);

    // Step 3: Remove .trunk from .gitignore
    debug!("Step 3: Checking .gitignore for .trunk entry");
    let gitignore_path = Path::new(&repo_root).join(".gitignore");
    if gitignore_path.exists() {
        let mut gitignore_content = String::new();
        let mut gitignore_file = File::open(&gitignore_path).unwrap_or_else(|e| {
            error!("Failed to read .gitignore: {}", e);
            exit(1);
        });
        gitignore_file
            .read_to_string(&mut gitignore_content)
            .expect("Failed to read .gitignore content");

        let updated_content: String = gitignore_content
            .lines()
            .filter(|line| line.trim() != ".trunk")
            .collect::<Vec<&str>>()
            .join("\n");

        if updated_content != gitignore_content {
            debug!("Step 3: Removing .trunk from .gitignore");
            let mut gitignore_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&gitignore_path)
                .unwrap_or_else(|e| {
                    error!("Failed to open .gitignore for writing: {}", e);
                    exit(1);
                });
            if !updated_content.is_empty() {
                writeln!(gitignore_file, "{}", updated_content)
                    .expect("Failed to write updated .gitignore content");
            }
            info!("✓ Step 3: Removed .trunk from .gitignore");
        } else {
            debug!("Step 3: No .trunk entry found in .gitignore");
            info!("= Step 3: No .trunk entry to remove from .gitignore");
        }
    } else {
        debug!("Step 3: No .gitignore file found");
        info!("= Step 3: No .gitignore file exists");
    }

    // Step 4: Remove .trunk directory
    debug!("Step 4: Checking for .trunk directory");
    let trunk_dir = Path::new(&repo_root).join(".trunk");
    if trunk_dir.exists() {
        debug!("Step 4: Removing .trunk directory");
        fs::remove_dir_all(&trunk_dir).unwrap_or_else(|e| {
            error!("Failed to remove .trunk directory: {}", e);
            exit(1);
        });
        info!("✓ Step 4: .trunk directory removed");
    } else {
        debug!("Step 4: No .trunk directory found");
        info!("= Step 4: No .trunk directory to remove");
    }

    info!("✅ Stegano completed successfully: All traces of .trunk removed");
}

fn run_git_command(command: &mut Command, verbose: bool) -> io::Result<std::process::Output> {
    // Check if git is available
    let git_check = Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if git_check.is_err() || !git_check.unwrap().success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Git executable not found or failed to execute",
        ));
    }

    // Always capture stdout, suppress stderr in non-verbose mode
    if !verbose {
        command.stderr(std::process::Stdio::null());
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