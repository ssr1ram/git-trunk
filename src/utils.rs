use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use log::{debug, info};

pub fn run_git_command(command: &mut Command, verbose: bool) -> io::Result<std::process::Output> {
    // Check if git is available
    let git_check = Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if git_check.is_err() || !git_check.unwrap().success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Git executable not found or failed to execute. Please ensure Git is installed and in your PATH.",
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

pub fn ensure_trunk_in_gitignore(
    repo_root: &Path,
    step_log_prefix: &str,
) -> io::Result<()> {
    let gitignore_path = repo_root.join(".gitignore");
    let mut gitignore_content = String::new();
    let mut gitignore_needs_update = false;

    if gitignore_path.exists() {
        let mut gitignore_file = File::open(&gitignore_path)
            .map_err(|e| {
                io::Error::new(e.kind(), format!("Failed to open .gitignore for reading: {}", e))
            })?;
        gitignore_file.read_to_string(&mut gitignore_content)
            .map_err(|e| {
                io::Error::new(e.kind(), format!("Failed to read .gitignore content: {}", e))
            })?;
        if !gitignore_content.lines().any(|line| line.trim() == ".trunk") {
            gitignore_needs_update = true;
        }
    } else {
        gitignore_needs_update = true;
    }

    if gitignore_needs_update {
        debug!("✨ {}: Adding .trunk to .gitignore", step_log_prefix);
        let mut gitignore_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore_path)
            .map_err(|e| {
                io::Error::new(e.kind(), format!("Failed to open .gitignore for writing: {}", e))
            })?;

        if !gitignore_content.is_empty() && !gitignore_content.ends_with('\n') {
            writeln!(gitignore_file)?;
        }
        writeln!(gitignore_file, ".trunk")?;
        info!("✓ {}: Added .trunk to .gitignore", step_log_prefix);
    } else {
        debug!("= {}: .trunk already in .gitignore", step_log_prefix);
        info!("= {}: .trunk already in .gitignore", step_log_prefix);
    }
    Ok(())
}

pub fn remove_trunk_from_gitignore(
    repo_root: &Path,
    step_log_prefix: &str,
) -> io::Result<()> {
    let gitignore_path = repo_root.join(".gitignore");

    if gitignore_path.exists() {
        let mut current_content = String::new();
        File::open(&gitignore_path)?
            .read_to_string(&mut current_content)?;

        let original_lines_count = current_content.lines().count();
        let new_lines: Vec<&str> = current_content
            .lines()
            .filter(|line| line.trim() != ".trunk")
            .collect();

        if new_lines.len() < original_lines_count {
            let mut updated_content = new_lines.join("\n");
            if !new_lines.is_empty() { // If there's any content left
                updated_content.push('\n');
            }
            // If new_lines was empty, updated_content is empty.
            // If new_lines was ["foo"], updated_content is "foo\n".
            // If new_lines was ["foo", "bar"], updated_content is "foo\nbar\n".

            OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&gitignore_path)?
                .write_all(updated_content.as_bytes())?;
            info!("✓ {}: Removed '.trunk' entry from .gitignore.", step_log_prefix);
        } else {
            debug!("= {}: No '.trunk' entry found to remove in .gitignore.", step_log_prefix);
            info!("= {}: No '.trunk' entry to remove from .gitignore.", step_log_prefix);
        }
    } else {
        debug!("🚫 {}: No .gitignore file found.", step_log_prefix);
        info!("= {}: No .gitignore file to modify.", step_log_prefix);
    }
    Ok(())
}