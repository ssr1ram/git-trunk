# **Overall Test Strategy:**

1.  **Test Environment:**
    *   All tests will be run within a temporary directory (e.g., `/tmp/git_trunk_tests` or `tests/functional/workspace`).
    *   Each major test scenario will create its own subdirectories for Git repositories to ensure isolation.
    *   We'll need `git` installed and the `git-trunk` executable available in the `PATH` or directly referenced.
2.  **Simulating Remotes:**
    *   For commands involving remotes (`push`, `checkout` from remote, `delete` from remote), we'll create a local bare Git repository to act as the `origin`.
3.  **Scripting:**
    *   Bash scripts are ideal.
    *   Each script (or function within a larger script) will represent a test case or a flow.
    *   Use `set -e` to exit on error, `set -u` to treat unset variables as errors, and `set -o pipefail` for pipeline safety. `set -x` can be useful for debugging script execution.
4.  **Output & Assertions:**
    *   Scripts should `echo` what they are doing at each step (e.g., "INFO: Initializing main repository...").
    *   Assertions will be made by:
        *   Checking command exit codes.
        *   Verifying file/directory existence (`[ -f file ]`, `[ -d dir ]`).
        *   Checking file contents (`grep "pattern" file`).
        *   Inspecting Git state (`git status`, `git log`, `git show-ref`, `git ls-remote`).
        *   Checking `git-trunk info` output.
5.  **Cleanup:**
    *   Each test script should clean up its temporary directories unless specifically designed to build on a previous state (which should be minimized for atomic tests).

**Test Plan Structure:**

We'll create a main test runner script (e.g., `run_tests.sh`) that calls individual test scripts for each command or flow.

```bash
#!/bin/bash
# Main test runner

BASE_TEST_DIR=$(pwd)/functional_tests_workspace
GIT_TRUNK_CMD="git trunk" # Or path to your compiled binary e.g., ../target/debug/git-trunk

# Ensure git-trunk command is found
if ! command -v $(echo $GIT_TRUNK_CMD | awk '{print $1}') &> /dev/null; then
    echo "ERROR: git-trunk command not found at '$GIT_TRUNK_CMD'. Please build or set path."
    exit 1
fi

# --- Helper Functions ---
cleanup() {
    echo "INFO: Cleaning up test workspace: $BASE_TEST_DIR"
    rm -rf "$BASE_TEST_DIR"
}

setup_workspace() {
    cleanup
    mkdir -p "$BASE_TEST_DIR"
    echo "INFO: Test workspace created at $BASE_TEST_DIR"
}

# Trap for cleanup on exit
trap cleanup EXIT

# --- Test Scripts ---
# (Each of these will be a separate .sh file or function)
# source ./test_init.sh
# source ./test_commit.sh
# ...

# --- Main Execution ---
setup_workspace

echo "===== RUNNING GIT-TRUNK FUNCTIONAL TESTS ====="

# Call individual test scripts/functions
# test_init_basic
# test_init_force
# test_full_flow_local
# test_full_flow_remote
# test_stegano
# test_delete
# test_hooks
# test_info
# test_multiple_stores

# Example:
# ./test_suites/test_init.sh "$BASE_TEST_DIR" "$GIT_TRUNK_CMD" || exit 1
# ./test_suites/test_commit.sh "$BASE_TEST_DIR" "$GIT_TRUNK_CMD" || exit 1
# ...

echo "===== ALL TESTS PASSED (or completed) ====="
exit 0
```

Let's detail the test scripts/scenarios for each command:

**Individual Test Script Structure (e.g., `test_suites/test_init.sh`):**

