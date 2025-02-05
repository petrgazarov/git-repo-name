set -e

setup_helper() {
    # Get path to local git-repo-name script
    GIT_REPO_NAME_PATH="$PWD/git-repo-name"
    export GIT_REPO_NAME_PATH
    chmod +x "$GIT_REPO_NAME_PATH"

    # Create a temporary directory for test repositories
    TEST_DIR="$(mktemp -d)"
    export TEST_DIR
    
    # Save original directory
    ORIG_DIR="$PWD"
    export ORIG_DIR
}

teardown_helper() {
    # Return to original directory
    cd "${ORIG_DIR}"
    
    # Safety checks before cleanup
    [ -n "${TEST_DIR}" ] || { echo "TEST_DIR is empty"; exit 1; }
    [ "${TEST_DIR}" != "/" ] || { echo "TEST_DIR is root"; exit 1; }
    
    # Clean up test directory
    rm -rf "${TEST_DIR}"
} 