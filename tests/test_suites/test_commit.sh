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

assert_commit_message_matches() {
  local repo_path="$1"
  local commit_hash_or_ref="$2"
  local expected_message="$3"
  local actual_message
  actual_message=$(git -C "$repo_path" log -1 --pretty=%B "$commit_hash_or_ref")
  # Normalize line endings for comparison (Git might use LF, while echo might use CRLF on some systems)
  actual_message_normalized=$(echo "$actual_message" | tr -d '\r')
  expected_message_normalized=$(echo "$expected_message" | tr -d '\r')

  if [[ "$actual_message_normalized" != "$expected_message_normalized" ]]; then
    echo "ERROR: Commit message in '$repo_path' for '$commit_hash_or_ref' does not match."
    echo "Expected: $expected_message_normalized"
    echo "Actual: $actual_message_normalized"
    exit 1
  fi
  echo "SUCCESS: Commit message in '$repo_path' for '$commit_hash_or_ref' matches."
}

get_commit_hash() {
  git -C "$1" rev-parse "$2" 2>/dev/null || echo "INVALID_REF"
}

# Test scenarios

test_commit_basic_prompt() {
  echo "INFO: Starting test_commit_basic_prompt..."
  local test_subdir="$TEST_DIR/commit_basic_prompt"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  "$GIT_TRUNK_CMD" init
  assert_success "git trunk init"
  echo "New content for readme" >> .trunk/main/readme.md
  # Changes are made in the inner store, but not yet committed there by git trunk
  # git trunk commit should handle staging and committing in the inner store.

  # Action: git trunk commit (simulate 'y' to prompt)
  yes | "$GIT_TRUNK_CMD" commit
  assert_success "yes | $GIT_TRUNK_CMD commit"

  # Verify
  assert_ref_exists "." "refs/trunk/main"
  local trunk_ref_hash
  trunk_ref_hash=$(get_commit_hash "." "refs/trunk/main")
  local store_commit_hash
  store_commit_hash=$(get_commit_hash ".trunk/main" "HEAD")

  if [[ "$trunk_ref_hash" != "$store_commit_hash" ]]; then
    echo "ERROR: Hash of refs/trunk/main ($trunk_ref_hash) does not match latest commit in .trunk/main ($store_commit_hash)."
    exit 1
  fi
  echo "SUCCESS: Hash of refs/trunk/main matches latest commit in .trunk/main."
  assert_commit_message_matches ".trunk/main" "HEAD" "Update trunk store 'main'"
  assert_commit_message_matches "." "refs/trunk/main" "Update trunk store 'main'"


  echo "INFO: test_commit_basic_prompt PASSED"
}

test_commit_custom_message() {
  echo "INFO: Starting test_commit_custom_message..."
  local test_subdir="$TEST_DIR/commit_custom_message"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  "$GIT_TRUNK_CMD" init
  assert_success "git trunk init"
  echo "Another change" >> .trunk/main/readme.md

  local custom_message="Custom trunk commit message"
  "$GIT_TRUNK_CMD" commit -m "$custom_message"
  assert_success "$GIT_TRUNK_CMD commit -m \"$custom_message\""

  assert_ref_exists "." "refs/trunk/main"
  assert_commit_message_matches ".trunk/main" "HEAD" "$custom_message"
  assert_commit_message_matches "." "refs/trunk/main" "$custom_message"
  local trunk_ref_hash
  trunk_ref_hash=$(get_commit_hash "." "refs/trunk/main")
  local store_commit_hash
  store_commit_hash=$(get_commit_hash ".trunk/main" "HEAD")
  if [[ "$trunk_ref_hash" != "$store_commit_hash" ]]; then
    echo "ERROR: Hash of refs/trunk/main ($trunk_ref_hash) does not match latest commit in .trunk/main ($store_commit_hash)."
    exit 1
  fi

  echo "INFO: test_commit_custom_message PASSED"
}

