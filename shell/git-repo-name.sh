#!/bin/bash

# Shell wrapper for git-repo-name that handles directory changes when the 
# repository directory is renamed during 'git repo-name pull' operations.
__git_repo_name_wrapper() {
    tmp_file=$(mktemp /tmp/git-repo-name-output.XXXXXX)
    
    # tee writes the full (raw) output to the temporary file,
    # and sed filters out any machine marker lines from what's displayed.
    command git-repo-name-bin "$@" | tee "$tmp_file" | sed '/^GRN_DIR_CHANGE:/d'
    
    # Capture the original exit code
    rc=$?
    if [ -n "$BASH_VERSION" ]; then
        exit_code=${PIPESTATUS[0]}
    else
        exit_code=$rc
    fi
    
    # After the command finishes, look for our machine marker in the full (raw) output.
    marker_line=$(grep "^GRN_DIR_CHANGE:" "$tmp_file" 2>/dev/null)
    if [ -n "$marker_line" ]; then
        old_path=$(echo "$marker_line" | sed -E 's/GRN_DIR_CHANGE:([^:]*):([^:]*)/\1/')
        new_path=$(echo "$marker_line" | sed -E 's/GRN_DIR_CHANGE:([^:]*):([^:]*)/\2/')
        
        # Only change directory if we're in the original (old) directory.
        if [ "$PWD" = "$old_path" ]; then
            cd "$new_path" || return 1
        fi
    fi
    
    rm -f "$tmp_file"
    
    return "$exit_code"
}

# Define our function as the git-repo-name command
if [ -n "$BASH_VERSION" ] || [ -n "$ZSH_VERSION" ]; then
    alias git-repo-name=__git_repo_name_wrapper
fi

# If this script is being executed directly, run the command
if [ "$(basename "$0")" = "git-repo-name" ] || [ "$(basename "$0")" = "git-repo-name.sh" ]; then
    __git_repo_name_wrapper "$@"
fi 