```bash
#!/bin/bash
# test_init.sh

set -euo pipefail
# set -x # Uncomment for debugging

TEST_DIR="$1" # e.g., /tmp/git_trunk_tests/test_init_basic
GIT_TRUNK_CMD="$2"

echo "INFO: Starting test_init_basic in $TEST_DIR"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

# 1. Setup: Create a new Git repo
echo "INFO: Setting up main_repo"
mkdir main_repo
cd main_repo
git init -b main
git config user.email "test@example.com"
git config user.name "Test User"
touch initial_file.txt
git add .
git commit -m "Initial commit"

# 2. Action: Run git trunk init
echo "INFO: Running '$GIT_TRUNK_CMD init'"
"$GIT_TRUNK_CMD" init
INIT_EC=$?

# 3. Assertions
if [ "$INIT_EC" -ne 0 ]; then
    echo "ERROR: '$GIT_TRUNK_CMD init' failed with exit code $INIT_EC"
    exit 1
fi
echo "SUCCESS: '$GIT_TRUNK_CMD init' command succeeded."

if [ ! -d ".trunk/main" ]; then
    echo "ERROR: .trunk/main directory not created"
    exit 1
fi
echo "VERIFY: .trunk/main directory exists."

if [ ! -f ".trunk/main/readme.md" ]; then
    echo "ERROR: .trunk/main/readme.md not created"
    exit 1
fi
echo "VERIFY: .trunk/main/readme.md exists."

if [ ! -d ".trunk/main/.git" ]; then
    echo "ERROR: .trunk/main is not a git repository"
    exit 1
fi
echo "VERIFY: .trunk/main is a git repository."

if ! grep -q "\.trunk" .gitignore; then
    echo "ERROR: .trunk not added to .gitignore"
    exit 1
fi
echo "VERIFY: .trunk is in .gitignore."

# Check for initial commit in .trunk/main
cd .trunk/main
if ! git log --oneline | grep -q "Initial commit for trunk store"; then
    echo "ERROR: Initial commit not found in .trunk/main"
    cd ../.. # back to main_repo
    exit 1
fi
cd ../.. # back to main_repo
echo "VERIFY: Initial commit exists in .trunk/main."

echo "INFO: test_init_basic PASSED"
cd .. # back to $TEST_DIR base
# rm -rf main_repo # Optional: cleanup specific test repo if not done by global trap
```

---

**Test Scenarios for `git-trunk` Commands:**

**1. `init` Command (`test_init.sh`)**

*   **Scenario 1.1: Basic Init**
    *   Setup: New Git repo.
    *   Action: `git trunk init`
    *   Verify:
        *   `.trunk/main` directory created.
        *   `.trunk/main/readme.md` created.
        *   `.trunk/main` is a Git repo with an initial commit.
        *   `.gitignore` in main repo contains `.trunk`.
*   **Scenario 1.2: Init with `--store custom_store`**
    *   Setup: New Git repo.
    *   Action: `git trunk init --store custom_store`
    *   Verify:
        *   `.trunk/custom_store` created with `readme.md`.
        *   `.trunk/custom_store` is a Git repo.
        *   `.gitignore` contains `.trunk`.
*   **Scenario 1.3: Init with `--force` when directory exists**
    *   Setup: New Git repo, `git trunk init`, then `touch .trunk/main/extra_file.txt`.
    *   Action: `git trunk init --force`
    *   Verify:
        *   `.trunk/main` is recreated (check `extra_file.txt` is gone).
        *   `readme.md` exists.
        *   It's a fresh Git repo inside.
*   **Scenario 1.4: Init when `.trunk` already in `.gitignore`**
    *   Setup: New Git repo, manually add `.trunk` to `.gitignore` and commit.
    *   Action: `git trunk init`
    *   Verify: Command succeeds, no duplicate `.trunk` entry in `.gitignore` (or it handles it gracefully).
*   **Scenario 1.5: Init in a non-Git directory (Expected failure)**
    *   Setup: A directory that is not a Git repository.
    *   Action: `git trunk init`
    *   Verify: Command fails with an appropriate error message.

**2. `commit` Command (`test_commit.sh`)**

*   **Scenario 2.1: Commit changes**
    *   Setup: `git trunk init`, then modify `.trunk/main/readme.md`.
    *   Action: `git trunk commit` (answer 'y' to prompt).
    *   Verify:
        *   Changes are committed within `.trunk/main` repo.
        *   `refs/trunk/main` is created/updated in the main repo.
        *   `git show-ref refs/trunk/main` shows a hash.
        *   Compare this hash with the latest commit hash in `.trunk/main`.
*   **Scenario 2.2: Commit with `-m "Custom message"`**
    *   Setup: `git trunk init`, modify `.trunk/main/readme.md`.
    *   Action: `git trunk commit -m "Custom trunk commit message"`
    *   Verify:
        *   Inner commit in `.trunk/main` has "Custom trunk commit message".
        *   `refs/trunk/main` updated.
