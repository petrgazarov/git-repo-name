use crate::{config::CONFIG, fs, git, Error, Result};
use git2::Repository;
use regex::Regex;
use reqwest::blocking::Client as ReqwestClient;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GitHubRepo {
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
}

pub fn get_repo_info(owner: &str, repo: &str) -> Result<GitHubRepo> {
    let client = create_client()?;
    let base_url = std::env::var("GITHUB_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.github.com".to_string());
    let url = format!("{}/repos/{}/{}", base_url, owner, repo);

    client
        .get(&url)
        .send()
        .map_err(|e| Error::GitHubApi(e.to_string()))?
        .error_for_status()
        .map_err(|e| Error::GitHubApi(e.to_string()))?
        .json()
        .map_err(|e| Error::GitHubApi(e.to_string()))
}

fn create_client() -> Result<ReqwestClient> {
    let token = CONFIG.get_github_token()?;
    let mut headers = HeaderMap::new();

    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("token {}", token))
            .map_err(|e| Error::GitHubApi(e.to_string()))?,
    );

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
        let remote = CONFIG.get_remote()?;
        if dry_run {
            println!(
                "Would change '{}' remote from '{}' to '{}'",
                remote, remote_url, resolved_remote_url
            );
        } else {
            println!(
                "Changing '{}' remote from '{}' to '{}'",
                remote, remote_url, resolved_remote_url
            );
            git::set_remote_url(repo, &remote, &resolved_remote_url)?;
        }
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
}

#[cfg(test)]
mod sync_from_github_remote_tests {
    use super::*;
    use crate::test_helpers;
    use assert_fs::prelude::*;
    use mockito::{Mock, ServerGuard};
    use predicates::prelude::*;
    use std::env;

    // Test fixture to reduce repetition in tests
    struct SyncTestFixture {
        temp: assert_fs::TempDir,
        repo_dir: std::path::PathBuf,
        repo: git2::Repository,
        mock_server: ServerGuard,
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

            // Start a mock server
            let mock_server = mockito::Server::new();
            env::set_var("GITHUB_API_BASE_URL", mock_server.url());

            Ok(Self {
                temp,
                repo_dir,
                repo,
                mock_server,
                _guard: guard,
            })
        }

        // Set up the remote with the given URL
        fn setup_remote(&self, remote_url: &str) -> anyhow::Result<()> {
            // Remove the remote if it exists
            match self.repo.find_remote("origin") {
                Ok(_) => {
                    self.repo.remote_delete("origin")?;
                }
                Err(_) => {
                    // Remote doesn't exist, which is fine
                }
            }
            self.repo.remote("origin", remote_url)?;
            Ok(())
        }

        // Run the sync operation and return the output
        fn run_sync(&self, remote_url: &str, dry_run: bool) -> anyhow::Result<String> {
            let (output, _) = test_helpers::capture_stdout(|| {
                sync_from_github_remote(&self.repo, remote_url, dry_run)
            })?;

            Ok(output)
        }

        // Helper to check if directory exists
        fn assert_directory_exists(&self, name: &str, should_exist: bool) -> anyhow::Result<()> {
            let path = self.temp.child(name);

            if should_exist {
                path.assert(predicate::path::exists());
            } else {
                path.assert(predicate::path::missing());
            }
            Ok(())
        }

        // Helper to check remote URL
        fn assert_remote_url(&self, expected_url: &str) -> anyhow::Result<()> {
            let remote_url = git::get_remote_url(&self.repo, &CONFIG.get_remote()?)?;
            assert_eq!(remote_url, expected_url);
            Ok(())
        }

