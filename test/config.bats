#!/usr/bin/env bats

set -e
load ./test_helper

setup() {
  setup_helper
  # Use temporary config directory for tests
  export XDG_CONFIG_HOME="${TEST_DIR}/.config"
}

teardown() {
  teardown_helper
}

@test "config/rejects invalid subcommand" {
  run "$GIT_REPO_NAME_PATH" config invalid-key
  [ "$status" -ne 0 ]
  [[ "$output" =~ "Unknown config key: invalid-key" ]]
  [[ "$output" =~ "Valid keys: github-token" ]]
}

@test "config/github-token (setter)/stores token securely" {
  run "$GIT_REPO_NAME_PATH" config github-token ghp_testtoken123
  [ "$status" -eq 0 ]
  [[ "$output" =~ ${TEST_DIR}/.config/git-repo-name/credentials ]]
  
  # Verify file permissions
  [ -d "${TEST_DIR}/.config/git-repo-name" ]
  [ "$(stat -f %A "${TEST_DIR}/.config/git-repo-name")" = "700" ]
  
  [ -f "${TEST_DIR}/.config/git-repo-name/credentials" ]
  [ "$(stat -f %A "${TEST_DIR}/.config/git-repo-name/credentials")" = "600" ]
  
  # Verify token content
  run grep "ghp_testtoken123" "${TEST_DIR}/.config/git-repo-name/credentials"
  [ "$status" -eq 0 ]
}

@test "config/github-token (getter)/retrieves stored token" {
  # First set the token
  run "$GIT_REPO_NAME_PATH" config github-token ghp_testtoken456
  
  # Test get command
  run "$GIT_REPO_NAME_PATH" config github-token
  [ "$status" -eq 0 ]
  [ "$output" = "ghp_testtoken456" ]
}

@test "config/github-token (getter)/fails when no token set" {
  run "$GIT_REPO_NAME_PATH" config github-token
  [ "$status" -ne 0 ]
  [[ "$output" =~ "No GitHub token found in configuration" ]]
}