*   **Scenario 2.3: Commit with `--force` (auto-stages and commits inner changes)**
    *   Setup: `git trunk init`, modify `.trunk/main/readme.md`.
    *   Action: `git trunk commit --force`
    *   Verify:
        *   No prompt.
        *   Inner commit created with default message.
        *   `refs/trunk/main` updated.
*   **Scenario 2.4: Commit with no changes in `.trunk/<store_name>`**
    *   Setup: `git trunk init`, then `git trunk commit`.
    *   Action: `git trunk commit` again.
    *   Verify: Command informs that there are no changes, or `refs/trunk/main` remains unchanged.
*   **Scenario 2.5: Commit with custom store**
    *   Setup: `git trunk init --store docs`, modify `.trunk/docs/readme.md`.
    *   Action: `git trunk commit --store docs`
    *   Verify: `refs/trunk/docs` is created/updated.

**3. `push` Command (`test_push.sh`)**

*   **Setup for Push Tests:**
    *   Create `main_repo`.
    *   Create `remote_repo.git` (bare repo: `git init --bare remote_repo.git`).
    *   In `main_repo`, `git remote add origin ../remote_repo.git`.
    *   In `main_repo`, `git trunk init`, modify `.trunk/main/readme.md`, `git trunk commit`.
*   **Scenario 3.1: Basic Push**
    *   Action: `git trunk push`
    *   Verify:
        *   `git ls-remote origin refs/trunk/main` in `main_repo` shows the ref on remote.
        *   Alternatively, clone `remote_repo.git` to a new location and check `git show-ref refs/trunk/main`.
*   **Scenario 3.2: Push specific store**
    *   Setup: Add another store: `git trunk init --store assets`, modify and `git trunk commit --store assets`.
    *   Action: `git trunk push --store assets`
    *   Verify: `refs/trunk/assets` is on remote, `refs/trunk/main` (if not pushed before) is NOT on remote.
*   **Scenario 3.3: Push to specific remote**
    *   Setup: Add another bare remote `alt_remote.git` and configure it in `main_repo` as `alternate`.
    *   Action: `git trunk push --remote alternate`
    *   Verify: `refs/trunk/main` is on `alternate` remote.
*   **Scenario 3.4: Push non-existent local ref (Expected failure)**
    *   Setup: Clean repo, no `git trunk init` or `commit`.
    *   Action: `git trunk push`
    *   Verify: Command fails gracefully.

**4. `checkout` Command (`test_checkout.sh`)**

*   **Scenario 4.1: Checkout from local ref**
    *   Setup:
        *   `repo_A`: `git init`, `git trunk init`, modify, `git trunk commit`. `refs/trunk/main` exists locally.
        *   Remove `.trunk/` directory (`rm -rf .trunk`).
    *   Action: `git trunk checkout`
    *   Verify:
        *   `.trunk/main` is recreated with content from `refs/trunk/main`.
        *   `.trunk/main` is a Git repo, and its `main` branch matches `refs/trunk/main`.
        *   `.gitignore` has `.trunk`.
*   **Scenario 4.2: Checkout from remote ref**
    *   Setup:
        *   `repo_source`: `git init`, `git trunk init`, modify, `git trunk commit`, `git trunk push` (to a bare `remote_repo.git`).
        *   `repo_clone`: Clone `main_repo` (not the trunk data, just the main project) to a new location, or just `git init` a new repo and add the remote. Ensure `refs/trunk/main` is *not* local initially.
    *   Action: In `repo_clone`, `git trunk checkout --remote origin` (or default if origin is set up).
    *   Verify:
        *   `.trunk/main` created with content from remote `refs/trunk/main`.
        *   `.trunk/main` is a Git repo.
