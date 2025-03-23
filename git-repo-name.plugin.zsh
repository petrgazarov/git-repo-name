#!/bin/zsh
# Zsh plugin for git-repo-name.
# Use this file with OH‑MY‑ZSH (or zplug, zgen, antigen, etc.)
#
# This plugin sources the git-repo-name.sh script which defines the git-repo-name function.
#
# For Homebrew installations, the function definition is
# located at $(brew --prefix)/share/git-repo-name/git-repo-name.sh.
# Otherwise, we fall back to a relative path based on this file's location.

if [ -f "$(brew --prefix 2>/dev/null)/share/git-repo-name/git-repo-name.sh" ]; then
    source "$(brew --prefix)/share/git-repo-name/git-repo-name.sh"
else
    # "${0:A:h}" returns the directory containing this file (in Zsh).
    source "${0:A:h}/git-repo-name.sh"
fi 