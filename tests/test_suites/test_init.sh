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
  if ! grep -q "$1" "$2"; then
    echo "ERROR: Pattern '$1' not found in file '$2'."
    exit 1
  fi
  echo "SUCCESS: Pattern '$1' found in file '$2'."
}

assert_grep_count() {
  local pattern="$1"
  local file="$2"
  local expected_count="$3"
  local actual_count
  actual_count=$(grep -c "$pattern" "$file" || true) # || true to prevent script exit if pattern not found
  if [ "$actual_count" -ne "$expected_count" ]; then
    echo "ERROR: Pattern '$pattern' found $actual_count times in file '$file', expected $expected_count."
    exit 1
  fi
  echo "SUCCESS: Pattern '$pattern' found $expected_count times in file '$file'."
}

# Test scenarios

test_init_basic() {
  echo "INFO: Starting test_init_basic..."
  local test_subdir="$TEST_DIR/init_basic"
  rm -rf "$test_subdir" # Clean up before test
  mkdir -p "$test_subdir"
  cd "$test_subdir"

  # Setup: New Git repo
  git init -b main > /dev/null
  echo "INFO: Initialized new Git repo in $test_subdir"

  # Action: git trunk init
  "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"

  # Verify
  assert_dir_exists ".trunk/main"
  assert_file_exists ".trunk/main/readme.md"
  assert_git_repo ".trunk/main"
  # Verify initial commit in .trunk/main
  if ! git -C ".trunk/main" log --oneline | grep -q "Initial commit"; then
    echo "ERROR: Initial commit not found in .trunk/main"
    exit 1
  fi
  echo "SUCCESS: Initial commit found in .trunk/main"
  assert_grep ".trunk" ".gitignore"

  echo "INFO: test_init_basic PASSED"
}

test_init_custom_store() {
  echo "INFO: Starting test_init_custom_store..."
  local test_subdir="$TEST_DIR/init_custom_store"
  rm -rf "$test_subdir" # Clean up before test
  mkdir -p "$test_subdir"
  cd "$test_subdir"

  # Setup: New Git repo
  git init -b main > /dev/null
  echo "INFO: Initialized new Git repo in $test_subdir"

  # Action: git trunk init --store custom_store
  "$GIT_TRUNK_CMD" init --store custom_store
  assert_success "$GIT_TRUNK_CMD init --store custom_store"

  # Verify
  assert_dir_exists ".trunk/custom_store"
  assert_file_exists ".trunk/custom_store/readme.md"
  assert_git_repo ".trunk/custom_store"
  assert_grep ".trunk" ".gitignore"

  echo "INFO: test_init_custom_store PASSED"
}

test_init_force() {
  echo "INFO: Starting test_init_force..."
  local test_subdir="$TEST_DIR/init_force"
  rm -rf "$test_subdir" # Clean up before test
  mkdir -p "$test_subdir"
  cd "$test_subdir"

  # Setup: New Git repo, git trunk init, then touch .trunk/main/extra_file.txt
  git init -b main > /dev/null
  echo "INFO: Initialized new Git repo in $test_subdir"
  "$GIT_TRUNK_CMD" init
  assert_success "Initial $GIT_TRUNK_CMD init"
  touch ".trunk/main/extra_file.txt"
  assert_file_exists ".trunk/main/extra_file.txt"
  echo "INFO: Created .trunk/main/extra_file.txt"

  # Action: git trunk init --force
  "$GIT_TRUNK_CMD" init --force
  assert_success "$GIT_TRUNK_CMD init --force"

  # Verify
  assert_dir_exists ".trunk/main"
  assert_file_exists ".trunk/main/readme.md"
  assert_file_does_not_exist ".trunk/main/extra_file.txt"
  assert_git_repo ".trunk/main"
  # Verify initial commit in .trunk/main (indicates a fresh repo)
  if ! git -C ".trunk/main" log --oneline | grep -q "Initial commit"; then
    echo "ERROR: Initial commit not found in .trunk/main after --force"
    exit 1
  fi
  echo "SUCCESS: Initial commit found in .trunk/main after --force"

  echo "INFO: test_init_force PASSED"
}

test_init_already_in_gitignore() {
  echo "INFO: Starting test_init_already_in_gitignore..."
  local test_subdir="$TEST_DIR/init_already_in_gitignore"
  rm -rf "$test_subdir" # Clean up before test
  mkdir -p "$test_subdir"
  cd "$test_subdir"

  # Setup: New Git repo, manually add .trunk to .gitignore and commit
  git init -b main > /dev/null
  echo ".trunk" > .gitignore
  git add .gitignore
  git commit -m "Add .trunk to .gitignore" > /dev/null
  echo "INFO: Initialized new Git repo and added .trunk to .gitignore"

  # Action: git trunk init
  "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"

  # Verify
  assert_dir_exists ".trunk/main"
  assert_file_exists ".trunk/main/readme.md"
  assert_grep_count "^\.trunk/?$" ".gitignore" 1 # Ensure only one entry for .trunk

  echo "INFO: test_init_already_in_gitignore PASSED"
}

test_init_non_git_dir() {
  echo "INFO: Starting test_init_non_git_dir..."
  local test_subdir="$TEST_DIR/init_non_git_dir"
  rm -rf "$test_subdir" # Clean up before test
  mkdir -p "$test_subdir"
  cd "$test_subdir"

  echo "INFO: Created non-Git directory $test_subdir"

  # Action: git trunk init
  "$GIT_TRUNK_CMD" init
  assert_failure "$GIT_TRUNK_CMD init in non-Git directory"

  # Verify: Check for a specific error message if possible (optional)
  # For now, just ensuring it fails is sufficient.
  # Example: if ! output=$( "$GIT_TRUNK_CMD" init 2>&1 ); then ... fi

  echo "INFO: test_init_non_git_dir PASSED (command failed as expected)"
}


# Main execution
main() {
  echo "INFO: Running test_init.sh..."
  test_init_basic
  test_init_custom_store
  test_init_force
  test_init_already_in_gitignore
  test_init_non_git_dir
  echo "INFO: All tests in test_init.sh PASSED"
  exit 0
}

main
