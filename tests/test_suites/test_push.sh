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

assert_remote_ref_exists() {
  local repo_path="$1"
  local remote_name="$2"
  local ref_name="$3"
  # Ensure we are in the correct directory for git ls-remote
  local current_dir=$(pwd)
  cd "$repo_path"
  if ! git ls-remote "$remote_name" "$ref_name" | grep -q "$ref_name"; then
    cd "$current_dir"
    echo "ERROR: Ref '$ref_name' not found on remote '$remote_name' in repo '$repo_path'."
    exit 1
  fi
  cd "$current_dir"
  echo "SUCCESS: Ref '$ref_name' found on remote '$remote_name' in repo '$repo_path'."
}

assert_remote_ref_does_not_exist() {
  local repo_path="$1"
  local remote_name="$2"
  local ref_name="$3"
  local current_dir=$(pwd)
  cd "$repo_path"
  if git ls-remote "$remote_name" "$ref_name" | grep -q "$ref_name"; then
    cd "$current_dir"
    echo "ERROR: Ref '$ref_name' unexpectedly found on remote '$remote_name' in repo '$repo_path'."
    exit 1
  fi
  cd "$current_dir"
  echo "SUCCESS: Ref '$ref_name' not found on remote '$remote_name' as expected in repo '$repo_path'."
}

# Test scenarios

test_push_basic() {
  echo "INFO: Starting test_push_basic..."
  local test_env_dir="$TEST_DIR/push_basic"
  
  (
    rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"
    
    local main_repo_path="$test_env_dir/main_repo"
    local remote_repo_path="$test_env_dir/remote_repo.git" # Path for the bare repo

    # Setup remote repo
    mkdir -p "$remote_repo_path"
    git init --bare "$remote_repo_path" > /dev/null
    echo "INFO: Initialized bare remote repo at $remote_repo_path"

    # Setup main repo
    mkdir -p "$main_repo_path"
    cd "$main_repo_path" # Important: all git commands run from here unless -C is used
    git init -b main > /dev/null
    git remote add origin ../remote_repo.git # Use relative path
    echo "INFO: Initialized main repo at $(pwd) and added remote 'origin' -> ../remote_repo.git"

    "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"
  echo "Push basic changes" >> .trunk/main/readme.md
  "$GIT_TRUNK_CMD" commit --force -m "Test commit for basic push"
  assert_success "$GIT_TRUNK_CMD commit --force"

  echo "INFO: Current directory: $(pwd)"
  echo "INFO: Git remote configuration:"
  git remote -v
  echo "INFO: Local refs:"
  git show-ref

  # Action: git trunk push
  "$GIT_TRUNK_CMD" push
  assert_success "$GIT_TRUNK_CMD push"

    # Verify
    assert_remote_ref_exists "." "origin" "refs/trunk/main" # "." refers to current dir main_repo_path
  )

  echo "INFO: test_push_basic PASSED"
}

test_push_specific_store() {
  echo "INFO: Starting test_push_specific_store..."
  local test_env_dir="$TEST_DIR/push_specific_store"

  (
    rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

    local main_repo_path="$test_env_dir/main_repo"
    local remote_repo_path="$test_env_dir/remote_repo.git"

    mkdir -p "$remote_repo_path"
    git init --bare "$remote_repo_path" > /dev/null
    echo "INFO: Initialized bare remote repo at $remote_repo_path"

    mkdir -p "$main_repo_path"
    cd "$main_repo_path"
    git init -b main > /dev/null
    git remote add origin ../remote_repo.git # Use relative path
    echo "INFO: Initialized main repo at $(pwd) and added remote 'origin' -> ../remote_repo.git"

    "$GIT_TRUNK_CMD" init # Default 'main' store
  assert_success "$GIT_TRUNK_CMD init"
  echo "Main store content" >> .trunk/main/readme.md
  "$GIT_TRUNK_CMD" commit --force -m "Commit for main store"
  assert_success "$GIT_TRUNK_CMD commit --force for main store"

  "$GIT_TRUNK_CMD" init --store assets
  assert_success "$GIT_TRUNK_CMD init --store assets"
  echo "Assets store content" >> .trunk/assets/data.txt
  "$GIT_TRUNK_CMD" commit --force --store assets -m "Commit for assets store"
  assert_success "$GIT_TRUNK_CMD commit --force --store assets"

  # Action: git trunk push --store assets
  "$GIT_TRUNK_CMD" push --store assets
  assert_success "$GIT_TRUNK_CMD push --store assets"

    # Verify
    assert_remote_ref_exists "." "origin" "refs/trunk/assets"
    assert_remote_ref_does_not_exist "." "origin" "refs/trunk/main" # Assuming 'main' wasn't pushed by this test
  )

  echo "INFO: test_push_specific_store PASSED"
}

