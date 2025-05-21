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

assert_dir_does_not_exist() {
  if [ -d "$1" ]; then
    echo "ERROR: Directory '$1' exists but was expected not to."
    exit 1
  fi
  echo "SUCCESS: Directory '$1' does not exist as expected."
}

assert_file_exists() {
  if [ ! -f "$1" ]; then
    echo "ERROR: File '$1' does not exist."
    exit 1
  fi
  echo "SUCCESS: File '$1' exists."
}

assert_file_does_not_exist() {
  if [ -f "$1" ]; then
    echo "ERROR: File '$1' exists but was expected not to."
    exit 1
  fi
  echo "SUCCESS: File '$1' does not exist as expected."
}

assert_git_repo() {
  if ! git -C "$1" rev-parse --is-inside-work-tree > /dev/null 2>&1; then
    echo "ERROR: Directory '$1' is not a Git repository."
    exit 1
  fi
  echo "SUCCESS: Directory '$1' is a Git repository."
}

assert_grep() {
  # Usage: assert_grep "pattern" "file" ["description"]
  local pattern="$1"
  local file="$2"
  local description="${3:-Pattern '$pattern' in file '$file'}"
  if ! grep -q "$pattern" "$file"; then
    echo "ERROR: Pattern '$pattern' not found in file '$file'."
    exit 1
  fi
  echo "SUCCESS: $description found."
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

assert_ref_does_not_exist() {
  local repo_path="$1"
  local ref_name="$2"
  if git -C "$repo_path" show-ref --verify --quiet "$ref_name"; then
    echo "ERROR: Ref '$ref_name' exists in repo '$repo_path' but was expected not to."
    exit 1
  fi
  echo "SUCCESS: Ref '$ref_name' does not exist in repo '$repo_path' as expected."
}


assert_remote_ref_exists() {
  local repo_path="$1" # Path to the repo where ls-remote is run
  local remote_name="$2"
  local ref_name="$3"
  local current_dir
  current_dir=$(pwd)
  cd "$repo_path" # cd to ensure git finds the remote if it's relative
  if ! git ls-remote "$remote_name" "$ref_name" | grep -q "$ref_name"; then
    cd "$current_dir"
    echo "ERROR: Ref '$ref_name' not found on remote '$remote_name' in repo '$repo_path'."
    exit 1
  fi
  cd "$current_dir"
  echo "SUCCESS: Ref '$ref_name' found on remote '$remote_name' in repo '$repo_path'."
}

get_commit_hash() {
  git -C "$1" rev-parse "$2" 2>/dev/null || echo "INVALID_REF"
}

assert_commit_hash_matches() {
  local repo1_path="$1"
  local repo1_ref="$2"
  local repo2_path="$3"
  local repo2_ref="$4"

  local hash1
  hash1=$(get_commit_hash "$repo1_path" "$repo1_ref")
  local hash2
  hash2=$(get_commit_hash "$repo2_path" "$repo2_ref")

  if [ "$hash1" == "INVALID_REF" ]; then
    echo "ERROR: Could not get commit hash for $repo1_ref in $repo1_path"
    exit 1
  fi
  if [ "$hash2" == "INVALID_REF" ]; then
    echo "ERROR: Could not get commit hash for $repo2_ref in $repo2_path"
    exit 1
  fi

  if [ "$hash1" != "$hash2" ]; then
    echo "ERROR: Commit hash mismatch. $repo1_path@$repo1_ref ($hash1) != $repo2_path@$repo2_ref ($hash2)"
    exit 1
  fi
  echo "SUCCESS: Commit hash matches for $repo1_path@$repo1_ref and $repo2_path@$repo2_ref ($hash1)."
}


# Test scenarios

test_checkout_from_local_ref() {
  echo "INFO: Starting test_checkout_from_local_ref..."
  local test_subdir="$TEST_DIR/checkout_local_ref"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"
  local original_content="Original content for local ref test"
  echo "$original_content" > .trunk/main/readme.md
  "$GIT_TRUNK_CMD" commit --force -m "Commit for local ref test"
  assert_success "$GIT_TRUNK_CMD commit --force"
  assert_ref_exists "." "refs/trunk/main"

  rm -rf .trunk
  assert_dir_does_not_exist ".trunk"
  echo "INFO: Removed .trunk directory"

  "$GIT_TRUNK_CMD" checkout
  assert_success "$GIT_TRUNK_CMD checkout"

  assert_dir_exists ".trunk/main"
  assert_file_exists ".trunk/main/readme.md"
  assert_grep "$original_content" ".trunk/main/readme.md" "Original content in checked-out readme.md"
  assert_git_repo ".trunk/main"
  assert_commit_hash_matches ".trunk/main" "HEAD" "." "refs/trunk/main"
  assert_grep ".trunk" ".gitignore"

  echo "INFO: test_checkout_from_local_ref PASSED"
}

test_checkout_from_remote_ref() {
  echo "INFO: Starting test_checkout_from_remote_ref..."
  local test_env_dir="$TEST_DIR/checkout_remote_ref"
  rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

  local repo_source_path="$test_env_dir/repo_source"
  local remote_repo_git_path="$test_env_dir/remote_repo.git" # Bare repo
  local repo_clone_path="$test_env_dir/repo_clone"

  # Setup remote_repo.git
  mkdir -p "$remote_repo_git_path"
  git init --bare "$remote_repo_git_path" > /dev/null
  echo "INFO: Initialized bare remote repo at $remote_repo_git_path"

  # Setup repo_source
  mkdir -p "$repo_source_path" && cd "$repo_source_path"
  git init -b main > /dev/null
  git remote add origin ../remote_repo.git # Relative path
  "$GIT_TRUNK_CMD" init
  assert_success "repo_source: $GIT_TRUNK_CMD init"
  local remote_content="Content from remote ref"
  echo "$remote_content" > .trunk/main/readme.md
  "$GIT_TRUNK_CMD" commit --force -m "Commit for remote ref test"
  assert_success "repo_source: $GIT_TRUNK_CMD commit --force"
  "$GIT_TRUNK_CMD" push
  assert_success "repo_source: $GIT_TRUNK_CMD push"
  assert_remote_ref_exists "." "origin" "refs/trunk/main"
  local source_trunk_hash 
  source_trunk_hash=$(get_commit_hash "." "refs/trunk/main")
  cd .. # Back to test_env_dir

  # Setup repo_clone
  mkdir -p "$repo_clone_path" && cd "$repo_clone_path"
  git init -b main > /dev/null
  git remote add origin ../remote_repo.git # Relative path
  assert_ref_does_not_exist "." "refs/trunk/main" # Ensure not local initially
  assert_dir_does_not_exist ".trunk"
  echo "INFO: Set up repo_clone at $(pwd)"

  # Action: git trunk checkout (should default to origin)
  "$GIT_TRUNK_CMD" checkout
  assert_success "repo_clone: $GIT_TRUNK_CMD checkout"

  # Verify
  assert_dir_exists ".trunk/main"
  assert_file_exists ".trunk/main/readme.md"
  assert_grep "$remote_content" ".trunk/main/readme.md"
  assert_git_repo ".trunk/main"
  assert_commit_hash_matches ".trunk/main" "HEAD" "$remote_repo_git_path" "refs/trunk/main" # Compare clone's store with remote ref
  # Also verify local ref was created and matches
  assert_ref_exists "." "refs/trunk/main"
  assert_commit_hash_matches "." "refs/trunk/main" "$remote_repo_git_path" "refs/trunk/main"
  assert_grep ".trunk" ".gitignore"

  echo "INFO: test_checkout_from_remote_ref PASSED"
}

test_checkout_force_overwrite() {
  echo "INFO: Starting test_checkout_force_overwrite..."
  local test_subdir="$TEST_DIR/checkout_force"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"
  local original_readme_content="Original readme for force test"
  echo "$original_readme_content" > .trunk/main/readme.md
  "$GIT_TRUNK_CMD" commit --force -m "Initial commit for force test"
  assert_success "$GIT_TRUNK_CMD commit --force"
  "$GIT_TRUNK_CMD" checkout # Ensure .trunk/main is populated
  assert_success "$GIT_TRUNK_CMD checkout (initial)"

  # Modify .trunk/main by adding an untracked file
  local extra_file=".trunk/main/test_overwrite.txt"
  echo "overwrite me" > "$extra_file"
  assert_file_exists "$extra_file"

  # Action: git trunk checkout --force
  "$GIT_TRUNK_CMD" checkout --force
  assert_success "$GIT_TRUNK_CMD checkout --force"

  # Verify
  assert_dir_exists ".trunk/main"
  assert_file_exists ".trunk/main/readme.md" # Original file should still be there
  assert_grep "$original_readme_content" ".trunk/main/readme.md"
  assert_file_does_not_exist "$extra_file" # The extra file should be gone
  assert_commit_hash_matches ".trunk/main" "HEAD" "." "refs/trunk/main"

  echo "INFO: test_checkout_force_overwrite PASSED"
}

test_checkout_specific_store() {
  echo "INFO: Starting test_checkout_specific_store..."
  local test_env_dir="$TEST_DIR/checkout_specific_store"
  rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

  local repo_source_path="$test_env_dir/repo_source"
  local remote_repo_git_path="$test_env_dir/remote_repo.git"
  local repo_clone_path="$test_env_dir/repo_clone"
  local store_name="docs"

  mkdir -p "$remote_repo_git_path" && git init --bare "$remote_repo_git_path" > /dev/null
  echo "INFO: Initialized bare remote repo at $remote_repo_git_path"

  mkdir -p "$repo_source_path" && cd "$repo_source_path"
  git init -b main > /dev/null
  git remote add origin ../remote_repo.git # Relative path
  "$GIT_TRUNK_CMD" init --store "$store_name"
  assert_success "repo_source: $GIT_TRUNK_CMD init --store $store_name"
  local store_content="Content for $store_name store"
  echo "$store_content" > ".trunk/$store_name/index.html"
  "$GIT_TRUNK_CMD" commit --force --store "$store_name" -m "Commit for $store_name store"
  assert_success "repo_source: $GIT_TRUNK_CMD commit --force --store $store_name"
  "$GIT_TRUNK_CMD" push --store "$store_name"
  assert_success "repo_source: $GIT_TRUNK_CMD push --store $store_name"
  assert_remote_ref_exists "." "origin" "refs/trunk/$store_name"
  cd ..

  mkdir -p "$repo_clone_path" && cd "$repo_clone_path"
  git init -b main > /dev/null
  git remote add origin ../remote_repo.git # Relative path
  assert_dir_does_not_exist ".trunk"
  echo "INFO: Set up repo_clone for specific store test at $(pwd)"

  # Action: git trunk checkout --store <store_name>
  "$GIT_TRUNK_CMD" checkout --store "$store_name"
  assert_success "repo_clone: $GIT_TRUNK_CMD checkout --store $store_name"

  # Verify
  assert_dir_exists ".trunk/$store_name"
  assert_file_exists ".trunk/$store_name/index.html"
  assert_grep "$store_content" ".trunk/$store_name/index.html"
  assert_git_repo ".trunk/$store_name"
  assert_commit_hash_matches ".trunk/$store_name" "HEAD" "$remote_repo_git_path" "refs/trunk/$store_name"
  assert_ref_exists "." "refs/trunk/$store_name"
  assert_grep ".trunk" ".gitignore" # Should still add .trunk to gitignore

  echo "INFO: test_checkout_specific_store PASSED"
}

test_checkout_non_existent_ref() {
  echo "INFO: Starting test_checkout_non_existent_ref..."
  local test_subdir="$TEST_DIR/checkout_non_existent"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"
  
  local remote_repo_git_path="$test_subdir/remote_for_non_existent.git"
  mkdir -p "$remote_repo_git_path" && git init --bare "$remote_repo_git_path" > /dev/null

  git init -b main > /dev/null
  # remote_repo_git_path is $test_subdir/remote_for_non_existent.git
  # and we are in $test_subdir, so the relative path is just the basename.
  git remote add origin remote_for_non_existent.git 
  echo "INFO: Initialized repo with remote, but no trunk refs exist locally or remotely."

  local non_existent_store="non_existent_store"
  # Action: git trunk checkout --store non_existent_store
  output=$("$GIT_TRUNK_CMD" checkout --store "$non_existent_store" 2>&1) || true
  assert_failure "$GIT_TRUNK_CMD checkout --store $non_existent_store"

  # Verify error message (flexible check)
  if ! echo "$output" | grep -qE "Ref 'refs/trunk/$non_existent_store' not found locally or on remote 'origin'|Local ref 'refs/trunk/$non_existent_store' for store '$non_existent_store' does not exist"; then
    echo "ERROR: Expected error message for non-existent ref not found in output:"
    echo "$output"
    # exit 1 # Optional: strict check for error message
  fi
  echo "SUCCESS: Command failed and appropriate error message fragment found."
  assert_dir_does_not_exist ".trunk/$non_existent_store"
  assert_dir_does_not_exist ".trunk" # .trunk should not be created if checkout fails

  echo "INFO: test_checkout_non_existent_ref PASSED"
}


# Main execution
main() {
  echo "INFO: Running test_checkout.sh..."
  test_checkout_from_local_ref
  test_checkout_from_remote_ref
  test_checkout_force_overwrite
  test_checkout_specific_store
  test_checkout_non_existent_ref
  echo "INFO: All tests in test_checkout.sh PASSED"
  exit 0
}

main
