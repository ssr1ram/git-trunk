use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use clap::Parser;
use log::{debug, error, info};
use crate::utils::run_git_command;
use chrono::{DateTime, Local};

#[derive(Parser, Debug)]
#[command(about = "Displays information about the git-trunk setup and stores")]
pub struct InfoArgs {
    #[arg(long, help = "Discover and display information for all stores found on the remote")]
    all: bool,
}

struct StoreInfo {
    name: String,
    local_path: PathBuf,
    local_path_exists: bool,
    is_git_repo: bool,
    local_store_last_commit_date: Option<String>,
    local_store_last_commit_hash: Option<String>,
    local_store_uncommitted_changes: Option<String>, // "Clean" or "X uncommitted changes"
    main_repo_ref: String,
    main_repo_ref_exists: bool,
    main_repo_ref_commit_date: Option<String>,
    main_repo_ref_commit_hash: Option<String>,
    remote_repo_ref_exists: Option<bool>, // None if remote check fails or not applicable
    remote_repo_ref_commit_hash: Option<String>,
}

fn get_commit_info(repo_path: &Path, ref_name: &str, verbose: bool) -> (Option<String>, Option<String>) {
    match run_git_command(
        Command::new("git")
            .arg("log")
            .arg("-1")
            .arg("--pretty=format:%h%n%at") // hash newline unixtimestamp
            .arg(ref_name)
            .current_dir(repo_path),
        verbose,
    ) {
        Ok(output) if output.status.success() => {
            let out_str = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = out_str.trim().split('\n').collect();
            if parts.len() == 2 {
                let hash = parts[0].to_string();
                let timestamp_str = parts[1];
                if let Ok(timestamp_secs) = timestamp_str.parse::<i64>() {
                    // Use DateTime::from_timestamp to create a DateTime<Utc> directly
                    match DateTime::from_timestamp(timestamp_secs, 0) {
                        Some(utc_dt) => {
                            // Convert to local time
                            let local_dt: DateTime<Local> = utc_dt.with_timezone(&Local);
                            return (Some(local_dt.format("%Y-%m-%d %H:%M:%S").to_string()), Some(hash));
                        }
                        None => {
                            debug!("🕰️ Failed to create DateTime<Utc> from timestamp: {}", timestamp_secs);
                            return (Some("Invalid date".to_string()), Some(hash));
                        }
                    }
                }
                debug!("🕰️ Failed to parse timestamp string: {}", timestamp_str);
                (None, Some(hash)) // Return hash even if date parsing fails
            } else {
                debug!("🕰️ Unexpected format from git log output: {}", out_str);
                (None, None)
            }
        }
        Ok(output) => {
            debug!("🔍 Git log command for ref '{}' in '{}' failed or returned no info. Exit_code: {:?}, stdout: {}, stderr: {}", ref_name, repo_path.display(), output.status.code(), String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
            (None,None)
        }
        Err(e) => {
            debug!("🔍 Failed to execute git log for ref '{}' in '{}': {}", ref_name, repo_path.display(), e);
            (None, None)
        },
    }
}


pub fn run(args: &InfoArgs, remote_name: &str, global_store_name: &str, verbose: bool) {
    info!("🐘 Git Trunk Information");

    // Get repository root
    debug!("➡️ Getting repository root");
    let repo_root_output = run_git_command(Command::new("git").arg("rev-parse").arg("--show-toplevel"), verbose)
        .unwrap_or_else(|e| { error!("❌ Failed to get git repository root: {}", e); exit(1); });
    let repo_root_str = String::from_utf8_lossy(&repo_root_output.stdout).trim().to_string();
    if repo_root_str.is_empty() { error!("❌ Git repository root is empty."); exit(1); }
    let repo_root = PathBuf::from(repo_root_str);
    debug!("✓ Repository root found at {}", repo_root.display());

    let trunk_base_dir = repo_root.join(".trunk");
    let mut stores_to_check: Vec<String> = Vec::new();

    if args.all {
        println!("\n🌳 Git Trunk Stores Overview (Remote: '{}', Mode: All Remote Stores)", remote_name);
        println!("{:-<100}", "");
        debug!("➡️ --all specified, discovering stores from remote '{}'", remote_name);
        match run_git_command(
            Command::new("git")
                .arg("ls-remote")
                .arg("--refs")
                .arg(remote_name)
                .arg("refs/trunk/*"), // Pattern to match all refs under refs/trunk/
            verbose) {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.trim().is_empty() {
                    info!("ℹ️ No remote refs found under 'refs/trunk/' on remote '{}'.", remote_name);
                    return;
                }
                for line in output_str.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let ref_name_full = parts[1]; // e.g., refs/trunk/main
                        if let Some(store_name_from_ref) = ref_name_full.strip_prefix("refs/trunk/") {
                            // Ensure it's a direct child, not refs/trunk/foo/bar
                            if !store_name_from_ref.is_empty() && !store_name_from_ref.contains('/') {
                                 if !stores_to_check.contains(&store_name_from_ref.to_string()){
                                    stores_to_check.push(store_name_from_ref.to_string());
                                 }
                            }
                        }
                    }
                }
                 if stores_to_check.is_empty() {
                    info!("ℹ️ No valid store names parsed from 'refs/trunk/*' on remote '{}'.", remote_name);
                    return;
                }
            }
            Ok(output) => { // ls-remote succeeded but no refs, or other non-zero exit
                info!("ℹ️ No remote refs found under 'refs/trunk/' on remote '{}' (or command failed, exit code: {:?}).", remote_name, output.status.code());
                debug!("ls-remote stdout: {}", String::from_utf8_lossy(&output.stdout));
                debug!("ls-remote stderr: {}", String::from_utf8_lossy(&output.stderr));
                return;
            }
            Err(e) => {
                error!("❌ Failed to execute 'git ls-remote' for remote '{}': {}", remote_name, e);
                return;
            }
        }
    } else { // Not --all, use local discovery or specified global_store_name
        println!("\n🌳 Git Trunk Stores Overview (Remote: '{}')", remote_name);
        println!("{:-<100}", "");

        if global_store_name != "main" { // User explicitly specified a store via global --store
            debug!("➡️ Using explicitly specified store: {}", global_store_name);
            stores_to_check.push(global_store_name.to_string());
        } else { // Default "main" store or explicitly --store main: discover local stores
            debug!("➡️ Discovering local stores (defaulting to check 'main')");
            // Discover stores from .trunk directory
            if trunk_base_dir.exists() && trunk_base_dir.is_dir() {
                match fs::read_dir(&trunk_base_dir) {
                    Ok(entries) => {
                        for entry in entries.filter_map(Result::ok) {
                            if entry.path().is_dir() {
                                if let Some(s_name) = entry.file_name().to_str() {
                                    if !stores_to_check.contains(&s_name.to_string()) {
                                        stores_to_check.push(s_name.to_string());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => { error!("❌ Could not read .trunk directory: {}", e); }
                }
            }
            // Discover stores from refs/trunk/ in main repo
            match run_git_command(Command::new("git").arg("for-each-ref").arg("--format=%(refname:short)").arg("refs/trunk/").current_dir(&repo_root), verbose) {
                Ok(output) if output.status.success() => {
                    String::from_utf8_lossy(&output.stdout).lines().for_each(|line| {
                        if let Some(name) = line.strip_prefix("trunk/") {
                             // Ensure it's a direct child, not trunk/foo/bar
                            if !name.is_empty() && !name.contains('/') {
                                if !stores_to_check.contains(&name.to_string()){
                                    stores_to_check.push(name.to_string());
                                }
                            }
                        }
                    });
                }
                _ => { /* Ignore error, refs might not exist */ }
            }
            // Ensure "main" is checked if it's the target, even if not found locally yet (might be on remote)
            if !stores_to_check.contains(&"main".to_string()) {
                stores_to_check.push("main".to_string());
            }
        }
    }

    stores_to_check.sort();
    stores_to_check.dedup();

    if stores_to_check.is_empty() {
        if args.all {
             info!("ℹ️ No git-trunk stores found on remote '{}' under refs/trunk/.", remote_name);
        } else {
             info!("ℹ️ No git-trunk stores found or specified locally for store '{}'.", global_store_name);
        }
        return;
    }
    
    // The header print was moved up into the if/else args.all block.

    for store_name in stores_to_check {
        debug!("➡️ Processing store: {}", store_name);
        let mut store_info = StoreInfo {
            name: store_name.clone(),
            local_path: trunk_base_dir.join(&store_name),
            local_path_exists: false,
            is_git_repo: false,
            local_store_last_commit_date: None,
            local_store_last_commit_hash: None,
            local_store_uncommitted_changes: None,
            main_repo_ref: format!("refs/trunk/{}", store_name),
            main_repo_ref_exists: false,
            main_repo_ref_commit_date: None,
            main_repo_ref_commit_hash: None,
            remote_repo_ref_exists: None,
            remote_repo_ref_commit_hash: None,
        };

        store_info.local_path_exists = store_info.local_path.exists() && store_info.local_path.is_dir();

        if store_info.local_path_exists {
            store_info.is_git_repo = store_info.local_path.join(".git").exists();
            if store_info.is_git_repo {
                let (date, hash) = get_commit_info(&store_info.local_path, "HEAD", verbose);
                store_info.local_store_last_commit_date = date;
                store_info.local_store_last_commit_hash = hash;

                match run_git_command(Command::new("git").arg("status").arg("--porcelain").current_dir(&store_info.local_path), verbose) {
                    Ok(output) if output.status.success() => {
                        if output.stdout.is_empty() {
                            store_info.local_store_uncommitted_changes = Some("Clean".to_string());
                        } else {
                            let count = String::from_utf8_lossy(&output.stdout).lines().count();
                            store_info.local_store_uncommitted_changes = Some(format!("{} uncommitted change(s)", count));
                        }
                    }
                    _ => store_info.local_store_uncommitted_changes = Some("Status check failed".to_string()),
                }
            }
        }

        store_info.main_repo_ref_exists = run_git_command(Command::new("git").arg("rev-parse").arg("--verify").arg(&store_info.main_repo_ref).current_dir(&repo_root), verbose)
            .map_or(false, |out| out.status.success());
        
        if store_info.main_repo_ref_exists {
            let (date, hash) = get_commit_info(&repo_root, &store_info.main_repo_ref, verbose);
            store_info.main_repo_ref_commit_date = date;
            store_info.main_repo_ref_commit_hash = hash;
        }

        match run_git_command(Command::new("git").arg("ls-remote").arg(remote_name).arg(&store_info.main_repo_ref).current_dir(&repo_root), verbose) {
            Ok(output) => {
                if output.status.success() && !output.stdout.is_empty() {
                    store_info.remote_repo_ref_exists = Some(true);
                    let remote_out = String::from_utf8_lossy(&output.stdout);
                    store_info.remote_repo_ref_commit_hash = remote_out.split_whitespace().next().map(|s| s[0..7].to_string()); // Take first 7 chars of hash
                } else {
                    store_info.remote_repo_ref_exists = Some(false);
                }
            }
            Err(e) => {
                debug!("⚠️ Failed to check remote ref for store {}: {}", store_name, e);
                store_info.remote_repo_ref_exists = None; // Indicate check failed
            }
        }
        
        // Presentation
        println!("\nStore: {}", store_info.name);
        println!("  Local Directory (.trunk/{})", store_info.name);
        println!("    Exists: {}", if store_info.local_path_exists { "✓ Yes" } else { "❌ No" });
        if store_info.local_path_exists {
            println!("    Is Git Repo: {}", if store_info.is_git_repo { "✓ Yes" } else { "❌ No" });
            if store_info.is_git_repo {
                println!("    Last Commit: {} ({})",
                    store_info.local_store_last_commit_date.as_deref().unwrap_or("N/A"),
                    store_info.local_store_last_commit_hash.as_deref().unwrap_or("N/A"));
                println!("    Status: {}", store_info.local_store_uncommitted_changes.as_deref().unwrap_or("N/A"));
            }
        }
        println!("  Main Repository Ref (refs/trunk/{})", store_info.name);
        println!("    Exists Locally: {}", if store_info.main_repo_ref_exists { "✓ Yes" } else { "❌ No" });
        if store_info.main_repo_ref_exists {
             println!("    Last Commit: {} ({})",
                store_info.main_repo_ref_commit_date.as_deref().unwrap_or("N/A"),
                store_info.main_repo_ref_commit_hash.as_deref().unwrap_or("N/A"));
        }
        println!("  Remote '{}' Ref (refs/trunk/{})", remote_name, store_info.name);
        match store_info.remote_repo_ref_exists {
            Some(true) => println!("    Exists on Remote: ✓ Yes (Hash: {})", store_info.remote_repo_ref_commit_hash.as_deref().unwrap_or("N/A")),
            Some(false) => println!("    Exists on Remote: ❌ No"),
            None => println!("    Exists on Remote: ❓ Check failed"),
        }
        println!("{:-<100}", "");

    }
}