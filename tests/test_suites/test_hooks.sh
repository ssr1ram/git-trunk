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

assert_file_executable() {
  if [ ! -x "$1" ]; then
    echo "ERROR: File '$1' is not executable."
    exit 1
  fi
  echo "SUCCESS: File '$1' is executable."
}

assert_grep() {
  local pattern="$1"
  local file="$2"
  local description="${3:-Pattern '$pattern' in file '$file'}"
  if ! grep -q "$pattern" "$file"; then
    echo "ERROR: Pattern '$pattern' not found in file '$file'."
    echo "File content:"
    cat "$file"
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

assert_remote_ref_exists() {
  local repo_path="$1" # Path to the repo where ls-remote is run
  local remote_name="$2"
  local ref_name="$3"
  local current_dir
  current_dir=$(pwd)
  cd "$repo_path" 
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

test_install_post_commit_hook() {
  echo "INFO: Starting test_install_post_commit_hook..."
  local test_subdir="$TEST_DIR/hooks_install_post_commit"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"

  yes | "$GIT_TRUNK_CMD" hooks --install post-commit
  assert_success "yes | $GIT_TRUNK_CMD hooks --install post-commit"

  local hook_path=".git/hooks/post-commit"
  assert_file_exists "$hook_path"
  assert_file_executable "$hook_path"
  assert_grep "git trunk commit --force --store main" "$hook_path" "Hook content check"

  echo "INFO: test_install_post_commit_hook PASSED"
}

test_post_commit_hook_functionality() {
  echo "INFO: Starting test_post_commit_hook_functionality..."
  local test_subdir="$TEST_DIR/hooks_post_commit_func"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  # Configure git user for commit to succeed
  git config user.name "Test User"
  git config user.email "test@example.com"

  "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"
  # Checkout to populate .trunk/main
  "$GIT_TRUNK_CMD" checkout 
  assert_success "$GIT_TRUNK_CMD checkout"


  yes | "$GIT_TRUNK_CMD" hooks --install post-commit
  assert_success "yes | $GIT_TRUNK_CMD hooks --install post-commit"

  local readme_path=".trunk/main/readme.md"
  echo "change for post-commit hook test" >> "$readme_path"
  # No need to git add/commit in .trunk/main, the hook should handle it.

  # Action: Commit in the main repo. This should trigger the hook.
  git add . # Add the changes in .trunk/main (which is part of the main repo's working tree)
  git commit -m "Main repo commit triggering post-commit hook"
  assert_success "git commit in main repo"

  # Verify: refs/trunk/main is updated
  assert_ref_exists "." "refs/trunk/main"
  # The commit in .trunk/main will have the message "Update trunk store 'main'" (default by the hook)
  # So we check if the latest commit in .trunk/main has this message.
  local store_head_message
  store_head_message=$(git -C .trunk/main log -1 --pretty=%B HEAD)
  if [[ "$store_head_message" != "Update trunk store 'main'" ]]; then
      echo "ERROR: Unexpected commit message in .trunk/main. Expected 'Update trunk store 'main'', got '$store_head_message'"
      exit 1
  fi
  echo "SUCCESS: Commit message in .trunk/main is as expected from hook."
  assert_commit_hash_matches ".trunk/main" "HEAD" "." "refs/trunk/main"

  echo "INFO: test_post_commit_hook_functionality PASSED"
}

test_install_pre_push_hook() {
  echo "INFO: Starting test_install_pre_push_hook..."
  local test_env_dir="$TEST_DIR/hooks_install_pre_push"
  rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

  local main_repo_path="$test_env_dir/main_repo"
  local remote_repo_git_path="$test_env_dir/origin_repo.git"

  mkdir -p "$remote_repo_git_path" && git init --bare "$remote_repo_git_path" > /dev/null
  echo "INFO: Initialized bare remote repo at $remote_repo_git_path"

  mkdir -p "$main_repo_path" && cd "$main_repo_path"
  git init -b main > /dev/null
  git remote add origin "$remote_repo_git_path"
  "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"

  yes | "$GIT_TRUNK_CMD" hooks --install pre-push
  assert_success "yes | $GIT_TRUNK_CMD hooks --install pre-push"

  local hook_path=".git/hooks/pre-push"
  assert_file_exists "$hook_path"
  assert_file_executable "$hook_path"
  # The hook script is more complex, check for key parts.
  # It should call `git trunk push` for the relevant store and remote.
  # The default remote is 'origin' and default store is 'main'.
  assert_grep "git trunk push" "$hook_path" "Hook content check for 'git trunk push'"
  assert_grep "main" "$hook_path" "Hook content check for store 'main'" # Check if store is mentioned
  assert_grep "origin" "$hook_path" "Hook content check for remote 'origin'" # Check if remote is mentioned

  echo "INFO: test_install_pre_push_hook PASSED"
}

test_pre_push_hook_functionality() {
  echo "INFO: Starting test_pre_push_hook_functionality..."
  local test_env_dir="$TEST_DIR/hooks_pre_push_func"
  rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

  local main_repo_path="$test_env_dir/main_repo"
  local remote_repo_git_path="$test_env_dir/origin_repo.git"

  mkdir -p "$remote_repo_git_path" && git init --bare "$remote_repo_git_path" > /dev/null
  echo "INFO: Initialized bare remote repo at $remote_repo_git_path"

  mkdir -p "$main_repo_path" && cd "$main_repo_path"
  git init -b main > /dev/null
  git config user.name "Test User"
  git config user.email "test@example.com"
  git remote add origin "$remote_repo_git_path"

  "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"
  echo "Pre-push hook test content" >> .trunk/main/readme.md
  "$GIT_TRUNK_CMD" commit --force -m "Commit for pre-push test"
  assert_success "$GIT_TRUNK_CMD commit --force"
  assert_ref_exists "." "refs/trunk/main"

  yes | "$GIT_TRUNK_CMD" hooks --install pre-push
  assert_success "yes | $GIT_TRUNK_CMD hooks --install pre-push"

  # Create a dummy commit on main branch to push
  git commit --allow-empty -m "Dummy commit on main branch to trigger push"
  assert_success "Dummy commit on main branch"

  # Action: Push main branch of main repo. Hook should push refs/trunk/main.
  git push origin main
  assert_success "git push origin main"

  # Verify: refs/trunk/main is pushed to origin remote
  assert_remote_ref_exists "." "origin" "refs/trunk/main"

  echo "INFO: test_pre_push_hook_functionality PASSED"
}

test_install_hooks_force() {
  echo "INFO: Starting test_install_hooks_force..."
  local test_subdir="$TEST_DIR/hooks_install_force"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"

  local hook_path=".git/hooks/post-commit"
  mkdir -p .git/hooks # Ensure hooks directory exists
  echo "# This is a dummy hook" > "$hook_path"
  chmod +x "$hook_path"
  assert_grep "dummy hook" "$hook_path" "Dummy hook content check before overwrite"

  # Action: Install with --force (no 'yes' pipe needed)
  "$GIT_TRUNK_CMD" hooks --install post-commit --force
  assert_success "$GIT_TRUNK_CMD hooks --install post-commit --force"

  assert_file_exists "$hook_path"
  assert_file_executable "$hook_path"
  assert_grep "git trunk commit --force --store main" "$hook_path" "Overwritten hook content check"

  echo "INFO: test_install_hooks_force PASSED"
}

test_install_hooks_custom_store() {
  echo "INFO: Starting test_install_hooks_custom_store..."
  local test_subdir="$TEST_DIR/hooks_install_custom_store"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"
  local custom_store_name="assets"

  git init -b main > /dev/null
  "$GIT_TRUNK_CMD" init --store "$custom_store_name"
  assert_success "$GIT_TRUNK_CMD init --store $custom_store_name"

  yes | "$GIT_TRUNK_CMD" hooks --install post-commit --store "$custom_store_name"
  assert_success "yes | $GIT_TRUNK_CMD hooks --install post-commit --store $custom_store_name"

  local hook_path=".git/hooks/post-commit"
  assert_file_exists "$hook_path"
  assert_file_executable "$hook_path"
  assert_grep "git trunk commit --force --store $custom_store_name" "$hook_path" "Hook content check for custom store"

  echo "INFO: test_install_hooks_custom_store PASSED"
}

# Main execution
main() {
  echo "INFO: Running test_hooks.sh..."
  test_install_post_commit_hook
  test_post_commit_hook_functionality
  test_install_pre_push_hook
  test_pre_push_hook_functionality
  test_install_hooks_force
  test_install_hooks_custom_store
  echo "INFO: All tests in test_hooks.sh PASSED"
  exit 0
}

main
