use clap::Parser;
use std::process::{Command, exit};
use log::{debug, error, info};
use crate::utils::run_git_command; // Ensure this line is present

#[derive(Parser, Debug)]
#[command(about = "Push refs/trunk/<store> to the specified remote")]
pub struct PushArgs {
    // Remote is now a global option, remove from here
    // store is now a global option, remove from here if it was ever considered locally
}

pub fn run(_args: &PushArgs, remote_name: &str, store_name: &str, verbose: bool) {
    let trunk_ref_name = format!("refs/trunk/{}", store_name);

    // Step 1: Verify that refs/trunk/<store_name> exists locally
    debug!("➡️ Step 1: Checking if {} exists locally for store '{}'", trunk_ref_name, store_name);
    let show_ref = run_git_command(
        Command::new("git")
            .args(["show-ref", "--quiet", &trunk_ref_name]),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("❌ Failed to check {}: {}", trunk_ref_name, e);
        exit(1);
    });

    if !show_ref.status.success() {
        error!("❌ {} for store '{}' does not exist in the local repository. Commit changes first using `git trunk commit --store {}`.", trunk_ref_name, store_name, store_name);
        exit(1);
    }
    info!("✓ Step 1: {} found locally for store '{}'", trunk_ref_name, store_name);

    // Step 2: Push refs/trunk/<store_name> to the remote
    debug!("📤 Step 2: Pushing {} for store '{}' to remote '{}'", trunk_ref_name, store_name, remote_name);
    let refspec = format!("{}:{}", trunk_ref_name, trunk_ref_name);
    let push_status = run_git_command(
        Command::new("git")
            .args([
                "push",
                remote_name,
                &refspec,
            ]),
        verbose,
    )
    .unwrap_or_else(|e| {
        error!("❌ Failed to execute git push for store '{}' to remote '{}': {}", store_name, remote_name, e);
        exit(1);
    })
    .status;

    if !push_status.success() {
        error!("❌ Failed to push {} for store '{}' to remote '{}'", trunk_ref_name, store_name, remote_name);
        exit(1);
    }

    info!("✓ Step 2: Successfully pushed {} for store '{}' to remote '{}'", trunk_ref_name, store_name, remote_name);
    info!("✅ Trunk store '{}' pushed successfully", store_name);
}