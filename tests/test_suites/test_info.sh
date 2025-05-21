#!/bin/bash

set -euo pipefail

TEST_DIR="$1"
GIT_TRUNK_CMD="$2"

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

assert_output_contains() {
  local output="$1"
  local pattern="$2"
  local description="${3:-Pattern '$pattern' in output}"
  if ! echo "$output" | grep -qE "$pattern"; then # Use -E for extended regex if needed
    echo "ERROR: Pattern '$pattern' not found in output."
    echo "Output was:"
    echo "$output"
    exit 1
  fi
  echo "SUCCESS: $description found in output."
}

assert_output_does_not_contain() {
  local output="$1"
  local pattern="$2"
  local description="${3:-Pattern '$pattern' NOT in output}"
  if echo "$output" | grep -qE "$pattern"; then
    echo "ERROR: Pattern '$pattern' was found in output but was not expected."
    echo "Output was:"
    echo "$output"
    exit 1
  fi
  echo "SUCCESS: $description not found in output, as expected."
}

assert_ref_exists() {
  local repo_path="$1"
  local ref_name="$2"
  if ! git -C "$repo_path" show-ref --verify --quiet "$ref_name"; then
    echo "ERROR: Ref '$ref_name' does not exist in repo '$repo_path'."
    exit 1
  fi
  echo "SUCCESS: Ref '$ref_name' exists in repo '$repo_path'."
}

get_commit_hash() {
  git -C "$1" rev-parse "$2" 2>/dev/null || echo "INVALID_REF"
}

# Test scenarios

test_info_single_synchronized_store() {
  echo "INFO: Starting test_info_single_synchronized_store..."
  local test_env_dir="$TEST_DIR/info_single_sync"
  rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

  local main_repo_path="$test_env_dir/main_repo"
  local remote_repo_git_path="$test_env_dir/remote_repo.git"

  mkdir -p "$remote_repo_git_path" && git init --bare "$remote_repo_git_path" > /dev/null
  echo "INFO: Initialized bare remote repo at $remote_repo_git_path"

  mkdir -p "$main_repo_path" && cd "$main_repo_path"
  git init -b main > /dev/null
  git remote add origin "$remote_repo_git_path"
  echo "INFO: Initialized main_repo at $(pwd)"

  "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"
  echo "sync content" > .trunk/main/data.txt
  "$GIT_TRUNK_CMD" commit --force -m "Initial commit"
  assert_success "$GIT_TRUNK_CMD commit --force"
  "$GIT_TRUNK_CMD" push
  assert_success "$GIT_TRUNK_CMD push"
  "$GIT_TRUNK_CMD" checkout
  assert_success "$GIT_TRUNK_CMD checkout"

  local local_ref_hash
  local_ref_hash=$(get_commit_hash "." "refs/trunk/main")
  local remote_ref_hash # Fetched by info command or we can check manually
  
  # Action
  local output
  output=$("$GIT_TRUNK_CMD" info)
  assert_success "$GIT_TRUNK_CMD info"
  echo "$output" # Print for debugging

  # Verify
  assert_output_contains "$output" "Store: main"
  assert_output_contains "$output" "Local Directory: .trunk/main (Exists)"
  assert_output_contains "$output" "Local Ref: refs/trunk/main ($local_ref_hash)"
  assert_output_contains "$output" "Remote Ref (origin): refs/trunk/main ($local_ref_hash)" # Expect same hash
  assert_output_contains "$output" "Status: Synchronized" "Output contains 'Status: Synchronized'"
  assert_output_does_not_contain "$output" "Uncommitted changes"

  echo "INFO: test_info_single_synchronized_store PASSED"
}

test_info_uncommitted_changes() {
  echo "INFO: Starting test_info_uncommitted_changes..."
  local test_subdir="$TEST_DIR/info_uncommitted"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"
  "$GIT_TRUNK_CMD" checkout # Ensure .trunk/main exists
  assert_success "$GIT_TRUNK_CMD checkout"
  echo "This is an uncommitted change" > .trunk/main/readme.md
  echo "INFO: Modified .trunk/main/readme.md"

  # Action
  local output
  output=$("$GIT_TRUNK_CMD" info)
  assert_success "$GIT_TRUNK_CMD info"
  echo "$output"

  # Verify
  assert_output_contains "$output" "Store: main"
  assert_output_contains "$output" "Local Directory: .trunk/main (Exists)"
  assert_output_contains "$output" "Uncommitted changes detected in .trunk/main"

  echo "INFO: test_info_uncommitted_changes PASSED"
}

