use crate::{
    git,
    remotes::github::{
        client::get_repo_info, client::update_repo_name, url::format_new_remote_url,
        url::parse_github_url,
    },
    types::{Error, Result},
    utils::fs,
};
use git2::Repository;

pub fn pull_from_github_remote(repo: &Repository, remote_url: &str, dry_run: bool) -> Result<()> {
    let (owner, remote_repo_name) = parse_github_url(remote_url)?;

    let local_directory_name = git::get_local_directory_name(repo)?;
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

pub fn push_to_github_remote(repo: &Repository, remote_url: &str, dry_run: bool) -> Result<()> {
    let local_directory_name = git::get_local_directory_name(repo)?;
    let (owner, remote_repo_name) = parse_github_url(remote_url)?;

    if remote_repo_name == local_directory_name {
        println!("Repository name already matches the local directory name");
        return Ok(());
    }

    if dry_run {
        println!(
            "Would update GitHub repository name from '{}' to '{}'",
            remote_repo_name, local_directory_name
        );
        let would_change_url = format_new_remote_url(remote_url, &owner, &local_directory_name);
        println!(
            "Would change 'origin' remote from '{}' to '{}'",
            remote_url, would_change_url
        );
        return Ok(());
    }

    let updated_repo = match update_repo_name(&owner, &remote_repo_name, &local_directory_name) {
        Ok(repo_info) => repo_info,
        Err(e) => {
            return Err(e);
        }
    };

    let resolved_owner = updated_repo.full_name.split('/').next().unwrap_or(&owner);

    let new_remote_url = format_new_remote_url(remote_url, resolved_owner, &updated_repo.name);
    git::set_remote_url(repo, remote_url, &new_remote_url, false)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;

    struct PullTestSetup {
        temp: assert_fs::TempDir,
        repo_dir: std::path::PathBuf,
        repo: git2::Repository,
        _guard: test_helpers::CurrentDirGuard,
    }

    fn setup_for_pull_test(local_repo_name: &str) -> anyhow::Result<PullTestSetup> {
        let guard = test_helpers::CurrentDirGuard::new();
        let temp = assert_fs::TempDir::new()?;
        test_helpers::setup_test_config(temp.path())?;

        let (repo_dir, repo) = test_helpers::create_main_repo(&temp, local_repo_name)?;
        std::env::set_current_dir(&repo_dir)?;

        Ok(PullTestSetup {
            temp,
            repo_dir,
            repo,
            _guard: guard,
        })
    }

    #[test]
    fn test_pull_up_to_date_dry_run() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("test-repo")?;
        let remote_url = "https://github.com/owner/test-repo.git";
        test_helpers::mock_github_get_repo("owner", "owner", "test-repo", "test-repo");
        pull_test_setup.repo.remote("origin", remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_github_remote(&pull_test_setup.repo, remote_url, true)
        })?;

        assert!(
            output.contains("Directory name and remote URL already up-to-date"),
            "Expected up-to-date message, got: {}",
            output
        );

        assert_eq!(remote_url, git::get_remote_url(&pull_test_setup.repo)?);
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "test-repo", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_up_to_date() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("test-repo")?;
        let remote_url = "https://github.com/owner/test-repo.git";
        test_helpers::mock_github_get_repo("owner", "owner", "test-repo", "test-repo");
        pull_test_setup.repo.remote("origin", remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_github_remote(&pull_test_setup.repo, remote_url, false)
        })?;

        assert!(
            output.contains("Directory name and remote URL already up-to-date"),
            "Expected up-to-date message, got: {}",
            output
        );

        assert_eq!(remote_url, git::get_remote_url(&pull_test_setup.repo)?);
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "test-repo", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_remote_url_update_dry_run() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("repo-name")?;
        let old_url = "git@github.com:old-owner/repo-name.git";
        let expected_new_url = "git@github.com:new-owner/repo-name.git";
        test_helpers::mock_github_get_repo("old-owner", "new-owner", "repo-name", "repo-name");
        pull_test_setup.repo.remote("origin", old_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_github_remote(&pull_test_setup.repo, old_url, true)
        })?;

        assert!(
            output.contains(&format!(
                "Would change 'origin' remote from '{}' to '{}'",
                old_url, expected_new_url
            )),
            "Expected remote URL update message, got: {}",
            output
        );

        assert_eq!(old_url, git::get_remote_url(&pull_test_setup.repo)?);
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "repo-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_remote_url_update() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("repo-name")?;
        let old_url = "git@github.com:old-owner/repo-name.git";
        let expected_new_url = "git@github.com:new-owner/repo-name.git";
        test_helpers::mock_github_get_repo("old-owner", "new-owner", "repo-name", "repo-name");
        pull_test_setup.repo.remote("origin", old_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_github_remote(&pull_test_setup.repo, old_url, false)
        })?;

        assert!(
            output.contains(&format!(
                "Changing 'origin' remote from '{}' to '{}'",
                old_url, expected_new_url
            )),
            "Expected remote URL update message, got: {}",
            output
        );

        assert_eq!(
            expected_new_url,
            git::get_remote_url(&pull_test_setup.repo)?
        );
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "repo-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_directory_rename_dry_run() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("old-name")?;
        let remote_url = "https://github.com/owner/new-name.git";
        let parent_dir = pull_test_setup.repo_dir.parent().unwrap().canonicalize()?;
        test_helpers::mock_github_get_repo("owner", "owner", "new-name", "new-name");
        pull_test_setup.repo.remote("origin", remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_github_remote(&pull_test_setup.repo, remote_url, true)
        })?;

        assert!(
            output.contains(&format!(
                "Would rename directory from '{}' to '{}'",
                parent_dir.join("old-name").display(),
                parent_dir.join("new-name").display()
            )),
            "Expected directory rename message, got: {}",
            output
        );

        assert_eq!(remote_url, git::get_remote_url(&pull_test_setup.repo)?);
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "old-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_directory_rename() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("old-name")?;
        let remote_url = "https://github.com/owner/new-name.git";
        let parent_dir = pull_test_setup.repo_dir.parent().unwrap().canonicalize()?;

        test_helpers::mock_github_get_repo("owner", "owner", "new-name", "new-name");
        pull_test_setup.repo.remote("origin", remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_github_remote(&pull_test_setup.repo, remote_url, false)
        })?;

        assert!(
            output.contains(&format!(
                "Renaming directory from '{}' to '{}'",
                parent_dir.join("old-name").display(),
                parent_dir.join("new-name").display()
            )),
            "Expected directory rename message, got: {}",
            output
        );

        assert_eq!(remote_url, git::get_remote_url(&pull_test_setup.repo)?);
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "old-name", false)?;
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "new-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_both_updates_dry_run() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("old-name")?;
        let old_url = "git@github.com:old-owner/old-name.git";
        let expected_new_url = "git@github.com:new-owner/new-name.git";
        let parent_dir = pull_test_setup.repo_dir.parent().unwrap().canonicalize()?;

        test_helpers::mock_github_get_repo("old-owner", "new-owner", "old-name", "new-name");
        pull_test_setup.repo.remote("origin", old_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_github_remote(&pull_test_setup.repo, old_url, true)
        })?;

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

        assert_eq!(old_url, git::get_remote_url(&pull_test_setup.repo)?);
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "old-name", true)?;
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "new-name", false)?;

        Ok(())
    }

    #[test]
    fn test_pull_both_updates() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("old-name")?;
        let old_url = "git@github.com:old-owner/old-name.git";
        let expected_new_url = "git@github.com:new-owner/new-name.git";
        let parent_dir = pull_test_setup.repo_dir.parent().unwrap().canonicalize()?;

        test_helpers::mock_github_get_repo("old-owner", "new-owner", "old-name", "new-name");
        pull_test_setup.repo.remote("origin", old_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_github_remote(&pull_test_setup.repo, old_url, false)
        })?;

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

        assert_eq!(
            expected_new_url,
            git::get_remote_url(&pull_test_setup.repo)?
        );
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "old-name", false)?;
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "new-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_invalid_github_url() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("test-repo")?;
        let invalid_url = "https://not-github.com/owner/repo.git";

        let result = pull_from_github_remote(&pull_test_setup.repo, invalid_url, false);

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
            let pull_test_setup = setup_for_pull_test("test-repo")?;
            test_helpers::mock_github_get_repo("owner", "owner", "test-repo", "test-repo");
            pull_test_setup.repo.remote("origin", url)?;

            let result = pull_from_github_remote(&pull_test_setup.repo, url, false);
            assert!(result.is_ok(), "Failed with URL format: {}", url);
            pull_test_setup.repo.remote_delete("origin")?;
        }

        Ok(())
    }

    #[test]
    fn test_pull_nonexistent_github_repo() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("test-repo")?;
        let remote_url = "git@github.com:owner/test-repo.git";

        test_helpers::mock_github_get_repo_error("owner", "test-repo");
        pull_test_setup.repo.remote("origin", remote_url)?;

        let result = pull_from_github_remote(&pull_test_setup.repo, remote_url, false);

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

    struct PushTestSetup {
        _temp: assert_fs::TempDir,
        repo: git2::Repository,
        _guard: test_helpers::CurrentDirGuard,
    }

    fn setup_for_push_test(local_repo_name: &str) -> anyhow::Result<PushTestSetup> {
        let guard = test_helpers::CurrentDirGuard::new();
        let temp = assert_fs::TempDir::new()?;
        test_helpers::setup_test_config(temp.path())?;

        let (repo_dir, repo) = test_helpers::create_main_repo(&temp, local_repo_name)?;
        std::env::set_current_dir(&repo_dir)?;

        Ok(PushTestSetup {
            _temp: temp,
            repo,
            _guard: guard,
        })
    }

    #[test]
    fn test_push_already_matches() -> anyhow::Result<()> {
        let push_test_setup = setup_for_push_test("test-repo")?;
        let remote_url = "https://github.com/owner/test-repo.git";
        push_test_setup.repo.remote("origin", remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            push_to_github_remote(&push_test_setup.repo, remote_url, false)
        })?;

        assert!(
            output.contains("Repository name already matches the local directory name"),
            "Expected message about matching repo name, got: {}",
            output
        );
        assert_eq!(remote_url, git::get_remote_url(&push_test_setup.repo)?);

        Ok(())
    }

    #[test]
    fn test_push_dry_run() -> anyhow::Result<()> {
        let push_test_setup = setup_for_push_test("new-name")?;
        let remote_url = "https://github.com/owner/old-name.git";

        push_test_setup.repo.remote("origin", remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            push_to_github_remote(&push_test_setup.repo, remote_url, true)
        })?;

        assert!(
            output.contains("Would update GitHub repository name from 'old-name' to 'new-name'"),
            "Expected dry run message about updating repo name, got: {}",
            output
        );
        assert_eq!(remote_url, git::get_remote_url(&push_test_setup.repo)?);

        Ok(())
    }

    #[test]
    fn test_push_update_repo_name() -> anyhow::Result<()> {
        let push_test_setup = setup_for_push_test("new-name")?;
        let old_url = "git@github.com:owner/old-name.git";
        let expected_new_url = "git@github.com:owner/new-name.git";

        push_test_setup.repo.remote("origin", old_url)?;
        test_helpers::mock_github_update_repo("owner", "owner", "old-name", "new-name");

        let (output, _) = test_helpers::capture_stdout(|| {
            push_to_github_remote(&push_test_setup.repo, old_url, false)
        })?;

        assert!(
            output.contains(&format!(
                "Changing 'origin' remote from '{}' to '{}'",
                old_url, expected_new_url
            )),
            "Expected changing remote message, got: {}",
            output
        );
        assert_eq!(
            expected_new_url,
            git::get_remote_url(&push_test_setup.repo)?
        );

        Ok(())
    }

    #[test]
    fn test_push_error_updating_repo_name() -> anyhow::Result<()> {
        let push_test_setup = setup_for_push_test("new-name")?;
        let remote_url = "https://github.com/owner/old-name.git";

        push_test_setup.repo.remote("origin", remote_url)?;
        test_helpers::mock_github_get_repo("owner", "owner", "old-name", "old-name");
        test_helpers::mock_github_update_repo_error("owner", "old-name", 403);

        let result = push_to_github_remote(&push_test_setup.repo, remote_url, false);

        match result {
            Err(e) => {
                // Check for the exact error message we're receiving
                let expected_error = "GitHub API error: Permission denied. Ensure your GitHub token has the 'Administration' repository permission (write).";
                assert_eq!(
                    e.to_string(),
                    expected_error,
                    "Expected specific error message"
                );
                Ok(())
            }
            Ok(_) => panic!("Expected error, but operation succeeded"),
        }
    }

    #[test]
    fn test_push_owner_change() -> anyhow::Result<()> {
        let push_test_setup = setup_for_push_test("new-name")?;
        let old_url = "git@github.com:old-owner/old-name.git";
        let expected_new_url = "git@github.com:new-owner/new-name.git";

        push_test_setup.repo.remote("origin", old_url)?;
        test_helpers::mock_github_update_repo("old-owner", "new-owner", "old-name", "new-name");

        let (output, _) = test_helpers::capture_stdout(|| {
            push_to_github_remote(&push_test_setup.repo, old_url, false)
        })?;

        assert!(
            output.contains(&format!(
                "Changing 'origin' remote from '{}' to '{}'",
                old_url, expected_new_url
            )),
            "Expected success message, got: {}",
            output
        );
        assert_eq!(
            expected_new_url,
            git::get_remote_url(&push_test_setup.repo)?
        );

        Ok(())
    }
}
