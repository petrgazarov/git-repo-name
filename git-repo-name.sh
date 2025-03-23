#!/bin/bash

# This file can be either executed directly or sourced in your shell startup file (e.g. ~/.bashrc or ~/.zshrc).
# To enable automatic PWD changes, source this file in your shell startup file.

git-repo-name() {
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
            builtin cd "$new_path" || return 1
        fi
    fi
    
    rm -f "$tmp_file"
    
    return "$exit_code"
}

if (return 0 2>/dev/null); then
    # Being sourced: do nothing. The shell function is defined above.
    :
else
    # Being executed: run the function.
    git-repo-name "$@"
fi