test_info_local_ahead_of_remote() {
  echo "INFO: Starting test_info_local_ahead_of_remote..."
  local test_env_dir="$TEST_DIR/info_local_ahead"
  rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

  local main_repo_path="$test_env_dir/main_repo"
  local remote_repo_git_path="$test_env_dir/remote_repo.git"

  mkdir -p "$remote_repo_git_path" && git init --bare "$remote_repo_git_path" > /dev/null
  mkdir -p "$main_repo_path" && cd "$main_repo_path"
  git init -b main > /dev/null
  git remote add origin "$remote_repo_git_path"

  "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"
  echo "initial content" > .trunk/main/data.txt
  "$GIT_TRUNK_CMD" commit --force -m "Initial commit"
  assert_success "$GIT_TRUNK_CMD commit --force (1)"
  "$GIT_TRUNK_CMD" push
  assert_success "$GIT_TRUNK_CMD push"
  "$GIT_TRUNK_CMD" checkout
  assert_success "$GIT_TRUNK_CMD checkout"

  local remote_commit_hash
  remote_commit_hash=$(get_commit_hash "." "refs/trunk/main") # This is also the initial local hash

  echo "new content" >> .trunk/main/data.txt
  "$GIT_TRUNK_CMD" commit --force -m "Second commit, local only"
  assert_success "$GIT_TRUNK_CMD commit --force (2)"
  local local_commit_hash
  local_commit_hash=$(get_commit_hash "." "refs/trunk/main")

  if [ "$local_commit_hash" == "$remote_commit_hash" ]; then
    echo "ERROR: Local and remote hashes are the same after second commit, cannot test 'ahead' state."
    exit 1
  fi
  echo "INFO: Local hash $local_commit_hash, Remote hash $remote_commit_hash"

  # Action
  local output
  output=$("$GIT_TRUNK_CMD" info)
  assert_success "$GIT_TRUNK_CMD info"
  echo "$output"

  # Verify
  assert_output_contains "$output" "Store: main"
  assert_output_contains "$output" "Local Ref: refs/trunk/main ($local_commit_hash)"
  assert_output_contains "$output" "Remote Ref (origin): refs/trunk/main ($remote_commit_hash)"
  assert_output_contains "$output" "Status: Local ref is ahead of remote" "Output contains 'Status: Local ref is ahead of remote'"

  echo "INFO: test_info_local_ahead_of_remote PASSED"
}

test_info_store_remote_only() {
  echo "INFO: Starting test_info_store_remote_only..."
  local test_env_dir="$TEST_DIR/info_remote_only"
  rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

  local repo_A_path="$test_env_dir/repo_A"
  local repo_B_path="$test_env_dir/repo_B"
  local remote_repo_git_path="$test_env_dir/remote_repo.git"
  local store_name="shared_store"

  mkdir -p "$remote_repo_git_path" && git init --bare "$remote_repo_git_path" > /dev/null

  # Setup in repo_A
  mkdir -p "$repo_A_path" && cd "$repo_A_path"
  git init -b main > /dev/null
  git remote add origin "$remote_repo_git_path"
  "$GIT_TRUNK_CMD" init --store "$store_name"
  echo "remote data" > ".trunk/$store_name/data.txt"
  "$GIT_TRUNK_CMD" commit --force --store "$store_name" -m "Commit $store_name"
  "$GIT_TRUNK_CMD" push --store "$store_name"
  local remote_ref_hash
  remote_ref_hash=$(git -C "$remote_repo_git_path" rev-parse "refs/trunk/$store_name")
  echo "INFO: repo_A setup complete, pushed $store_name ($remote_ref_hash)"
  cd ..

  # Setup in repo_B
  mkdir -p "$repo_B_path" && cd "$repo_B_path"
  git init -b main > /dev/null
  git remote add origin "$remote_repo_git_path"
  git fetch origin # Fetch to make git aware of remote refs for `info` command
  echo "INFO: repo_B setup complete at $(pwd)"

  # Action
  local output
  output=$("$GIT_TRUNK_CMD" info --store "$store_name")
  assert_success "$GIT_TRUNK_CMD info --store $store_name"
  echo "$output"

  # Verify
  assert_output_contains "$output" "Store: $store_name"
  assert_output_contains "$output" "Local Directory: .trunk/$store_name (Not checked out)"
  assert_output_contains "$output" "Local Ref: refs/trunk/$store_name (Not found)"
  assert_output_contains "$output" "Remote Ref (origin): refs/trunk/$store_name ($remote_ref_hash)"
  assert_output_contains "$output" "Status: Remote only" "Output contains 'Status: Remote only'"


  echo "INFO: test_info_store_remote_only PASSED"
}