test_commit_force() {
  echo "INFO: Starting test_commit_force..."
  local test_subdir="$TEST_DIR/commit_force"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  "$GIT_TRUNK_CMD" init
  assert_success "git trunk init"
  echo "Force commit changes" >> .trunk/main/readme.md

  "$GIT_TRUNK_CMD" commit --force
  assert_success "$GIT_TRUNK_CMD commit --force"

  assert_ref_exists "." "refs/trunk/main"
  # Default message for inner commit when --force is used without -m
  assert_commit_message_matches ".trunk/main" "HEAD" "Update trunk store 'main'"
  assert_commit_message_matches "." "refs/trunk/main" "Update trunk store 'main'"
  local trunk_ref_hash
  trunk_ref_hash=$(get_commit_hash "." "refs/trunk/main")
  local store_commit_hash
  store_commit_hash=$(get_commit_hash ".trunk/main" "HEAD")
  if [[ "$trunk_ref_hash" != "$store_commit_hash" ]]; then
    echo "ERROR: Hash of refs/trunk/main ($trunk_ref_hash) does not match latest commit in .trunk/main ($store_commit_hash)."
    exit 1
  fi

  echo "INFO: test_commit_force PASSED"
}

test_commit_no_changes() {
  echo "INFO: Starting test_commit_no_changes..."
  local test_subdir="$TEST_DIR/commit_no_changes"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  "$GIT_TRUNK_CMD" init
  assert_success "git trunk init"
  "$GIT_TRUNK_CMD" commit --force # Initial commit
  assert_success "Initial $GIT_TRUNK_CMD commit --force"
  local initial_trunk_ref_hash
  initial_trunk_ref_hash=$(get_commit_hash "." "refs/trunk/main")

  # Action: git trunk commit --force again with no changes
  output=$("$GIT_TRUNK_CMD" commit --force 2>&1)
  assert_success "$GIT_TRUNK_CMD commit --force (no changes)"

  # Verify: Command informs "no changes" or refs/trunk/main remains unchanged
  if ! echo "$output" | grep -q "No changes to commit in store 'main'"; then
     echo "WARNING: Expected 'No changes to commit' message not found in output:"
     echo "$output"
     # We still check if the ref is unchanged as the primary validation
  else
    echo "SUCCESS: 'No changes to commit' message found."
  fi

  local current_trunk_ref_hash
  current_trunk_ref_hash=$(get_commit_hash "." "refs/trunk/main")
  if [[ "$initial_trunk_ref_hash" != "$current_trunk_ref_hash" ]]; then
    echo "ERROR: refs/trunk/main hash changed despite no changes. Initial: $initial_trunk_ref_hash, Current: $current_trunk_ref_hash"
    exit 1
  fi
  echo "SUCCESS: refs/trunk/main hash remains unchanged."

  echo "INFO: test_commit_no_changes PASSED"
}

test_commit_custom_store() {
  echo "INFO: Starting test_commit_custom_store..."
  local custom_store_name="docs"
  local test_subdir="$TEST_DIR/commit_custom_store"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  "$GIT_TRUNK_CMD" init --store "$custom_store_name"
  assert_success "$GIT_TRUNK_CMD init --store $custom_store_name"
  echo "Content for custom store" >> ".trunk/$custom_store_name/readme.md"

  # Using --force to avoid prompt, as the prompt behavior is tested in basic_prompt
  "$GIT_TRUNK_CMD" commit --store "$custom_store_name" --force
  assert_success "$GIT_TRUNK_CMD commit --store $custom_store_name --force"

  assert_ref_exists "." "refs/trunk/$custom_store_name"
  assert_commit_message_matches ".trunk/$custom_store_name" "HEAD" "Update trunk store '$custom_store_name'"
  assert_commit_message_matches "." "refs/trunk/$custom_store_name" "Update trunk store '$custom_store_name'"

  local trunk_ref_hash
  trunk_ref_hash=$(get_commit_hash "." "refs/trunk/$custom_store_name")
  local store_commit_hash
  store_commit_hash=$(get_commit_hash ".trunk/$custom_store_name" "HEAD")

  if [[ "$trunk_ref_hash" != "$store_commit_hash" ]]; then
    echo "ERROR: Hash of refs/trunk/$custom_store_name ($trunk_ref_hash) does not match latest commit in .trunk/$custom_store_name ($store_commit_hash)."
    exit 1
  fi
  echo "SUCCESS: Hash of refs/trunk/$custom_store_name matches latest commit in .trunk/$custom_store_name."

  echo "INFO: test_commit_custom_store PASSED"
}

# Main execution
main() {
  echo "INFO: Running test_commit.sh..."
  test_commit_basic_prompt
  test_commit_custom_message
  test_commit_force
  test_commit_no_changes
  test_commit_custom_store
  echo "INFO: All tests in test_commit.sh PASSED"
  exit 0
}

main
