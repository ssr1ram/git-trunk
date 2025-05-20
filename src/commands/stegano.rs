use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, exit};
use clap::Parser;
use log::{debug, error, info};
use crate::utils::run_git_command;

#[derive(Parser, Debug)]
#[command(about = "Remove all traces of .trunk/<store> from the main repository's working directory. If .trunk becomes empty, it and its .gitignore entry are also removed.")]
pub struct SteganoArgs {}

pub fn run(_args: &SteganoArgs, _remote_name: &str, store_name: &str, verbose: bool) {
    // Step 1: Check if we are in a Git repository
    debug!("➡️ Step 1: Checking if inside a Git repository");
    let git_check_output = run_git_command(
        Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree"),
        verbose,
    );
    if git_check_output.map(|output| !output.status.success()).unwrap_or(true) {
        error!("❌ stegano can only be invoked inside a git repo");
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

    // Step 3: Remove .trunk/<store_name> directory
    let store_dir_relative_path = format!(".trunk/{}", store_name);
    let trunk_store_dir = repo_root.join(&store_dir_relative_path);
    let mut trunk_store_dir_handled = false;

    debug!("➡️ Step 3: Checking for {} directory for store '{}'", store_dir_relative_path, store_name);
    if trunk_store_dir.exists() {
        debug!("🗑️ Step 3: Removing {} directory for store '{}'", store_dir_relative_path, store_name);
        match fs::remove_dir_all(&trunk_store_dir) {
            Ok(_) => {
                info!("✓ Step 3: {} directory removed for store '{}'", store_dir_relative_path, store_name);
                trunk_store_dir_handled = true;
            }
            Err(e) => {
                error!("❌ Failed to remove {} directory: {}. Further cleanup of .trunk and .gitignore might be skipped.", store_dir_relative_path, e);
                // Do not exit, but trunk_store_dir_handled remains false
            }
        }
    } else {
        debug!("🚫 Step 3: No {} directory found for store '{}'", store_dir_relative_path, store_name);
        info!("= Step 3: No {} directory to remove for store '{}'", store_dir_relative_path, store_name);
        trunk_store_dir_handled = true; // Considered handled as it's already gone
    }

    // Step 4: Conditionally remove parent .trunk directory and .gitignore entry
    if trunk_store_dir_handled {
        let parent_trunk_dir = repo_root.join(".trunk");
        let mut cleanup_gitignore_entry = false;

        if parent_trunk_dir.exists() {
            match fs::read_dir(&parent_trunk_dir) {
                Ok(mut entries) => {
                    if entries.next().is_none() { // Parent .trunk directory is empty
                        debug!("🗑️ Step 4a: Parent .trunk directory is empty. Attempting to remove it.");
                        if let Err(e) = fs::remove_dir(&parent_trunk_dir) {
                            error!("⚠️ Warning: Failed to remove empty parent .trunk directory at {}: {}", parent_trunk_dir.display(), e);
                        } else {
                            info!("✓ Step 4a: Empty parent .trunk directory removed.");
                            cleanup_gitignore_entry = true; // Signal to remove from .gitignore
                        }
                    } else {
                        debug!("ℹ️ Step 4a: Parent .trunk directory is not empty (other stores may exist). Retaining it and its .gitignore entry.");
                    }
                },
                Err(e) => {
                    error!("⚠️ Warning: Could not read parent .trunk directory contents at {}: {}", parent_trunk_dir.display(), e);
                }
            }
        } else {
            // Parent .trunk directory doesn't exist, implies it was already cleaned up or this was the only effective store.
            debug!("💨 Step 4a: Parent .trunk directory does not exist. Proceeding with .gitignore cleanup attempt.");
            cleanup_gitignore_entry = true;
        }

        if cleanup_gitignore_entry {
    debug!("🧹 Step 4b: Attempting to remove '.trunk' from .gitignore");
    let gitignore_path = repo_root.join(".gitignore");

    if gitignore_path.exists() {
        let mut gitignore_content = String::new();
        match File::open(&gitignore_path) {
            Ok(mut file) => {
                if let Err(e) = file.read_to_string(&mut gitignore_content) {
                    error!("❌ Failed to read .gitignore content: {}. Skipping .gitignore cleanup.", e);
                } else {
                    // Check for the exact patterns added: "\n.trunk\n" or ".trunk\n"
                    let possible_suffixes = ["\n.trunk\n", ".trunk\n"];
                    let mut updated_content = gitignore_content.clone();
                    let mut modified = false;

                    for suffix in possible_suffixes.iter() {
                        if updated_content.ends_with(suffix) {
                            updated_content = updated_content.strip_suffix(suffix).unwrap().to_string();
                            modified = true;
                            break;
                        }
                    }

                    if modified {
                        match OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .open(&gitignore_path)
                        {
                            Ok(mut file) => {
                                if let Err(e) = write!(file, "{}", updated_content) {
                                    error!("❌ Failed to write updated .gitignore content: {}. Manual cleanup of .gitignore may be needed.", e);
                                } else {
                                    info!("✓ Step 4b: Removed '.trunk' entry from .gitignore.");
                                }
                            }
                            Err(e) => {
                                error!("❌ Failed to open .gitignore for writing: {}. Manual cleanup of .gitignore may be needed.", e);
                            }
                        }
                    } else {
                        debug!("= Step 4b: No matching '.trunk' entry found in .gitignore.");
                        info!("= Step 4b: No '.trunk' entry to remove from .gitignore.");
                    }
                }
            }
            Err(e) => {
                error!("❌ Failed to open .gitignore for reading: {}. Skipping .gitignore cleanup.", e);
            }
        }
    } else {
        debug!("🚫 Step 4b: No .gitignore file found.");
        info!("= Step 4b: No .gitignore file to modify.");
    }
}
    } else {
        info!("⚠️ Skipping .trunk parent directory and .gitignore cleanup due to issues removing the store directory {}.", store_dir_relative_path);
    }

    info!("✅ Stegano for store '{}' completed.", store_name);
}