test_info_all_mixed_states() {
  echo "INFO: Starting test_info_all_mixed_states..."
  local test_env_dir="$TEST_DIR/info_all_mixed"
  rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

  local main_repo_path="$test_env_dir/main_repo"
  local remote_repo_git_path="$test_env_dir/remote_repo.git"

  mkdir -p "$remote_repo_git_path" && git init --bare "$remote_repo_git_path" > /dev/null
  mkdir -p "$main_repo_path" && cd "$main_repo_path"
  git init -b main > /dev/null
  git remote add origin "$remote_repo_git_path"

  # Store 'main': synchronized
  "$GIT_TRUNK_CMD" init --store main
  echo "main content" > .trunk/main/data.txt
  "$GIT_TRUNK_CMD" commit --force --store main -m "main commit" && "$GIT_TRUNK_CMD" push --store main && "$GIT_TRUNK_CMD" checkout --store main
  assert_success "Setup store 'main' (synchronized)"

  # Store 'docs': local changes, not committed to its trunk ref
  "$GIT_TRUNK_CMD" init --store docs
  echo "docs initial" > .trunk/docs/index.md
  "$GIT_TRUNK_CMD" commit --force --store docs -m "docs initial commit" && "$GIT_TRUNK_CMD" push --store docs && "$GIT_TRUNK_CMD" checkout --store docs
  echo "docs local changes" >> .trunk/docs/index.md # Uncommitted changes in working dir
  assert_success "Setup store 'docs' (local changes)"

  # Store 'assets': remote only (pushed from main_repo, then 'stegano' or never checked out)
  "$GIT_TRUNK_CMD" init --store assets
  echo "assets content" > .trunk/assets/img.dat
  "$GIT_TRUNK_CMD" commit --force --store assets -m "assets commit" && "$GIT_TRUNK_CMD" push --store assets
  # To make it remote only for this test, we can remove the local copy after push
  "$GIT_TRUNK_CMD" stegano --store assets # or just rm -rf .trunk/assets and delete local ref
  assert_success "Setup store 'assets' (remote only)"
  
  # Store 'libs': local ref ahead of remote
  "$GIT_TRUNK_CMD" init --store libs
  echo "libs v1" > .trunk/libs/lib.c
  "$GIT_TRUNK_CMD" commit --force --store libs -m "libs v1" && "$GIT_TRUNK_CMD" push --store libs && "$GIT_TRUNK_CMD" checkout --store libs
  echo "libs v2" >> .trunk/libs/lib.c
  "$GIT_TRUNK_CMD" commit --force --store libs -m "libs v2" # Not pushed
  assert_success "Setup store 'libs' (local ahead)"


  # Action
  local output
  output=$("$GIT_TRUNK_CMD" info --all)
  assert_success "$GIT_TRUNK_CMD info --all"
  echo "$output"

  # Verify (flexible checks as order might vary)
  assert_output_contains "$output" "Store: main"
  assert_output_contains "$output" "Status: Synchronized" # For main
  
  assert_output_contains "$output" "Store: docs"
  assert_output_contains "$output" "Uncommitted changes detected in .trunk/docs" # For docs

  assert_output_contains "$output" "Store: assets"
  # Depending on how 'remote only' is determined, could be 'Not checked out' or 'Remote only'
  assert_output_contains "$output" "Status: Remote only|Local Directory: .trunk/assets (Not checked out)" # For assets

  assert_output_contains "$output" "Store: libs"
  assert_output_contains "$output" "Status: Local ref is ahead of remote" # For libs

  echo "INFO: test_info_all_mixed_states PASSED"
}

test_info_non_existent_store() {
  echo "INFO: Starting test_info_non_existent_store..."
  local test_subdir="$TEST_DIR/info_non_existent"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  echo "INFO: Initialized clean Git repo in $(pwd)"

  # Action
  local output
  # The tool might exit non-zero for "not found", or zero with specific message.
  # Based on "exit cleanly", assuming exit 0.
  output=$("$GIT_TRUNK_CMD" info --store does_not_exist)
  assert_success "$GIT_TRUNK_CMD info --store does_not_exist" # Check exit code
  echo "$output"

  # Verify
  assert_output_contains "$output" "Store: does_not_exist"
  assert_output_contains "$output" "Status: Not found|Store 'does_not_exist' not found" "Output indicates store not found"

  echo "INFO: test_info_non_existent_store PASSED"
}

# Main execution
main() {
  echo "INFO: Running test_info.sh..."
  test_info_single_synchronized_store
  test_info_uncommitted_changes
  test_info_local_ahead_of_remote
  test_info_store_remote_only
  test_info_all_mixed_states
  test_info_non_existent_store
  echo "INFO: All tests in test_info.sh PASSED"
  exit 0
}

main