        // Mock GitHub API response for a repository
        fn mock_github_repo(
            &mut self,
            old_owner: &str,
            new_owner: &str,
            old_repo_name: &str,
            new_repo_name: &str,
        ) -> Mock {
            let response_body = serde_json::json!({
                "name": new_repo_name,
                "full_name": format!("{}/{}", new_owner, new_repo_name),
                // GitHub API always returns HTTPS URLs regardless of the request URL format
                "clone_url": format!("https://github.com/{}/{}.git", new_owner, new_repo_name)
            });

            self.mock_server
                .mock(
                    "GET",
                    format!("/repos/{}/{}", old_owner, old_repo_name).as_str(),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(response_body.to_string())
                .create()
        }

        // Mock GitHub API error response
        fn mock_github_error(&mut self, owner: &str, repo: &str, status: usize) -> Mock {
            self.mock_server
                .mock("GET", format!("/repos/{}/{}", owner, repo).as_str())
                .with_status(status)
                .with_header("content-type", "application/json")
                .with_body(r#"{"message": "Not Found"}"#)
                .create()
        }
    }

    #[test]
    fn test_sync_up_to_date_dry_run() -> anyhow::Result<()> {
        let mut fixture = SyncTestFixture::new("test-repo")?;
        let remote_url = "https://github.com/owner/test-repo.git";

        // Set up the mock to return the same repo name
        fixture.mock_github_repo("owner", "owner", "test-repo", "test-repo");

        // Set up the remote
        fixture.setup_remote(remote_url)?;

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
        fixture.assert_directory_exists("test-repo", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_up_to_date() -> anyhow::Result<()> {
        let mut fixture = SyncTestFixture::new("test-repo")?;
        let remote_url = "https://github.com/owner/test-repo.git";

        // Set up the mock to return the same repo name
        fixture.mock_github_repo("owner", "owner", "test-repo", "test-repo");

        // Set up the remote
        fixture.setup_remote(remote_url)?;

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
        fixture.assert_directory_exists("test-repo", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_remote_url_update_dry_run() -> anyhow::Result<()> {
        let mut fixture = SyncTestFixture::new("repo-name")?;
        let old_url = "git@github.com:old-owner/repo-name.git";
        let expected_new_url = "git@github.com:new-owner/repo-name.git";

        // Set up the mock to return the same repo name but with new owner
        fixture.mock_github_repo("old-owner", "new-owner", "repo-name", "repo-name");

        // Set up the remote
        fixture.setup_remote(old_url)?;

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
        fixture.assert_directory_exists("repo-name", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_remote_url_update() -> anyhow::Result<()> {
        let mut fixture = SyncTestFixture::new("repo-name")?;
        let old_url = "git@github.com:old-owner/repo-name.git";
        let expected_new_url = "git@github.com:new-owner/repo-name.git";

        // Set up the mock to return the same repo name but with HTTPS URL
        fixture.mock_github_repo("old-owner", "new-owner", "repo-name", "repo-name");

        // Set up the remote with SSH URL
        fixture.setup_remote(old_url)?;

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
        fixture.assert_directory_exists("repo-name", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_directory_rename_dry_run() -> anyhow::Result<()> {
        let mut fixture = SyncTestFixture::new("old-name")?;
        let remote_url = "https://github.com/owner/new-name.git";

        // Set up the mock to return a different repo name
        fixture.mock_github_repo("owner", "owner", "new-name", "new-name");

        // Set up the remote
        fixture.setup_remote(remote_url)?;

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
        fixture.assert_directory_exists("old-name", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_directory_rename() -> anyhow::Result<()> {
        let mut fixture = SyncTestFixture::new("old-name")?;
        let remote_url = "https://github.com/owner/new-name.git";

        // Set up the mock to return a different repo name
        fixture.mock_github_repo("owner", "owner", "new-name", "new-name");

        // Set up the remote
        fixture.setup_remote(remote_url)?;

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
        fixture.assert_directory_exists("old-name", false)?;
        fixture.assert_directory_exists("new-name", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_both_updates_dry_run() -> anyhow::Result<()> {
        let mut fixture = SyncTestFixture::new("old-name")?;
        let old_url = "git@github.com:old-owner/old-name.git";
        let expected_new_url = "git@github.com:new-owner/new-name.git";

        // Set up the mock to return a different repo name
        fixture.mock_github_repo("old-owner", "new-owner", "old-name", "new-name");

        // Set up the remote with SSH URL
        fixture.setup_remote(old_url)?;

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
        fixture.assert_directory_exists("old-name", true)?;
        fixture.assert_directory_exists("new-name", false)?;

        Ok(())
    }

    #[test]
    fn test_sync_both_updates() -> anyhow::Result<()> {
        let mut fixture = SyncTestFixture::new("old-name")?;
        let old_url = "git@github.com:old-owner/old-name.git";
        let expected_new_url = "git@github.com:new-owner/new-name.git";

        // Set up the mock to return a different repo name
        fixture.mock_github_repo("old-owner", "new-owner", "old-name", "new-name");

        // Set up the remote with SSH URL
        fixture.setup_remote(old_url)?;

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
        fixture.assert_directory_exists("old-name", false)?;
        fixture.assert_directory_exists("new-name", true)?;

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
            let mut fixture = SyncTestFixture::new("test-repo")?;

            // Set up the mock to return the same repo name
            fixture.mock_github_repo("owner", "owner", "test-repo", "test-repo");

            // Set up the remote
            fixture.setup_remote(url)?;

            // Run sync
            let result = fixture.run_sync(url, false);
            assert!(result.is_ok(), "Failed with URL format: {}", url);
        }

        Ok(())
    }

    #[test]
    fn test_sync_nonexistent_github_repo() -> anyhow::Result<()> {
        let mut fixture = SyncTestFixture::new("test-repo")?;
        let remote_url = "git@github.com:owner/test-repo.git";

        // Set up the mock to return 404
        fixture.mock_github_error("owner", "test-repo", 404);

        // Set up the remote
        fixture.setup_remote(remote_url)?;

        let result = fixture.run_sync(remote_url, false);

        match result {
            Err(e) => {
                assert!(
                    e.to_string().contains("404"),
                    "Expected 404 error message, got: {}",
                    e
                );
                Ok(())
            }
            Ok(_) => panic!("Expected error, but operation succeeded"),
        }
    }
}
