use crate::{
    config::CONFIG,
    git,
    types::{Error, Result},
    utils::fs,
};
use git2::Repository;
use regex::Regex;
use reqwest::blocking::Client as ReqwestClient;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest::StatusCode;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GitHubRepo {
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
}

pub fn get_repo_info(owner: &str, repo: &str) -> Result<GitHubRepo> {
    let base_url = std::env::var("GITHUB_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.github.com".to_string());
    let url = format!("{}/repos/{}/{}", base_url, owner, repo);

    let token = CONFIG.get_github_token().ok();
    let client = create_client(token.as_deref())?;
    let response = client.get(&url).send();

    match response {
        Ok(resp) => {
            if resp.status() == StatusCode::NOT_FOUND {
                // GitHub returns 404 for private repos when unauthorized
                Err(Error::GitHubApi(
                    "Repository not found. If this is a private repository, please configure a GitHub token with 'git repo-name config github-token YOUR_TOKEN'".to_string(),
                ))
            } else {
                // Process successful response
                match resp.error_for_status() {
                    Ok(resp) => resp.json().map_err(|e| Error::GitHubApi(e.to_string())),
                    Err(e) => Err(Error::GitHubApi(e.to_string())),
                }
            }
        }
        Err(e) => Err(Error::GitHubApi(e.to_string())),
    }
}

fn create_client(token: Option<&str>) -> Result<ReqwestClient> {
    let mut headers = HeaderMap::new();

    // Add authorization header only if token is provided
    if let Some(token_str) = token {
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("token {}", token_str))
                .map_err(|e| Error::GitHubApi(e.to_string()))?,
        );
    }

    headers.insert(USER_AGENT, HeaderValue::from_static("git-repo-name"));

    ReqwestClient::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| Error::GitHubApi(e.to_string()))
}

pub fn is_github_url(url: &str) -> bool {
    let re = Regex::new(r"^(?:https://(?:www\.)?github\.com/|git@github\.com:|ssh://git@github\.com/|git://github\.com/)[^/]+/[^/\s]+(?:\.git)?$").unwrap();
    re.is_match(url)
}

pub fn parse_github_url(url: &str) -> Result<(String, String)> {
    let re = Regex::new(r"^(?:https://(?:www\.)?github\.com/|git@github\.com:|ssh://git@github\.com/|git://github\.com/)([^/]+)/([^/\.]+?)(?:\.git)?$").unwrap();

    let caps = re
        .captures(url)
        .ok_or_else(|| Error::InvalidGitHubUrl(url.to_string()))?;

    let owner = caps
        .get(1)
        .ok_or_else(|| Error::InvalidGitHubUrl(url.to_string()))?
        .as_str()
        .to_string();

    let repo = caps
        .get(2)
        .ok_or_else(|| Error::InvalidGitHubUrl(url.to_string()))?
        .as_str()
        .to_string();

    Ok((owner, repo))
}

fn format_new_remote_url(original_remote_url: &str, owner: &str, repo_name: &str) -> String {
    if original_remote_url.starts_with("git@") {
        // SSH shorthand (e.g. git@github.com:owner/repo.git)
        format!("git@github.com:{}/{}.git", owner, repo_name)
    } else if original_remote_url.starts_with("ssh://") {
        // Full SSH URL (e.g. ssh://git@github.com/owner/repo.git)
        format!("ssh://git@github.com/{}/{}.git", owner, repo_name)
    } else if original_remote_url.starts_with("git://") {
        // Git protocol (e.g. git://github.com/owner/repo.git)
        format!("git://github.com/{}/{}.git", owner, repo_name)
    } else {
        // Otherwise default to HTTPS.
        format!("https://github.com/{}/{}.git", owner, repo_name)
    }
}

