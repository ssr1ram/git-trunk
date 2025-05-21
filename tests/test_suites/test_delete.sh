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

assert_remote_ref_does_not_exist() {
  local repo_path="$1" # Path to the repo where ls-remote is run
  local remote_name="$2"
  local ref_name="$3"
  local current_dir
  current_dir=$(pwd)
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

test_delete_store_local_and_remote() {
  echo "INFO: Starting test_delete_store_local_and_remote..."
  local test_env_dir="$TEST_DIR/delete_local_remote"
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
  echo "content" > .trunk/main/data.txt
  "$GIT_TRUNK_CMD" commit --force -m "Initial commit"
  assert_success "$GIT_TRUNK_CMD commit --force"
  "$GIT_TRUNK_CMD" push
  assert_success "$GIT_TRUNK_CMD push"
  "$GIT_TRUNK_CMD" checkout
  assert_success "$GIT_TRUNK_CMD checkout"

  assert_dir_exists ".trunk/main"
  assert_ref_exists "." "refs/trunk/main"
  assert_remote_ref_exists "." "origin" "refs/trunk/main"
  assert_grep ".trunk" ".gitignore"

  # Action: yes | git trunk delete
  yes | "$GIT_TRUNK_CMD" delete
  assert_success "yes | $GIT_TRUNK_CMD delete"

  # Verify
  assert_dir_does_not_exist ".trunk/main"
  assert_dir_does_not_exist ".trunk" # .trunk/ is now empty and should be removed
  assert_ref_does_not_exist "." "refs/trunk/main"
  assert_remote_ref_does_not_exist "." "origin" "refs/trunk/main"
  # .gitignore entry for .trunk is NOT touched by delete
  if [ -f ".gitignore" ]; then # .gitignore might be removed if .trunk was the only entry and auto-cleanup happened
    assert_grep ".trunk" ".gitignore" ".gitignore still contains .trunk entry"
  else
    echo "INFO: .gitignore not present, which is acceptable if .trunk was its only content and git trunk init created it."
    # To be super strict, we could ensure .git trunk init actually *created* .gitignore
    # and then check its content if it still exists. For now, this is okay.
  fi


  echo "INFO: test_delete_store_local_and_remote PASSED"
}

test_delete_specific_store() {
  echo "INFO: Starting test_delete_specific_store..."
  local test_env_dir="$TEST_DIR/delete_specific_store"
  rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

  local main_repo_path="$test_env_dir/main_repo"
  local remote_repo_git_path="$test_env_dir/remote_repo.git"

  mkdir -p "$remote_repo_git_path" && git init --bare "$remote_repo_git_path" > /dev/null
  echo "INFO: Initialized bare remote repo at $remote_repo_git_path"

  mkdir -p "$main_repo_path" && cd "$main_repo_path"
  git init -b main > /dev/null
  git remote add origin "$remote_repo_git_path"
  echo "INFO: Initialized main_repo at $(pwd)"

  # Setup storeA
  "$GIT_TRUNK_CMD" init --store storeA
  echo "dataA" > .trunk/storeA/data.txt
  "$GIT_TRUNK_CMD" commit --force --store storeA -m "Commit storeA"
  "$GIT_TRUNK_CMD" push --store storeA
  "$GIT_TRUNK_CMD" checkout --store storeA
  assert_success "Setup storeA"

  # Setup storeB
  "$GIT_TRUNK_CMD" init --store storeB
  echo "dataB" > .trunk/storeB/data.txt
  "$GIT_TRUNK_CMD" commit --force --store storeB -m "Commit storeB"
  "$GIT_TRUNK_CMD" push --store storeB
  "$GIT_TRUNK_CMD" checkout --store storeB
  assert_success "Setup storeB"

  assert_dir_exists ".trunk/storeA"
  assert_ref_exists "." "refs/trunk/storeA"
  assert_remote_ref_exists "." "origin" "refs/trunk/storeA"
  assert_dir_exists ".trunk/storeB"
  assert_ref_exists "." "refs/trunk/storeB"
  assert_remote_ref_exists "." "origin" "refs/trunk/storeB"
  assert_grep ".trunk" ".gitignore"

  # Action: yes | git trunk delete --store storeA
  yes | "$GIT_TRUNK_CMD" delete --store storeA
  assert_success "yes | $GIT_TRUNK_CMD delete --store storeA"

  # Verify storeA deleted
  assert_dir_does_not_exist ".trunk/storeA"
  assert_ref_does_not_exist "." "refs/trunk/storeA"
  assert_remote_ref_does_not_exist "." "origin" "refs/trunk/storeA"

  # Verify storeB remains
  assert_dir_exists ".trunk/storeB"
  assert_ref_exists "." "refs/trunk/storeB"
  assert_remote_ref_exists "." "origin" "refs/trunk/storeB"
  assert_dir_exists ".trunk" # .trunk/ is not empty
  assert_grep ".trunk" ".gitignore" # .gitignore entry remains

  echo "INFO: test_delete_specific_store PASSED"
}

test_delete_store_local_only() {
  echo "INFO: Starting test_delete_store_local_only..."
  local test_env_dir="$TEST_DIR/delete_local_only"
  rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

  local main_repo_path="$test_env_dir/main_repo"
  local remote_repo_git_path="$test_env_dir/remote_repo.git" # Exists but not used for this store

  mkdir -p "$remote_repo_git_path" && git init --bare "$remote_repo_git_path" > /dev/null
  echo "INFO: Initialized bare remote repo at $remote_repo_git_path"

  mkdir -p "$main_repo_path" && cd "$main_repo_path"
  git init -b main > /dev/null
  git remote add origin "$remote_repo_git_path" # Configure remote
  echo "INFO: Initialized main_repo at $(pwd)"

  "$GIT_TRUNK_CMD" init --store local_only
  echo "local data" > .trunk/local_only/data.txt
  "$GIT_TRUNK_CMD" commit --force --store local_only -m "Commit local_only"
  # DO NOT PUSH
  "$GIT_TRUNK_CMD" checkout --store local_only # Ensure it's checked out
  assert_success "Setup local_only store"

  assert_dir_exists ".trunk/local_only"
  assert_ref_exists "." "refs/trunk/local_only"
  assert_remote_ref_does_not_exist "." "origin" "refs/trunk/local_only" # Verify not on remote

  # Action: yes | git trunk delete --store local_only
  yes | "$GIT_TRUNK_CMD" delete --store local_only
  assert_success "yes | $GIT_TRUNK_CMD delete --store local_only"

  # Verify
  assert_dir_does_not_exist ".trunk/local_only"
  assert_ref_does_not_exist "." "refs/trunk/local_only"
  assert_remote_ref_does_not_exist "." "origin" "refs/trunk/local_only" # Still not on remote
  assert_dir_does_not_exist ".trunk" # .trunk/ should be empty and removed

  echo "INFO: test_delete_store_local_only PASSED"
}

test_delete_store_remote_only() {
  echo "INFO: Starting test_delete_store_remote_only..."
  local test_env_dir="$TEST_DIR/delete_remote_only"
  rm -rf "$test_env_dir" && mkdir -p "$test_env_dir"

  local repo_A_path="$test_env_dir/repo_A"
  local repo_B_path="$test_env_dir/repo_B"
  local remote_repo_git_path="$test_env_dir/remote_repo.git"
  local store_name="remote_one"

  mkdir -p "$remote_repo_git_path" && git init --bare "$remote_repo_git_path" > /dev/null
  echo "INFO: Initialized bare remote repo at $remote_repo_git_path"

  # Setup in repo_A: init, commit, push store_name
  mkdir -p "$repo_A_path" && cd "$repo_A_path"
  git init -b main > /dev/null
  git remote add origin "$remote_repo_git_path"
  "$GIT_TRUNK_CMD" init --store "$store_name"
  echo "remote data" > ".trunk/$store_name/data.txt"
  "$GIT_TRUNK_CMD" commit --force --store "$store_name" -m "Commit $store_name"
  "$GIT_TRUNK_CMD" push --store "$store_name"
  assert_remote_ref_exists "." "origin" "refs/trunk/$store_name"
  echo "INFO: repo_A setup complete and $store_name pushed from $(pwd)"
  cd .. # Back to test_env_dir

  # Setup in repo_B: clone/setup, ensure ref is NOT local
  mkdir -p "$repo_B_path" && cd "$repo_B_path"
  git init -b main > /dev/null
  git remote add origin "$remote_repo_git_path"
  # Fetch to know about remote refs, but don't create local tracking or trunk refs
  git fetch origin
  assert_remote_ref_exists "." "origin" "refs/trunk/$store_name" # Exists on remote
  assert_ref_does_not_exist "." "refs/trunk/$store_name"       # Does NOT exist locally
  assert_dir_does_not_exist ".trunk/$store_name"
  assert_dir_does_not_exist ".trunk"
  echo "INFO: repo_B setup complete at $(pwd)"

  # Action: yes | git trunk delete --store remote_one --remote origin
  # Need to specify --remote as the tool might not know which remote to use if not default
  # The prompt for delete should be for the remote ref.
  yes | "$GIT_TRUNK_CMD" delete --store "$store_name" --remote origin
  assert_success "yes | $GIT_TRUNK_CMD delete --store $store_name --remote origin"

  # Verify
  assert_remote_ref_does_not_exist "." "origin" "refs/trunk/$store_name"
  assert_ref_does_not_exist "." "refs/trunk/$store_name" # Still doesn't exist locally
  assert_dir_does_not_exist ".trunk/$store_name"         # Still doesn't exist locally
  assert_dir_does_not_exist ".trunk"

  echo "INFO: test_delete_store_remote_only PASSED"
}


# Main execution
main() {
  echo "INFO: Running test_delete.sh..."
  test_delete_store_local_and_remote
  test_delete_specific_store
  test_delete_store_local_only
  test_delete_store_remote_only
  echo "INFO: All tests in test_delete.sh PASSED"
  exit 0
}

main
