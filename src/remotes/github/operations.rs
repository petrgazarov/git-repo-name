use crate::{
    git,
    remotes::github::{client::get_repo_info, url::format_new_remote_url, url::parse_github_url},
    types::{Error, Result},
    utils::fs,
};
use git2::Repository;

pub fn pull_from_github_remote(repo: &Repository, remote_url: &str, dry_run: bool) -> Result<()> {
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

pub fn push_to_github_remote(_repo: &Repository, _remote_url: &str, _dry_run: bool) -> Result<()> {
    println!("TODO: Implement push to GitHub remote");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;

    struct PullTestFixture {
        temp: assert_fs::TempDir,
        repo_dir: std::path::PathBuf,
        repo: git2::Repository,
        _guard: test_helpers::CurrentDirGuard,
    }

    impl PullTestFixture {
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

        fn run_pull(&self, remote_url: &str, dry_run: bool) -> anyhow::Result<String> {
            let (output, _) = test_helpers::capture_stdout(|| {
                pull_from_github_remote(&self.repo, remote_url, dry_run)
            })?;
            Ok(output)
        }

        fn assert_remote_url(&self, expected_url: &str) -> anyhow::Result<()> {
            let remote_url = git::get_remote_url(&self.repo)?;
            assert_eq!(remote_url, expected_url);
            Ok(())
        }

        fn mock_github_repo(
            &self,
            old_owner: &str,
            new_owner: &str,
            old_repo_name: &str,
            new_repo_name: &str,
        ) {
            test_helpers::mock_github_repo(old_owner, new_owner, old_repo_name, new_repo_name)
        }

        fn mock_github_error(&self, owner: &str, repo: &str, status: usize) {
            test_helpers::mock_github_error(owner, repo, status)
        }
    }

    #[test]
    fn test_pull_up_to_date_dry_run() -> anyhow::Result<()> {
        let fixture = PullTestFixture::new("test-repo")?;
        let remote_url = "https://github.com/owner/test-repo.git";

        fixture.mock_github_repo("owner", "owner", "test-repo", "test-repo");
        fixture.repo.remote("origin", remote_url)?;

        let output = fixture.run_pull(remote_url, true)?;

        assert!(
            output.contains("Directory name and remote URL already up-to-date"),
            "Expected up-to-date message, got: {}",
            output
        );

        fixture.assert_remote_url(remote_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "test-repo", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_up_to_date() -> anyhow::Result<()> {
        let fixture = PullTestFixture::new("test-repo")?;
        let remote_url = "https://github.com/owner/test-repo.git";

        fixture.mock_github_repo("owner", "owner", "test-repo", "test-repo");
        fixture.repo.remote("origin", remote_url)?;

        let output = fixture.run_pull(remote_url, false)?;

        assert!(
            output.contains("Directory name and remote URL already up-to-date"),
            "Expected up-to-date message, got: {}",
            output
        );

        fixture.assert_remote_url(remote_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "test-repo", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_remote_url_update_dry_run() -> anyhow::Result<()> {
        let fixture = PullTestFixture::new("repo-name")?;
        let old_url = "git@github.com:old-owner/repo-name.git";
        let expected_new_url = "git@github.com:new-owner/repo-name.git";

        fixture.mock_github_repo("old-owner", "new-owner", "repo-name", "repo-name");
        fixture.repo.remote("origin", old_url)?;

        let output = fixture.run_pull(old_url, true)?;

        assert!(
            output.contains(&format!(
                "Would change 'origin' remote from '{}' to '{}'",
                old_url, expected_new_url
            )),
            "Expected remote URL update message, got: {}",
            output
        );

        fixture.assert_remote_url(old_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "repo-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_remote_url_update() -> anyhow::Result<()> {
        let fixture = PullTestFixture::new("repo-name")?;
        let old_url = "git@github.com:old-owner/repo-name.git";
        let expected_new_url = "git@github.com:new-owner/repo-name.git";

        fixture.mock_github_repo("old-owner", "new-owner", "repo-name", "repo-name");
        fixture.repo.remote("origin", old_url)?;

        let output = fixture.run_pull(old_url, false)?;

        assert!(
            output.contains(&format!(
                "Changing 'origin' remote from '{}' to '{}'",
                old_url, expected_new_url
            )),
            "Expected remote URL update message, got: {}",
            output
        );

        fixture.assert_remote_url(expected_new_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "repo-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_directory_rename_dry_run() -> anyhow::Result<()> {
        let fixture = PullTestFixture::new("old-name")?;
        let remote_url = "https://github.com/owner/new-name.git";
        let parent_dir = fixture.repo_dir.parent().unwrap().canonicalize()?;

        fixture.mock_github_repo("owner", "owner", "new-name", "new-name");
        fixture.repo.remote("origin", remote_url)?;

        let output = fixture.run_pull(remote_url, true)?;

        assert!(
            output.contains(&format!(
                "Would rename directory from '{}' to '{}'",
                parent_dir.join("old-name").display(),
                parent_dir.join("new-name").display()
            )),
            "Expected directory rename message, got: {}",
            output
        );

        fixture.assert_remote_url(remote_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "old-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_directory_rename() -> anyhow::Result<()> {
        let fixture = PullTestFixture::new("old-name")?;
        let remote_url = "https://github.com/owner/new-name.git";
        let parent_dir = fixture.repo_dir.parent().unwrap().canonicalize()?;

        fixture.mock_github_repo("owner", "owner", "new-name", "new-name");
        fixture.repo.remote("origin", remote_url)?;

        let output = fixture.run_pull(remote_url, false)?;

        assert!(
            output.contains(&format!(
                "Renaming directory from '{}' to '{}'",
                parent_dir.join("old-name").display(),
                parent_dir.join("new-name").display()
            )),
            "Expected directory rename message, got: {}",
            output
        );

        fixture.assert_remote_url(remote_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "old-name", false)?;
        test_helpers::assert_directory_existence(&fixture.temp, "new-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_both_updates_dry_run() -> anyhow::Result<()> {
        let fixture = PullTestFixture::new("old-name")?;
        let old_url = "git@github.com:old-owner/old-name.git";
        let expected_new_url = "git@github.com:new-owner/new-name.git";
        let parent_dir = fixture.repo_dir.parent().unwrap().canonicalize()?;

        fixture.mock_github_repo("old-owner", "new-owner", "old-name", "new-name");
        fixture.repo.remote("origin", old_url)?;

        let output = fixture.run_pull(old_url, true)?;

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

        fixture.assert_remote_url(old_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "old-name", true)?;
        test_helpers::assert_directory_existence(&fixture.temp, "new-name", false)?;

        Ok(())
    }

    #[test]
    fn test_pull_both_updates() -> anyhow::Result<()> {
        let fixture = PullTestFixture::new("old-name")?;
        let old_url = "git@github.com:old-owner/old-name.git";
        let expected_new_url = "git@github.com:new-owner/new-name.git";
        let parent_dir = fixture.repo_dir.parent().unwrap().canonicalize()?;

        fixture.mock_github_repo("old-owner", "new-owner", "old-name", "new-name");
        fixture.repo.remote("origin", old_url)?;

        let output = fixture.run_pull(old_url, false)?;

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

        fixture.assert_remote_url(expected_new_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "old-name", false)?;
        test_helpers::assert_directory_existence(&fixture.temp, "new-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_invalid_github_url() -> anyhow::Result<()> {
        let fixture = PullTestFixture::new("test-repo")?;
        let invalid_url = "https://not-github.com/owner/repo.git";

        let result = fixture.run_pull(invalid_url, false);

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
    fn test_pull_different_url_formats() -> anyhow::Result<()> {
        let test_cases = vec![
            "https://github.com/owner/test-repo.git",
            "git@github.com:owner/test-repo.git",
            "ssh://git@github.com/owner/test-repo.git",
            "git://github.com/owner/test-repo.git",
        ];

        for url in test_cases {
            let fixture = PullTestFixture::new("test-repo")?;
            fixture.mock_github_repo("owner", "owner", "test-repo", "test-repo");
            fixture.repo.remote("origin", url)?;

            let result = fixture.run_pull(url, false);
            assert!(result.is_ok(), "Failed with URL format: {}", url);
            fixture.repo.remote_delete("origin")?;
        }

        Ok(())
    }

    #[test]
    fn test_pull_nonexistent_github_repo() -> anyhow::Result<()> {
        let fixture = PullTestFixture::new("test-repo")?;
        let remote_url = "git@github.com:owner/test-repo.git";

        fixture.mock_github_error("owner", "test-repo", 404);
        fixture.repo.remote("origin", remote_url)?;

        let result = fixture.run_pull(remote_url, false);

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
