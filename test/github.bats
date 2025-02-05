#!/usr/bin/env bats

set -e
load ./test_helper

setup() {
    setup_helper
    # Use temporary config directory for tests
    export XDG_CONFIG_HOME="${TEST_DIR}/.config"
    
    # Set up mock curl command
    MOCK_CURL="${TEST_DIR}/curl"
    cp "${BATS_TEST_DIRNAME}/mocks/curl_github-repo-rename" "$MOCK_CURL"
    chmod +x "$MOCK_CURL"
    export PATH="${TEST_DIR}:$PATH"
}

teardown() {
    teardown_helper
}

@test "github/HTTPS URL/when remote repo was renamed/renames local repo directory and updates remote URL" {
    # Set up test repository
    mkdir "${TEST_DIR}/old-repo"
    cd "${TEST_DIR}/old-repo"
    git init
    git remote add origin "https://github.com/old-owner/old-repo.git"
    
    # Configure GitHub token
    "$GIT_REPO_NAME_PATH" config github-token ghp_testtoken123
    
    # Run git repo-name sync
    run "$GIT_REPO_NAME_PATH" sync
    
    # Assert
    [ "$status" -eq 0 ]
    [[ $output =~ Updating\ remote\ URL\ to:\ https://github\.com/new-owner/new-repo\.git ]]
    [[ $output =~ Renaming\ directory\ from\ \'old-repo\'\ to\ \'new-repo\' ]]
    
    # Verify directory was renamed
    [ ! -d "${TEST_DIR}/old-repo" ]
    [ -d "${TEST_DIR}/new-repo" ]
    
    # Verify remote URL was updated
    cd "${TEST_DIR}/new-repo"
    [ "$(git remote get-url origin)" = "https://github.com/new-owner/new-repo.git" ]
}

# @test "github/SSH URL/when remote repo was renamed/renames local repo directory and updates remote URL" {
#     mkdir "${TEST_DIR}/old-repo"
#     cd "${TEST_DIR}/old-repo"
#     git init
#     git remote add origin "git@github.com:old-owner/old-repo.git"
    
#     "$GIT_REPO_NAME_PATH" config github-token ghp_testtoken123
#     run "$GIT_REPO_NAME_PATH" sync
    
#     [ "$status" -eq 0 ]
#     [[ $output =~ Updating\ remote\ URL\ to:\ https://github\.com/new-owner/new-repo\.git ]]
#     [[ $output =~ Renaming\ directory\ from\ \'old-repo\'\ to\ \'new-repo\' ]]
    
#     [ ! -d "${TEST_DIR}/old-repo" ]
#     [ -d "${TEST_DIR}/new-repo" ]
    
#     cd "${TEST_DIR}/new-repo"
#     [ "$(git remote get-url origin)" = "https://github.com/new-owner/new-repo.git" ]
# }

# @test "github/git protocol URL/when remote repo was renamed/renames local repo directory and updates remote URL" {
#     mkdir "${TEST_DIR}/old-repo"
#     cd "${TEST_DIR}/old-repo"
#     git init
#     git remote add origin "git://github.com/old-owner/old-repo.git"
    
#     "$GIT_REPO_NAME_PATH" config github-token ghp_testtoken123
#     run "$GIT_REPO_NAME_PATH" sync
    
#     [ "$status" -eq 0 ]
#     [[ $output =~ Updating\ remote\ URL\ to:\ https://github\.com/new-owner/new-repo\.git ]]
#     [[ $output =~ Renaming\ directory\ from\ \'old-repo\'\ to\ \'new-repo\' ]]
    
#     [ ! -d "${TEST_DIR}/old-repo" ]
#     [ -d "${TEST_DIR}/new-repo" ]
    
#     cd "${TEST_DIR}/new-repo"
#     [ "$(git remote get-url origin)" = "https://github.com/new-owner/new-repo.git" ]
# }

# @test "github/when no token configured/fails with helpful message" {
#     # Set up test repository
#     mkdir "${TEST_DIR}/old-repo"
#     cd "${TEST_DIR}/old-repo"
#     git init
#     git remote add origin "https://github.com/old-owner/old-repo.git"
    
#     # Run without configuring token
#     run "$GIT_REPO_NAME_PATH" sync
    
#     [ "$status" -ne 0 ]
#     [[ $output =~ No\ GitHub\ token\ found\ in\ configuration ]]
#     [[ $output =~ To\ set\ a\ GitHub\ token,\ run:\ git\ repo-name\ config\ github-token\ \<token\> ]]
# }