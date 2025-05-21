#!/bin/bash

set -euo pipefail

# Args are TEST_SUITE_DIR and GIT_TRUNK_CMD
# This script's specific test dir will be a subdir of TEST_SUITE_DIR

# Helper function for assertions
assert_success() {
  if [ "$?" -ne 0 ]; then
    echo "ERROR: Command failed: $*"
    exit 1
  fi
  echo "SUCCESS: $*"
}

assert_failure() {
  if [ "$?" -eq 0 ]; then
    echo "ERROR: Command succeeded but was expected to fail: $*"
    exit 1
  fi
  echo "SUCCESS: Command failed as expected: $*"
}

assert_dir_exists() {
  if [ ! -d "$1" ]; then
    echo "ERROR: Directory '$1' does not exist."
    exit 1
  fi
  echo "SUCCESS: Directory '$1' exists."
}

assert_file_exists() {
  if [ ! -f "$1" ]; then
    echo "ERROR: File '$1' does not exist."
    exit 1
  fi
  echo "SUCCESS: File '$1' exists."
}

assert_grep() {
  local pattern="$1"
  local file="$2"
  local description="${3:-Pattern '$pattern' in file '$file'}"
  if ! grep -qFx "$pattern" "$file"; then # -F for fixed string, -x for whole line match
    echo "ERROR: Pattern '$pattern' not found as a whole line in file '$file'."
    echo "File content:"
    cat "$file"
    exit 1
  fi
  echo "SUCCESS: $description found."
}

# Main test function
test_collaborative_workflow() {
  local test_suite_dir="$1" # The general test_suites directory
  local git_trunk_cmd="$2"
  local scenario_dir="$test_suite_dir/full_flow_scenario"

  echo "INFO: Starting test_collaborative_workflow in $scenario_dir..."
  rm -rf "$scenario_dir" && mkdir -p "$scenario_dir"

  local repo_a_path="$scenario_dir/repo_a"
  local repo_b_path="$scenario_dir/repo_b"
  local remote_repo_git_path="$scenario_dir/remote_repo.git" # Bare repo

  # 1. Setup
  echo "INFO: Setting up repositories..."
  mkdir -p "$repo_a_path"
  mkdir -p "$repo_b_path"
  mkdir -p "$remote_repo_git_path" && git init --bare "$remote_repo_git_path" > /dev/null
  assert_success "Bare remote repository initialized at $remote_repo_git_path"

  # --- Repo A - Initial work ---
  echo "INFO: === Repo A: Initial work ==="
  cd "$repo_a_path"
  git init -b main > /dev/null
  git config user.name "RepoA User" && git config user.email "repo_a@example.com"
  assert_success "Repo A initialized and configured at $(pwd)"

  "$git_trunk_cmd" init --store store1
  assert_success "Repo A: $git_trunk_cmd init --store store1"

  local initial_content="Initial content from repo_a"
  echo "$initial_content" > .trunk/store1/data.txt
  assert_file_exists ".trunk/store1/data.txt"
  echo "INFO: Repo A: Created .trunk/store1/data.txt with initial content."

  yes | "$git_trunk_cmd" commit --store store1 # --force not needed if 'yes' is piped
  assert_success "Repo A: $git_trunk_cmd commit --store store1"

  git remote add origin ../remote_repo.git # Relative path from repo_a to remote_repo.git
  assert_success "Repo A: Added remote 'origin' -> ../remote_repo.git"

  "$git_trunk_cmd" push --store store1 --remote origin
  assert_success "Repo A: $git_trunk_cmd push --store store1 --remote origin"
  echo "INFO: Repo A: Initial content pushed to remote."

  # --- Repo B - Clone and update ---
  echo "INFO: === Repo B: Clone and update ==="
  cd "$repo_b_path"
  git init -b main > /dev/null
  git config user.name "RepoB User" && git config user.email "repo_b@example.com"
  assert_success "Repo B initialized and configured at $(pwd)"

  git remote add origin ../remote_repo.git # Relative path from repo_b to remote_repo.git
  assert_success "Repo B: Added remote 'origin' -> ../remote_repo.git"

  "$git_trunk_cmd" checkout --store store1 --remote origin
  assert_success "Repo B: $git_trunk_cmd checkout --store store1 --remote origin"

  assert_file_exists ".trunk/store1/data.txt"
  assert_grep "$initial_content" ".trunk/store1/data.txt" "Repo B: Verified initial content after checkout"
  echo "INFO: Repo B: Verified initial content from Repo A."

  local updated_content_repo_b="Updated content from repo_b"
  echo "$updated_content_repo_b" > .trunk/store1/data.txt
  echo "INFO: Repo B: Modified .trunk/store1/data.txt with new content."

  yes | "$git_trunk_cmd" commit --store store1
  assert_success "Repo B: $git_trunk_cmd commit --store store1"

  "$git_trunk_cmd" push --store store1 --remote origin
  assert_success "Repo B: $git_trunk_cmd push --store store1 --remote origin"
  echo "INFO: Repo B: Updated content pushed to remote."

  # --- Repo A - Pull updates ---
  echo "INFO: === Repo A: Pull updates ==="
  cd "$repo_a_path"
  echo "INFO: Repo A: Currently in $(pwd)"

  # Before checkout, ensure local store content is still the initial one
  assert_grep "$initial_content" ".trunk/store1/data.txt" "Repo A: Verified initial content before pulling updates"


  "$git_trunk_cmd" checkout --store store1 --remote origin --force
  assert_success "Repo A: $git_trunk_cmd checkout --store store1 --remote origin --force"

  assert_file_exists ".trunk/store1/data.txt"
  assert_grep "$updated_content_repo_b" ".trunk/store1/data.txt" "Repo A: Verified updated content from Repo B after checkout"
  echo "INFO: Repo A: Verified updated content from Repo B."

  echo "INFO: test_collaborative_workflow PASSED"
}

# Main script execution
# The actual TEST_DIR and GIT_TRUNK_CMD are passed by the run_tests.sh script
# For standalone testing, you could set defaults:
# TEST_SUITE_DIR_ARG="${1:-./tmp_tests}" # Default to a local temp dir for tests
# GIT_TRUNK_CMD_ARG="${2:-git-trunk}"   # Default to git-trunk if in PATH

# mkdir -p "$TEST_SUITE_DIR_ARG" # Ensure base test dir exists if running standalone

test_collaborative_workflow "$1" "$2"

exit 0
