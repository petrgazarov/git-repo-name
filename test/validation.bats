#!/usr/bin/env bats

set -e
load ./test_helper

@test "git sync/validation/fails with invalid source value" {
    # Create a git repository
    mkdir "${TEST_DIR}/repo"
    cd "${TEST_DIR}/repo"
    git init
    
    # Run git sync with invalid source
    run "$GIT_SYNC_PATH" --source=invalid
    
    # Assert
    [ "$status" -eq 2 ]  # Command line usage error
    [[ "$output" =~ invalid\ value\ for\ --source:\ \'invalid\'.\ Valid\ values\ are\ \'remote\'\ or\ \'local\' ]]
}

@test "git sync/validation/fails when not in git repository" {
    # Create a non-git directory
    mkdir "${TEST_DIR}/not-git"
    cd "${TEST_DIR}/not-git"
    
    # Run git sync
    run "$GIT_SYNC_PATH"
    
    # Assert
    [ "$status" -eq 128 ]
    [[ "$output" =~ fatal:\ not\ a\ git\ repository\ \(or\ any\ of\ the\ parent\ directories\):\ \.git ]]
}

@test "git sync/validation/fails when no remote configured" {
    # Create a git repo without remote
    mkdir "${TEST_DIR}/no-remote"
    cd "${TEST_DIR}/no-remote"
    git init
    
    run "$GIT_SYNC_PATH"
    
    [ "$status" -eq 2 ]
    [[ "$output" =~ error:\ No\ such\ remote\ \'origin\' ]]
} 