*   **Scenario 4.3: Checkout with `--force` when `.trunk/<store_name>` exists**
    *   Setup: `git trunk init`, `git trunk checkout`. Then `echo "overwrite me" > .trunk/main/test_overwrite.txt`.
    *   Action: `git trunk checkout --force` (assuming `refs/trunk/main` doesn't have `test_overwrite.txt`).
    *   Verify:
        *   `.trunk/main/test_overwrite.txt` is gone (or content is from the ref).
        *   No prompt.
*   **Scenario 4.4: Checkout specific store**
    *   Setup: Push `refs/trunk/docs` to remote. In a new repo, `git trunk checkout --store docs`.
    *   Verify: `.trunk/docs` is checked out.
*   **Scenario 4.5: Checkout non-existent ref (local and remote) (Expected failure)**
    *   Action: `git trunk checkout --store non_existent_store`
    *   Verify: Command fails gracefully.

**5. `stegano` Command (`test_stegano.sh`)**

*   **Scenario 5.1: Stegano single store, `.trunk/` and `.gitignore` entry remain (if other stores/files exist)**
    *   Setup:
        *   `git trunk init --store s1`, `git trunk checkout --store s1`.
        *   `git trunk init --store s2`, `git trunk checkout --store s2`.
    *   Action: `git trunk stegano --store s1`
    *   Verify:
        *   `.trunk/s1` directory is removed.
        *   `.trunk/s2` directory still exists.
        *   `.trunk/` directory still exists.
        *   `.gitignore` still contains `.trunk`.
        *   `refs/trunk/s1` still exists (`git show-ref refs/trunk/s1`).
*   **Scenario 5.2: Stegano last store, `.trunk/` and `.gitignore` entry removed**
    *   Setup: `git trunk init`, `git trunk checkout`. (Only one store 'main').
    *   Action: `git trunk stegano`
    *   Verify:
        *   `.trunk/main` is removed.
        *   `.trunk/` directory is removed.
        *   `.gitignore` no longer contains the `.trunk` entry (or line is commented/removed).
        *   `refs/trunk/main` still exists.
*   **Scenario 5.3: Stegano when `.trunk/` directory doesn't exist**
    *   Setup: `git trunk init`, `git trunk stegano` (so `.trunk` is gone).
    *   Action: `git trunk stegano` again.
    *   Verify: Command completes without error, possibly with a message "nothing to do".

**6. `delete` Command (`test_delete.sh`)**

*   **Setup for Delete Tests:**
    *   Requires a main repo and a bare remote (`remote_repo.git`).
    *   `git trunk init`, modify, `git trunk commit`, `git trunk push`.
*   **Scenario 6.1: Delete store (local and remote)**
    *   Action: `git trunk delete` (answer 'y' to prompt).
    *   Verify:
        *   `.trunk/main` is removed (if it was checked out).
        *   Local `refs/trunk/main` is deleted (`git show-ref refs/trunk/main` should find nothing).
        *   Remote `refs/trunk/main` is deleted (`git ls-remote origin refs/trunk/main` should find nothing).
        *   If `.trunk/` becomes empty, it's removed.
        *   Based on the *detailed description* (not the example output's step 4), `.gitignore` entry for `.trunk` is *not* touched by `delete` itself. We should verify this. If the example output *is* the intended behavior, then this check changes.
*   **Scenario 6.2: Delete specific store**
    *   Setup: `init/commit/push` for `storeA` and `storeB`.
    *   Action: `git trunk delete --store storeA`
    *   Verify:
        *   Only `storeA` related items (working dir, local ref, remote ref) are deleted.
        *   `storeB` items remain.
        *   `.trunk` in `.gitignore` remains if `storeB` or other content keeps `.trunk/` non-empty or if other stores were intended.
*   **Scenario 6.3: Delete store that only exists locally**
    *   Setup: `init/commit` store `local_only`, do NOT `push`.
    *   Action: `git trunk delete --store local_only`
    *   Verify: Local ref and working dir removed. Command doesn't fail trying to delete from remote.
*   **Scenario 6.4: Delete store that only exists remotely (e.g. after deleting local ref manually)**
    *   Setup: `init/commit/push` store `remote_one`. Then `git update-ref -d refs/trunk/remote_one`.
    *   Action: `git trunk delete --store remote_one`
    *   Verify: Remote ref deleted. Command doesn't fail trying to delete local ref or working dir.

**7. `hooks` Command (`test_hooks.sh`)**

*   **Scenario 7.1: Install post-commit hook**
    *   Setup: `git trunk init`.
    *   Action: `git trunk hooks --install post-commit` (answer 'y').
    *   Verify:
        *   `.git/hooks/post-commit` file exists and is executable.
        *   Content of the hook script contains `git trunk commit --force --store main` (or the relevant store).
*   **Scenario 7.2: Test post-commit hook functionality**
    *   Setup: Install post-commit hook (7.1). Modify a file in `.trunk/main/`.
    *   Action: In the main repo, `touch new_main_file.txt && git add . && git commit -m "Main repo commit"`.
    *   Verify:
        *   `refs/trunk/main` in the main repo is updated automatically to reflect changes in `.trunk/main/`.
*   **Scenario 7.3: Install pre-push hook**
    *   Setup: `git trunk init`.
    *   Action: `git trunk hooks --install pre-push` (answer 'y').
    *   Verify:
        *   `.git/hooks/pre-push` file exists and is executable.
        *   Content contains `git trunk push --store main --remote <remote_name_from_hook_or_origin>`.
*   **Scenario 7.4: Test pre-push hook functionality**
    *   Setup: Bare remote. Install pre-push hook (7.3). `git trunk init`, `commit`.
    *   Action: In main repo, `git push origin main` (assuming `main` branch of main repo is being pushed).
    *   Verify:
        *   `refs/trunk/main` is pushed to the remote.
*   **Scenario 7.5: Install hooks with `--force`**
    *   Setup: Manually create dummy `.git/hooks/post-commit`.
    *   Action: `git trunk hooks --install post-commit --force`.
    *   Verify: Hook is overwritten with the git-trunk hook.
*   **Scenario 7.6: Install hooks for a custom store**
    *   Setup: `git trunk init --store custom`.
    *   Action: `git trunk hooks --install post-commit --store custom`.
    *   Verify: Hook script references `--store custom`.

**8. `info` Command (`test_info.sh`)**

*   **Scenario 8.1: Info for a single, synchronized store**
    *   Setup: `init`, `commit`, `push`. `checkout`.
    *   Action: `git trunk info`
    *   Verify: Output shows local dir, local ref, remote ref all exist and (ideally) point to the same commit hash. No uncommitted changes.
*   **Scenario 8.2: Info with uncommitted changes in `.trunk/<store_name>`**
    *   Setup: `init`, `checkout`. Modify `.trunk/main/readme.md` but don't `git trunk commit`.
    *   Action: `git trunk info`
    *   Verify: Output indicates uncommitted changes for the local `.trunk/main` directory.
*   **Scenario 8.3: Info when local ref is ahead of remote**
    *   Setup: `init`, `commit`, `push`. Then `checkout`, modify `.trunk/main/readme.md`, `git trunk commit` (but not push).
    *   Action: `git trunk info`
    *   Verify: Output shows different commit hashes for local `refs/trunk/main` and remote `refs/trunk/main`.
*   **Scenario 8.4: Info when store only exists on remote**
    *   Setup: `init`, `commit`, `push` from `repo_A`. In `repo_B` (new clone or different repo pointing to same remote), don't `checkout`.
    *   Action: `git trunk info` (or `git trunk info --store main`)
    *   Verify: Output shows remote ref exists, local dir and local ref might be absent or show as not checked out.
*   **Scenario 8.5: Info with `--all`**
    *   Setup: Multiple stores (`main`, `docs`, `assets`) pushed to remote. Some checked out locally, some not.
    *   Action: `git trunk info --all`
    *   Verify: Output lists all stores found on the remote and their respective local/remote statuses.
*   **Scenario 8.6: Info for non-existent store**
    *   Action: `git trunk info --store does_not_exist`
    *   Verify: Output indicates the store is not found.

**9. Global Options (`test_global_opts.sh`)**

*   **Scenario 9.1: `--verbose`**
    *   Action: Run any command (e.g., `git trunk init -v`).
    *   Verify: Output is more detailed than without `-v`. (This might be a visual check or regex for debug patterns).
*   **Scenario 9.2: `--remote <REMOTE>` (covered by push/checkout tests)**
    *   Ensure commands like `push`, `checkout` respect the specified remote.
*   **Scenario 9.3: `--store <STORE>` (covered by most command tests)**
    *   Ensure commands operate on the correct store when specified.

**Running the Tests:**

The main `run_tests.sh` would iterate through these test scripts, passing the necessary base directory and `git-trunk` command. Each script would `cd` into its unique test directory, perform actions, and assert.

This comprehensive plan should cover the core functionality and many edge cases of `git-trunk`. Remember to make assertion checks robust (e.g., not just existence, but also content or specific Git states).