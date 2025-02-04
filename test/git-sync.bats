#!/usr/bin/env bats

set -e
load ./test_helper

setup() {
    # Call helper's setup first
    setup_helper

    # Then do our additional setup
    git init --bare "${TEST_DIR}/remote.git"
    git init "${TEST_DIR}/local"
    cd "${TEST_DIR}/local"
    git remote add origin "${TEST_DIR}/remote.git"
}

@test "git sync/with different names/renames local to match remote" {
    # Setup local repo with different name than remote
    local_name="old-name"
    remote_name="new-name"
    mv "${TEST_DIR}/local" "${TEST_DIR}/${local_name}"
    cd "${TEST_DIR}/${local_name}"
    
    # Setup remote with different name
    mv "${TEST_DIR}/remote.git" "${TEST_DIR}/${remote_name}.git"
    git remote set-url origin "${TEST_DIR}/${remote_name}.git"
    
    # Run git sync
    run "$GIT_SYNC_PATH"
    
    # Assert
    [ "$status" -eq 0 ]
    [[ "$output" =~ Renaming\ directory\ from\ \'${local_name}\'\ to\ \'${remote_name}\' ]]
    [ -d "${TEST_DIR}/${remote_name}" ]
    [ ! -d "${TEST_DIR}/${local_name}" ]
}

@test "git sync/when names match/handles all url formats" {
    cd "${TEST_DIR}/local"
    
    # Test different URL formats
    for url in \
        "https://github.com/user/repo-name.git" \
        "https://github.com/user/repo-name" \
        "git@github.com:user/repo-name.git" \
        "git@github.com:user/repo-name" \
        "ssh://git@github.com:22/user/repo-name.git" \
        "git://github.com/user/repo-name.git" \
        "https://gitlab.com/user/repo-name.git" \
        "git@gitlab.com:group/subgroup/repo-name.git" \
        "https://bitbucket.org/user/repo-name.git" \
        "/absolute/path/to/repo-name.git" \
        "file:///path/to/repo-name.git" \
        "../relative/path/to/repo-name.git"
    do
        git remote set-url origin "$url"
        run "$GIT_SYNC_PATH"
        [ "$status" -eq 0 ]
        [[ "$output" =~ Repository\ names\ already\ match:\ repo-name ]]
    done
}

@test "git sync/when names don't match/handles all url formats" {
    # Setup initial state
    local_name="old-repo-name"
    new_name="new-repo-name"
    
    for old_url in \
        "https://github.com/user/${local_name}.git" \
        "https://github.com/user/${local_name}" \
        "git@github.com:user/${local_name}.git" \
        "git@github.com:user/${local_name}" \
        "ssh://git@github.com:22/user/${local_name}.git" \
        "git://github.com/user/${local_name}.git" \
        "https://gitlab.com/user/${local_name}.git" \
        "git@gitlab.com:group/subgroup/${local_name}.git" \
        "https://bitbucket.org/user/${local_name}.git" \
        "/absolute/path/to/${local_name}.git" \
        "file:///path/to/${local_name}.git" \
        "../relative/path/to/${local_name}.git"
    do
        # Reset test state
        mv "${TEST_DIR}/local" "${TEST_DIR}/${local_name}"
        cd "${TEST_DIR}/${local_name}"
        git remote set-url origin "$old_url"
        
        # Create corresponding new URL format
        new_url="${old_url/${local_name}/${new_name}}"
        git remote set-url origin "$new_url"
        
        # Run git sync
        run "$GIT_SYNC_PATH"
        
        # Assert
        [ "$status" -eq 0 ]
        [[ "$output" =~ Renaming\ directory\ from\ \'${local_name}\'\ to\ \'${new_name}\' ]]
        [ -d "${TEST_DIR}/${new_name}" ]
        [ ! -d "${TEST_DIR}/${local_name}" ]
        
        # Reset for next iteration
        mv "${TEST_DIR}/${new_name}" "${TEST_DIR}/local"
    done
}