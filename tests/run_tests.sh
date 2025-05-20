#!/bin/bash

# Exit on error, treat unset variables as errors, and ensure pipeline safety.
set -euo pipefail

# --- Configuration ---
# GIT_TRUNK_CMD can be overridden by an environment variable
: "${GIT_TRUNK_CMD:=git-trunk}" # Default to 'git-trunk' if not set

BASE_TEST_DIR_NAME="functional_tests_workspace"
BASE_TEST_DIR="$(pwd)/${BASE_TEST_DIR_NAME}"
TEST_SUITES_DIR="$(pwd)/test_suites"

# List of test suite scripts to run
TEST_SUITES=(
    "test_init.sh"
    "test_commit.sh"
    "test_push.sh"
    "test_checkout.sh"
    "test_stegano.sh"
    "test_delete.sh"
    "test_hooks.sh"
    "test_info.sh"
    "test_full_flow.sh"
)

# --- Helper Functions ---
cleanup_workspace() {
    echo "INFO: Cleaning up test workspace: $BASE_TEST_DIR"
    rm -rf "$BASE_TEST_DIR"
}

setup_workspace() {
    cleanup_workspace
    mkdir -p "$BASE_TEST_DIR"
    echo "INFO: Test workspace created at $BASE_TEST_DIR"
}

# --- Main Execution ---
main() {
    echo "===== GIT-TRUNK FUNCTIONAL TESTS ====="
    echo "INFO: Using git-trunk command: '$GIT_TRUNK_CMD'"
    echo "INFO: Test workspace: $BASE_TEST_DIR"

    # Verify git-trunk command exists
    if ! command -v $(echo "$GIT_TRUNK_CMD" | awk '{print $1}') &> /dev/null; then
        echo "ERROR: git-trunk command not found at '$GIT_TRUNK_CMD'."
        echo "Please build it or set the GIT_TRUNK_CMD environment variable."
        exit 1
    fi

    # Verify git is installed
    if ! command -v git &> /dev/null; then
        echo "ERROR: git command not found. Please install Git."
        exit 1
    fi

    setup_workspace
    # Trap for cleanup on script exit (successful or not)
    # trap cleanup_workspace EXIT # Can be noisy if scripts fail early

    local all_tests_passed=true
    local test_suite_script_path
    local test_suite_name
    local test_suite_dir

    for test_suite_script in "${TEST_SUITES[@]}"; do
        test_suite_script_path="${TEST_SUITES_DIR}/${test_suite_script}"
        test_suite_name=$(basename "$test_suite_script" .sh)
        test_suite_dir="${BASE_TEST_DIR}/${test_suite_name}_suite"

        echo -e "\n--- RUNNING SUITE: $test_suite_name ---"
        mkdir -p "$test_suite_dir"

        if [ ! -f "$test_suite_script_path" ]; then
            echo "ERROR: Test suite script not found: $test_suite_script_path"
            all_tests_passed=false
            continue
        fi

        chmod +x "$test_suite_script_path"
        if ! "$test_suite_script_path" "$test_suite_dir" "$GIT_TRUNK_CMD"; then
            echo "ERROR: Test suite '$test_suite_name' FAILED."
            all_tests_passed=false
            # Optionally, exit immediately on first failure:
            # echo "Forcing exit due to test suite failure."
            # exit 1
        else
            echo "SUCCESS: Test suite '$test_suite_name' PASSED."
        fi
    done

    echo -e "\n===== TEST SUMMARY ====="
    if $all_tests_passed; then
        echo "INFO: All test suites PASSED."
        # cleanup_workspace # Optional: clean up only on full success
        exit 0
    else
        echo "ERROR: Some test suites FAILED."
        echo "INFO: Test workspace retained for inspection: $BASE_TEST_DIR"
        exit 1
    fi
}

# Run the main function
main