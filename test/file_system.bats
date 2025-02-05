#!/usr/bin/env bats

set -e
load ./test_helper

setup() {
    # Call helper's setup first
    setup_helper

    # Then do our additional setup
    git init "${TEST_DIR}/local"
}

teardown() {
    teardown_helper
}

@test "file system source/matching names/does not do anything" {
    cd "${TEST_DIR}/local"
    
    # Test different URL formats
    for url in \
        "/absolute/path/to/local.git" \
        "file:///path/to/local.git" \
        "../relative/path/to/local.git"
    do
        git remote add origin "$url"
        run "$GIT_REPO_NAME_PATH" sync
        [ "$status" -eq 0 ]
        [[ "$output" =~ Repository\ names\ already\ match:\ local ]]
        # Verify directory wasn't renamed
        [ -d "${TEST_DIR}/local" ]
        
        # Clean up for next iteration
        git remote remove origin
    done
}

@test "file system source/when remote repo was renamed/renames local repo directory" {
    # Setup initial state    
    old_path="${TEST_DIR}/local"
    new_repo_name="new-repo-name"
    new_path="${TEST_DIR}/${new_repo_name}"

    for new_url in \
        "/absolute/path/to/${new_repo_name}.git" \
        "file:///path/to/${new_repo_name}.git" \
        "../relative/path/to/${new_repo_name}.git"
    do
        cd "${old_path}"
        git remote add origin "$new_url"
        
        # Run git repo-name sync
        run "$GIT_REPO_NAME_PATH" sync
        
        # Assert
        [ "$status" -eq 0 ]
        [[ "$output" =~ Renaming\ directory\ from\ \'local\'\ to\ \'${new_repo_name}\' ]]
        [ -d "${new_path}" ]
        [ ! -d "${old_path}" ]
        
        # Reset for next iteration
        mv "${new_path}" "${old_path}"
        git remote remove origin
    done
}

@test "file system source/when remote repo was renamed/works when run from a nested directory within local repo" {
    # Setup initial state    
    old_path="${TEST_DIR}/local"
    new_repo_name="new-repo-name"
    new_path="${TEST_DIR}/${new_repo_name}"
    nested_directory="nested/deeper/path"

    # Create a nested directory structure
    mkdir -p "${old_path}/${nested_directory}"
    
    for new_url in \
        "/absolute/path/to/${new_repo_name}.git" \
        "file:///path/to/${new_repo_name}.git" \
        "../relative/path/to/${new_repo_name}.git"
    do
        cd "${old_path}/${nested_directory}"
        git remote add origin "$new_url"
        
        # Run git repo-name sync from nested directory
        run "$GIT_REPO_NAME_PATH" sync
        
        # Assert
        [ "$status" -eq 0 ]
        [[ "$output" =~ Renaming\ directory\ from\ \'local\'\ to\ \'${new_repo_name}\' ]]
        [ -d "${new_path}" ]
        [ ! -d "${old_path}" ]
        [ -d "${new_path}/${nested_directory}" ]
        
        # Reset for next iteration
        mv "${new_path}" "${old_path}"
        git remote remove origin
    done
}

