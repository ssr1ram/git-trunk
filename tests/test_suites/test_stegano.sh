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

assert_not_grep() {
  # Usage: assert_not_grep "pattern" "file" ["description"]
  local pattern="$1"
  local file="$2"
  local description="${3:-Pattern '$pattern' NOT in file '$file'}"
  if grep -q "$pattern" "$file"; then
    echo "ERROR: Pattern '$pattern' was found in file '$file' but was not expected."
    exit 1
  fi
  echo "SUCCESS: $description as expected."
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

# Test scenarios

test_stegano_single_store_others_exist() {
  echo "INFO: Starting test_stegano_single_store_others_exist..."
  local test_subdir="$TEST_DIR/stegano_single_store"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  echo "INFO: Initialized Git repo in $(pwd)"

  # Setup store s1
  "$GIT_TRUNK_CMD" init --store s1
  assert_success "$GIT_TRUNK_CMD init --store s1"
  echo "content s1" > .trunk/s1/data.txt
  "$GIT_TRUNK_CMD" commit --force --store s1 -m "Commit s1"
  assert_success "$GIT_TRUNK_CMD commit --force --store s1"
  "$GIT_TRUNK_CMD" checkout --store s1
  assert_success "$GIT_TRUNK_CMD checkout --store s1"

  # Setup store s2
  "$GIT_TRUNK_CMD" init --store s2
  assert_success "$GIT_TRUNK_CMD init --store s2"
  echo "content s2" > .trunk/s2/data.txt
  "$GIT_TRUNK_CMD" commit --force --store s2 -m "Commit s2"
  assert_success "$GIT_TRUNK_CMD commit --force --store s2"
  "$GIT_TRUNK_CMD" checkout --store s2
  assert_success "$GIT_TRUNK_CMD checkout --store s2"

  assert_dir_exists ".trunk/s1"
  assert_dir_exists ".trunk/s2"
  assert_dir_exists ".trunk"
  assert_grep ".trunk" ".gitignore"

  # Action: git trunk stegano --store s1
  "$GIT_TRUNK_CMD" stegano --store s1
  assert_success "$GIT_TRUNK_CMD stegano --store s1"

  # Verify
  assert_dir_does_not_exist ".trunk/s1"
  assert_dir_exists ".trunk/s2" "Store s2 directory still exists"
  assert_dir_exists ".trunk" "Parent .trunk directory still exists"
  assert_grep ".trunk" ".gitignore" ".gitignore still contains .trunk entry"
  assert_ref_exists "." "refs/trunk/s1" "Ref refs/trunk/s1 still exists"
  assert_ref_exists "." "refs/trunk/s2" "Ref refs/trunk/s2 still exists"

  echo "INFO: test_stegano_single_store_others_exist PASSED"
}

test_stegano_last_store() {
  echo "INFO: Starting test_stegano_last_store..."
  local test_subdir="$TEST_DIR/stegano_last_store"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  echo "INFO: Initialized Git repo in $(pwd)"

  # Setup store main
  "$GIT_TRUNK_CMD" init --store main
  assert_success "$GIT_TRUNK_CMD init --store main"
  echo "content main" > .trunk/main/data.txt
  "$GIT_TRUNK_CMD" commit --force --store main -m "Commit main"
  assert_success "$GIT_TRUNK_CMD commit --force --store main"
  "$GIT_TRUNK_CMD" checkout --store main
  assert_success "$GIT_TRUNK_CMD checkout --store main"

  assert_dir_exists ".trunk/main"
  assert_dir_exists ".trunk"
  assert_grep "^\.trunk/?$" ".gitignore" ".gitignore contains .trunk entry"

  # Action: git trunk stegano --store main
  "$GIT_TRUNK_CMD" stegano --store main # Could also be just `stegano` if it defaults
  assert_success "$GIT_TRUNK_CMD stegano --store main"

  # Verify
  assert_dir_does_not_exist ".trunk/main"
  assert_dir_does_not_exist ".trunk" "Parent .trunk directory removed"
  # Check if .gitignore no longer contains the .trunk entry or it's commented out
  # This depends on the exact behavior of `git trunk stegano`
  # For now, we'll check it's not there as an active entry.
  # A more robust check might look for "# .trunk" or ensure the line is gone.
  if [ -f ".gitignore" ]; then
    assert_not_grep "^\.trunk/?$" ".gitignore" ".gitignore no longer contains active .trunk entry"
  else
    echo "SUCCESS: .gitignore file does not exist, so .trunk entry is not present."
  fi
  assert_ref_exists "." "refs/trunk/main" "Ref refs/trunk/main still exists"

  echo "INFO: test_stegano_last_store PASSED"
}

test_stegano_no_trunk_dir() {
  echo "INFO: Starting test_stegano_no_trunk_dir..."
  local test_subdir="$TEST_DIR/stegano_no_trunk_dir"
  rm -rf "$test_subdir" && mkdir -p "$test_subdir" && cd "$test_subdir"

  git init -b main > /dev/null
  echo "INFO: Initialized Git repo in $(pwd)"

  # Setup: init, commit, checkout, then stegano to remove .trunk
  "$GIT_TRUNK_CMD" init
  assert_success "$GIT_TRUNK_CMD init"
  echo "initial" > .trunk/main/data.txt
  "$GIT_TRUNK_CMD" commit --force -m "Initial commit"
  assert_success "$GIT_TRUNK_CMD commit --force"
  "$GIT_TRUNK_CMD" checkout
  assert_success "$GIT_TRUNK_CMD checkout"
  "$GIT_TRUNK_CMD" stegano # Remove .trunk for the main store
  assert_success "$GIT_TRUNK_CMD stegano (first pass)"
  assert_dir_does_not_exist ".trunk" "Parent .trunk directory confirmed removed"

  # Action: git trunk stegano again
  output=$("$GIT_TRUNK_CMD" stegano 2>&1)
  assert_success "$GIT_TRUNK_CMD stegano (second pass, .trunk already gone)"

  # Verify
  # Command completes without error (assert_success checks this)
  # Optionally, check for "nothing to do" message if the tool provides one
  if [[ "$output" == *"nothing to do"* || "$output" == *"'.trunk' directory not found"* || "$output" == *"No stores found to stegano"* ]]; then
    echo "SUCCESS: Command reported nothing to do or .trunk not found, as expected."
  else
    echo "INFO: Command output was: $output (no specific 'nothing to do' message checked)"
  fi
  assert_dir_does_not_exist ".trunk" # Still shouldn't exist

  echo "INFO: test_stegano_no_trunk_dir PASSED"
}

# Main execution
main() {
  echo "INFO: Running test_stegano.sh..."
  test_stegano_single_store_others_exist
  test_stegano_last_store
  test_stegano_no_trunk_dir
  echo "INFO: All tests in test_stegano.sh PASSED"
  exit 0
}

main