pub fn sync_from_github_remote(repo: &Repository, remote_url: &str, dry_run: bool) -> Result<()> {
    let (owner, remote_repo_name) = parse_github_url(remote_url)?;
    let local_directory_name = repo
        .workdir()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?
        .file_name()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?
        .to_str()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?
        .to_string();
    let repo_info = get_repo_info(&owner, &remote_repo_name)?;
    let resolved_repo_name = repo_info.name;
    let resolved_owner = repo_info.full_name.split('/').next().unwrap_or(&owner);

    let repo_path = repo
        .workdir()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?;

    let resolved_remote_url =
        format_new_remote_url(remote_url, resolved_owner, &resolved_repo_name);
    let should_rename_directory = local_directory_name != resolved_repo_name;
    let should_change_remote = resolved_remote_url != remote_url;

    if !should_rename_directory && !should_change_remote {
        println!("Directory name and remote URL already up-to-date");
        return Ok(());
    }

    if should_change_remote {
        git::set_remote_url(repo, remote_url, &resolved_remote_url, dry_run)?;
    }

    if should_rename_directory {
        fs::rename_directory(repo_path, &resolved_repo_name, dry_run)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url() {
        let test_cases = vec![
            // HTTPS URLs
            ("https://github.com/owner/repo.git", ("owner", "repo")),
            ("https://github.com/owner/repo", ("owner", "repo")),
            ("https://www.github.com/owner/repo.git", ("owner", "repo")),
            ("https://www.github.com/owner/repo", ("owner", "repo")),
            // SSH URLs
            ("git@github.com:owner/repo.git", ("owner", "repo")),
            ("git@github.com:owner/repo", ("owner", "repo")),
            ("ssh://git@github.com/owner/repo.git", ("owner", "repo")),
            ("ssh://git@github.com/owner/repo", ("owner", "repo")),
            // Git protocol URLs
            ("git://github.com/owner/repo.git", ("owner", "repo")),
            ("git://github.com/owner/repo", ("owner", "repo")),
        ];

        for (url, (expected_owner, expected_repo)) in test_cases {
            let (owner, repo) = parse_github_url(url).unwrap();
            assert_eq!(owner, expected_owner);
            assert_eq!(repo, expected_repo);
        }
    }

    #[test]
    fn test_is_github_url() {
        // Valid URLs
        assert!(is_github_url("https://github.com/owner/repo.git"));
        assert!(is_github_url("https://github.com/owner/repo"));
        assert!(is_github_url("https://www.github.com/owner/repo.git"));
        assert!(is_github_url("https://www.github.com/owner/repo"));
        assert!(is_github_url("git@github.com:owner/repo.git"));
        assert!(is_github_url("git@github.com:owner/repo"));
        assert!(is_github_url("ssh://git@github.com/owner/repo.git"));
        assert!(is_github_url("ssh://git@github.com/owner/repo"));
        assert!(is_github_url("git://github.com/owner/repo.git"));
        assert!(is_github_url("git://github.com/owner/repo"));

        // Invalid URLs
        assert!(!is_github_url("https://gitlab.com/owner/repo.git"));
        assert!(!is_github_url("git@gitlab.com:owner/repo.git"));
        assert!(!is_github_url("https://github.com"));
        assert!(!is_github_url("git@github.com:"));
    }

    #[test]
    fn test_format_new_remote_url() {
        let cases = vec![
            // (original_remote_url, owner, repo_name, expected_remote_url)
            (
                "git@github.com:oldowner/oldrepo.git",
                "newowner",
                "newrepo",
                "git@github.com:newowner/newrepo.git",
            ),
            (
                "ssh://git@github.com/oldowner/oldrepo.git",
                "newowner",
                "newrepo",
                "ssh://git@github.com/newowner/newrepo.git",
            ),
            (
                "git://github.com/oldowner/oldrepo.git",
                "newowner",
                "newrepo",
                "git://github.com/newowner/newrepo.git",
            ),
            (
                "https://github.com/oldowner/oldrepo.git",
                "newowner",
                "newrepo",
                "https://github.com/newowner/newrepo.git",
            ),
            (
                "https://github.com/oldowner/oldrepo",
                "newowner",
                "newrepo",
                "https://github.com/newowner/newrepo.git",
            ),
            (
                "http://github.com/oldowner/oldrepo.git",
                "newowner",
                "newrepo",
                "https://github.com/newowner/newrepo.git",
            ),
        ];
        for (original, owner, repo_name, expected) in cases {
            assert_eq!(format_new_remote_url(original, owner, repo_name), expected);
        }
    }

    #[test]
    fn test_get_repo_info() -> anyhow::Result<()> {
        use crate::config::CONFIG;
        use crate::test_helpers;
        use assert_fs::TempDir;

        // Create a temp dir for test config
        let temp = TempDir::new()?;
        test_helpers::setup_test_config(temp.path())?;

        // Directory guard to restore working directory when test completes
        let _guard = test_helpers::CurrentDirGuard::new();

        // Test data
        let owner = "test-owner";
        let repo = "test-repo";

        // Mock the GitHub API response for public repo
        test_helpers::mock_github_repo(owner, owner, repo, repo);

        // Test unauthenticated request - remove the token
        {
            // Clear the token in our test configuration
            CONFIG.set_github_token("")?;

            // Test with no token
            let result = get_repo_info(owner, repo);
            assert!(
                result.is_ok(),
                "Expected success for public repo with unauthenticated request"
            );
        }

        // Setup a mock that returns 404 for private repo
        let private_repo = format!("{}-private", repo);
        test_helpers::mock_github_error(owner, &private_repo, 404);

        // Test private repo with unauthenticated request
        {
            // Still using empty token
            let result = get_repo_info(owner, &private_repo);
            assert!(
                result.is_err(),
                "Expected error for private repo with unauthenticated request"
            );

            let err = result.unwrap_err();
            assert!(
                err.to_string().contains("private repository"),
                "Error should mention private repository, got: {}",
                err
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod sync_from_github_remote_tests {
    use super::*;
    use crate::test_helpers;

    // Test fixture to reduce repetition in tests
    struct SyncTestFixture {
        temp: assert_fs::TempDir,
        repo_dir: std::path::PathBuf,
        repo: git2::Repository,
        _guard: test_helpers::CurrentDirGuard,
    }

    impl SyncTestFixture {
        // Create a new test fixture with the given repo names
        fn new(local_repo_name: &str) -> anyhow::Result<Self> {
            let guard = test_helpers::CurrentDirGuard::new();
            let temp = assert_fs::TempDir::new()?;
            test_helpers::setup_test_config(temp.path())?;

            let (repo_dir, repo) = test_helpers::create_main_repo(&temp, local_repo_name)?;
            std::env::set_current_dir(&repo_dir)?;

            Ok(Self {
                temp,
                repo_dir,
                repo,
                _guard: guard,
            })
        }

        // Run the sync operation and return the output
        fn run_sync(&self, remote_url: &str, dry_run: bool) -> anyhow::Result<String> {
            let (output, _) = test_helpers::capture_stdout(|| {
                sync_from_github_remote(&self.repo, remote_url, dry_run)
            })?;
            Ok(output)
        }

        // Helper to check remote URL
        fn assert_remote_url(&self, expected_url: &str) -> anyhow::Result<()> {
            let remote_url = git::get_remote_url(&self.repo)?;
            assert_eq!(remote_url, expected_url);
            Ok(())
        }

        // Mock GitHub API response for a repository
        fn mock_github_repo(
            &self,
            old_owner: &str,
            new_owner: &str,
            old_repo_name: &str,
            new_repo_name: &str,
        ) {
            test_helpers::mock_github_repo(old_owner, new_owner, old_repo_name, new_repo_name)
        }

        // Mock GitHub API error response
        fn mock_github_error(&self, owner: &str, repo: &str, status: usize) {
            test_helpers::mock_github_error(owner, repo, status)
        }
    }

    #[test]
    fn test_sync_up_to_date_dry_run() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("test-repo")?;
        let remote_url = "https://github.com/owner/test-repo.git";

        // Set up the mock to return the same repo name
        fixture.mock_github_repo("owner", "owner", "test-repo", "test-repo");

        // Set up the remote
        fixture.repo.remote("origin", remote_url)?;

        // Run sync in dry run mode
        let output = fixture.run_sync(remote_url, true)?;

        // Verify output
        assert!(
            output.contains("Directory name and remote URL already up-to-date"),
            "Expected up-to-date message, got: {}",
            output
        );

        // Verify no changes were made
        fixture.assert_remote_url(remote_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "test-repo", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_up_to_date() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("test-repo")?;
        let remote_url = "https://github.com/owner/test-repo.git";

        // Set up the mock to return the same repo name
        fixture.mock_github_repo("owner", "owner", "test-repo", "test-repo");

        // Set up the remote
        fixture.repo.remote("origin", remote_url)?;

        // Run sync
        let output = fixture.run_sync(remote_url, false)?;

        // Verify output
        assert!(
            output.contains("Directory name and remote URL already up-to-date"),
            "Expected up-to-date message, got: {}",
            output
        );

        // Verify no changes were made
        fixture.assert_remote_url(remote_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "test-repo", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_remote_url_update_dry_run() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("repo-name")?;
        let old_url = "git@github.com:old-owner/repo-name.git";
        let expected_new_url = "git@github.com:new-owner/repo-name.git";

        // Set up the mock to return the same repo name but with new owner
        fixture.mock_github_repo("old-owner", "new-owner", "repo-name", "repo-name");

        // Set up the remote
        fixture.repo.remote("origin", old_url)?;

        // Run sync in dry run mode
        let output = fixture.run_sync(old_url, true)?;

        // Verify output shows the URL would be changed
        assert!(
            output.contains(&format!(
                "Would change 'origin' remote from '{}' to '{}'",
                old_url, expected_new_url
            )),
            "Expected remote URL update message, got: {}",
            output
        );

        // Verify no changes were actually made
        fixture.assert_remote_url(old_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "repo-name", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_remote_url_update() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("repo-name")?;
        let old_url = "git@github.com:old-owner/repo-name.git";
        let expected_new_url = "git@github.com:new-owner/repo-name.git";

        // Set up the mock to return the same repo name but with HTTPS URL
        fixture.mock_github_repo("old-owner", "new-owner", "repo-name", "repo-name");

        // Set up the remote with SSH URL
        fixture.repo.remote("origin", old_url)?;

        // Run sync
        let output = fixture.run_sync(old_url, false)?;

        // Verify output shows the URL was changed
        assert!(
            output.contains(&format!(
                "Changing 'origin' remote from '{}' to '{}'",
                old_url, expected_new_url
            )),
            "Expected remote URL update message, got: {}",
            output
        );

        // Verify remote URL was updated but directory name stayed the same
        fixture.assert_remote_url(expected_new_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "repo-name", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_directory_rename_dry_run() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("old-name")?;
        let remote_url = "https://github.com/owner/new-name.git";

        // Set up the mock to return a different repo name
        fixture.mock_github_repo("owner", "owner", "new-name", "new-name");

        // Set up the remote
        fixture.repo.remote("origin", remote_url)?;

        // Run sync in dry run mode
        let output = fixture.run_sync(remote_url, true)?;

        let parent_dir = fixture.repo_dir.parent().unwrap().canonicalize()?;

        // Verify output shows directory would be renamed
        assert!(
            output.contains(&format!(
                "Would rename directory from '{}' to '{}'",
                parent_dir.join("old-name").display(),
                parent_dir.join("new-name").display()
            )),
            "Expected directory rename message, got: {}",
            output
        );

        // Verify no changes were made
        fixture.assert_remote_url(remote_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "old-name", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_directory_rename() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("old-name")?;
        let remote_url = "https://github.com/owner/new-name.git";

        // Set up the mock to return a different repo name
        fixture.mock_github_repo("owner", "owner", "new-name", "new-name");

        // Set up the remote
        fixture.repo.remote("origin", remote_url)?;

        // Run sync
        let output = fixture.run_sync(remote_url, false)?;

        let parent_dir = fixture.repo_dir.parent().unwrap().canonicalize()?;

        // Verify output shows directory was renamed
        assert!(
            output.contains(&format!(
                "Renaming directory from '{}' to '{}'",
                parent_dir.join("old-name").display(),
                parent_dir.join("new-name").display()
            )),
            "Expected directory rename message, got: {}",
            output
        );

        // Verify directory was renamed but remote URL stayed the same
        fixture.assert_remote_url(remote_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "old-name", false)?;
        test_helpers::assert_directory_existence(&fixture.temp, "new-name", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_both_updates_dry_run() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("old-name")?;
        let old_url = "git@github.com:old-owner/old-name.git";
        let expected_new_url = "git@github.com:new-owner/new-name.git";

        // Set up the mock to return a different repo name
        fixture.mock_github_repo("old-owner", "new-owner", "old-name", "new-name");

        // Set up the remote with SSH URL
        fixture.repo.remote("origin", old_url)?;

        // Run sync in dry run mode
        let output = fixture.run_sync(old_url, true)?;

        let parent_dir = fixture.repo_dir.parent().unwrap().canonicalize()?;

        // Verify output shows both changes would be made
        assert!(
            output.contains(&format!(
                "Would change 'origin' remote from '{}' to '{}'",
                old_url, expected_new_url
            )),
            "Expected remote URL update message, got: {}",
            output
        );
        assert!(
            output.contains(&format!(
                "Would rename directory from '{}' to '{}'",
                parent_dir.join("old-name").display(),
                parent_dir.join("new-name").display()
            )),
            "Expected directory rename message, got: {}",
            output
        );

        // Verify no changes were made
        fixture.assert_remote_url(old_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "old-name", true)?;
        test_helpers::assert_directory_existence(&fixture.temp, "new-name", false)?;

        Ok(())
    }

    #[test]
    fn test_sync_both_updates() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("old-name")?;
        let old_url = "git@github.com:old-owner/old-name.git";
        let expected_new_url = "git@github.com:new-owner/new-name.git";

        // Set up the mock to return a different repo name
        fixture.mock_github_repo("old-owner", "new-owner", "old-name", "new-name");

        // Set up the remote with SSH URL
        fixture.repo.remote("origin", old_url)?;

        // Run sync
        let output = fixture.run_sync(old_url, false)?;

        let parent_dir = fixture.repo_dir.parent().unwrap().canonicalize()?;

        // Verify output shows both changes were made
        assert!(
            output.contains(&format!(
                "Changing 'origin' remote from '{}' to '{}'",
                old_url, expected_new_url
            )),
            "Expected remote URL update message, got: {}",
            output
        );
        assert!(
            output.contains(&format!(
                "Renaming directory from '{}' to '{}'",
                parent_dir.join("old-name").display(),
                parent_dir.join("new-name").display()
            )),
            "Expected directory rename message, got: {}",
            output
        );

        // Verify both changes were made
        fixture.assert_remote_url(expected_new_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "old-name", false)?;
        test_helpers::assert_directory_existence(&fixture.temp, "new-name", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_invalid_github_url() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("test-repo")?;
        let invalid_url = "https://not-github.com/owner/repo.git";

        let result = fixture.run_sync(invalid_url, false);

        match result {
            Err(e) => {
                assert!(
                    e.to_string().contains("Invalid GitHub URL"),
                    "Expected InvalidGitHubUrl error, got: {}",
                    e
                );
                Ok(())
            }
            Ok(_) => panic!("Expected error, but operation succeeded"),
        }
    }

    #[test]
    fn test_sync_different_url_formats() -> anyhow::Result<()> {
        let test_cases = vec![
            "https://github.com/owner/test-repo.git",
            "git@github.com:owner/test-repo.git",
            "ssh://git@github.com/owner/test-repo.git",
            "git://github.com/owner/test-repo.git",
        ];

        for url in test_cases {
            let fixture = SyncTestFixture::new("test-repo")?;

            // Set up the mock to return the same repo name
            fixture.mock_github_repo("owner", "owner", "test-repo", "test-repo");

            // Set up the remote
            fixture.repo.remote("origin", url)?;

            // Run sync
            let result = fixture.run_sync(url, false);
            assert!(result.is_ok(), "Failed with URL format: {}", url);

            fixture.repo.remote_delete("origin")?;
        }

        Ok(())
    }

    #[test]
    fn test_sync_nonexistent_github_repo() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("test-repo")?;
        let remote_url = "git@github.com:owner/test-repo.git";

        // Set up the mock to return 404
        fixture.mock_github_error("owner", "test-repo", 404);

        // Set up the remote
        fixture.repo.remote("origin", remote_url)?;

        let result = fixture.run_sync(remote_url, false);

        match result {
            Err(e) => {
                assert!(
                    e.to_string().contains("Repository not found"),
                    "Expected 'Repository not found' error message, got: {}",
                    e
                );
                Ok(())
            }
            Ok(_) => panic!("Expected error, but operation succeeded"),
        }
    }
}