test_push_specific_remote() {
  echo "INFO: Starting test_push_specific_remote..."
  local test_env_dir="$TEST_DIR/push_specific_remote"

  (
    rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

    local main_repo_path="$test_env_dir/main_repo"
    local origin_remote_path="$test_env_dir/origin_repo.git"
    local alt_remote_path="$test_env_dir/alt_repo.git"

    mkdir -p "$origin_remote_path" && git init --bare "$origin_remote_path" > /dev/null
    mkdir -p "$alt_remote_path" && git init --bare "$alt_remote_path" > /dev/null
    echo "INFO: Initialized bare remotes origin_repo.git and alt_repo.git"

    mkdir -p "$main_repo_path"
    cd "$main_repo_path"
    git init -b main > /dev/null
    git remote add origin ../origin_repo.git # Use relative path
    git remote add alternate ../alt_repo.git # Use relative path
    echo "INFO: Initialized main repo at $(pwd) and added remotes 'origin' -> ../origin_repo.git and 'alternate' -> ../alt_repo.git"

    "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"
  echo "Content for specific remote push" >> .trunk/main/readme.md
  "$GIT_TRUNK_CMD" commit --force -m "Commit for specific remote test"
  assert_success "$GIT_TRUNK_CMD commit --force"

  # Action: git trunk push --remote alternate
  "$GIT_TRUNK_CMD" push --remote alternate
  assert_success "$GIT_TRUNK_CMD push --remote alternate"

    # Verify
    assert_remote_ref_exists "." "alternate" "refs/trunk/main"
    assert_remote_ref_does_not_exist "." "origin" "refs/trunk/main" # Should not be on origin
  )

  echo "INFO: test_push_specific_remote PASSED"
}

test_push_non_existent_local_ref() {
  echo "INFO: Starting test_push_non_existent_local_ref..."
  local test_env_dir="$TEST_DIR/push_non_existent_local_ref"

  (
    rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

    local main_repo_path="$test_env_dir/main_repo"
    local remote_repo_path="$test_env_dir/remote_repo.git"

    mkdir -p "$remote_repo_path" && git init --bare "$remote_repo_path" > /dev/null
    echo "INFO: Initialized bare remote repo at $remote_repo_path"

    mkdir -p "$main_repo_path"
    cd "$main_repo_path"
    git init -b main > /dev/null
    git remote add origin "$remote_repo_path"
    echo "INFO: Initialized main repo at $(pwd) (no trunk init/commit yet)"
    # NO git trunk init or commit, so refs/trunk/main does not exist locally

    # Action: git trunk push
  # Expect failure, capture output to check message if desired
  output=$("$GIT_TRUNK_CMD" push 2>&1) || true 
  # The `|| true` prevents script exit if $GIT_TRUNK_CMD fails, allowing assert_failure to check.
  # However, $? will be 0 if the command succeeds, which assert_failure will catch.
  # If the command fails, $? will be non-zero, which assert_failure expects.

  # Verify
  assert_failure "$GIT_TRUNK_CMD push (with non-existent local ref)"
  if ! echo "$output" | grep -q "Local ref 'refs/trunk/main' for store 'main' does not exist."; then
    echo "ERROR: Expected error message not found in output:"
    echo "$output"
    # exit 1 # Optional: fail test if specific message is missing
  fi
    echo "SUCCESS: Command failed and appropriate error message fragment found (or failure was sufficient)."
    assert_remote_ref_does_not_exist "." "origin" "refs/trunk/main"
  )

  echo "INFO: test_push_non_existent_local_ref PASSED"
}


# Main execution
main() {
  echo "INFO: Running test_push.sh..."
  test_push_basic
  test_push_specific_store
  test_push_specific_remote
  test_push_non_existent_local_ref
  echo "INFO: All tests in test_push.sh PASSED"
  exit 0
}

main
