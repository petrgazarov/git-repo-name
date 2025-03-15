use crate::{
    git,
    remotes::file,
    types::{Error, Result},
    utils::fs,
};
use git2::Repository;
use std::path::Path;

pub fn pull_from_file_remote(repo: &Repository, remote_url: &str, dry_run: bool) -> Result<()> {
    let local_directory_name = git::get_local_directory_name(repo)?;
    let canonical_path = fs::resolve_canonical_path(Path::new(&remote_url))?;
    let resolved_repo_name = git::extract_repo_name_from_path(&canonical_path)?;

    let repo_path = repo
        .workdir()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?;

    let resolved_remote_url = file::url::format_new_remote_url(remote_url, &canonical_path)?;
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

pub fn push_to_file_remote(repo: &Repository, remote_url: &str, dry_run: bool) -> Result<()> {
    let local_directory_name = git::get_local_directory_name(repo)?;

    let remote_path = remote_url.trim_start_matches("file://");
    if !Path::new(remote_path).exists() {
        return Err(Error::Fs(format!(
            "Remote repository does not exist: {}",
            remote_url
        )));
    }

    let canonical_path = fs::resolve_canonical_path(Path::new(remote_url))?;
    let remote_repo_name = git::extract_repo_name_from_path(&canonical_path)?;

    if remote_repo_name == local_directory_name {
        println!("Remote repository name already matches the local directory name");
        return Ok(());
    }

    let fs_path = Path::new(
        canonical_path
            .strip_prefix("file://")
            .unwrap_or(&canonical_path),
    );

    let parent_dir = fs_path.parent().unwrap();
    let old_repo_path = parent_dir.join(format!("{}.git", remote_repo_name));
    let new_repo_path = parent_dir.join(format!("{}.git", local_directory_name));

    let new_canonical_path = format!("file://{}", new_repo_path.display());
    let new_remote_url = file::url::format_new_remote_url(remote_url, &new_canonical_path)?;

    fs::rename_directory(
        &old_repo_path,
        &format!("{}.git", local_directory_name),
        dry_run,
    )?;
    if dry_run {
        println!(
            "Would change 'origin' remote from '{}' to '{}'",
            remote_url, new_remote_url
        );
        return Ok(());
    }
    git::set_remote_url(repo, remote_url, &new_remote_url, dry_run)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;

    struct PullTestSetup {
        temp: assert_fs::TempDir,
        bare_repo_path: std::path::PathBuf,
        repo: git2::Repository,
        canonical_remote_url: String,
        _guard: test_helpers::CurrentDirGuard,
    }

    fn setup_for_pull_test(
        bare_repo_name: &str,
        local_repo_name: &str,
    ) -> anyhow::Result<PullTestSetup> {
        let guard = test_helpers::CurrentDirGuard::new();
        let temp = assert_fs::TempDir::new()?;
        test_helpers::setup_test_config(temp.path())?;

        let bare_repo_path = test_helpers::create_bare_repo(&temp, bare_repo_name)?;
        let (repo_dir, repo) = test_helpers::create_main_repo(&temp, local_repo_name)?;

        std::env::set_current_dir(&repo_dir)?;

        let canonical_remote_url = test_helpers::get_canonical_remote_url(&bare_repo_path)?;

        Ok(PullTestSetup {
            temp,
            bare_repo_path,
            repo,
            canonical_remote_url,
            _guard: guard,
        })
    }

    #[test]
    fn test_pull_up_to_date_dry_run() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("same-repo.git", "same-repo")?;
        let remote_url = pull_test_setup.canonical_remote_url.clone();
        pull_test_setup.repo.remote("origin", &remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_file_remote(&pull_test_setup.repo, &remote_url, true)
        })?;

        assert!(
            output.contains("Directory name and remote URL already up-to-date"),
            "Expected up-to-date message, got: {}",
            output
        );

        assert_eq!(remote_url, git::get_remote_url(&pull_test_setup.repo)?);
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "same-repo", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_up_to_date() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("same-repo.git", "same-repo")?;
        let remote_url = pull_test_setup.canonical_remote_url.clone();
        pull_test_setup.repo.remote("origin", &remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_file_remote(&pull_test_setup.repo, &remote_url, false)
        })?;

        assert!(
            output.contains("Directory name and remote URL already up-to-date"),
            "Expected up-to-date message, got: {}",
            output
        );

        assert_eq!(remote_url, git::get_remote_url(&pull_test_setup.repo)?);
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "same-repo", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_remote_url_update_dry_run() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("test-repo.git", "test-repo")?;
        let relative_remote_url = "file://../test-repo.git";
        pull_test_setup.repo.remote("origin", relative_remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_file_remote(&pull_test_setup.repo, &relative_remote_url, true)
        })?;

        assert!(
            output.contains(&format!(
                "Would change 'origin' remote from '{}' to '{}'",
                relative_remote_url, pull_test_setup.canonical_remote_url
            )),
            "Expected remote URL update message, got: {}",
            output
        );
        assert_eq!(
            relative_remote_url,
            git::get_remote_url(&pull_test_setup.repo)?
        );

        Ok(())
    }

    #[test]
    fn test_pull_remote_url_update() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("test-repo.git", "test-repo")?;
        let relative_remote_url = "file://../test-repo.git";
        pull_test_setup.repo.remote("origin", relative_remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_file_remote(&pull_test_setup.repo, &relative_remote_url, false)
        })?;

        assert!(
            output.contains(&format!(
                "Changing 'origin' remote from '{}' to '{}'",
                relative_remote_url, pull_test_setup.canonical_remote_url
            )),
            "Expected remote URL update message, got: {}",
            output
        );

        Ok(())
    }

    #[test]
    fn test_pull_directory_rename_dry_run() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("new-name.git", "old-name")?;
        let remote_url = pull_test_setup.canonical_remote_url.clone();
        pull_test_setup.repo.remote("origin", &remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_file_remote(&pull_test_setup.repo, &remote_url, true)
        })?;
        let parent_dir = pull_test_setup
            .bare_repo_path
            .parent()
            .unwrap()
            .canonicalize()?;

        assert!(
            output.contains(&format!(
                "Would rename directory from '{}' to '{}'",
                parent_dir.join("old-name").display(),
                parent_dir.join("new-name").display()
            )),
            "Expected directory rename message, got: {}",
            output
        );
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "old-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_directory_rename() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("new-name.git", "old-name")?;
        let remote_url = pull_test_setup.canonical_remote_url.clone();
        pull_test_setup.repo.remote("origin", &remote_url)?;
        let parent_dir = pull_test_setup
            .bare_repo_path
            .parent()
            .unwrap()
            .canonicalize()?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_file_remote(&pull_test_setup.repo, &remote_url, false)
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

        test_helpers::assert_directory_existence(&pull_test_setup.temp, "old-name", false)?;
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "new-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_both_updates_dry_run() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("new-name.git", "old-name")?;
        let relative_remote_url = "file://../new-name.git";
        pull_test_setup
            .repo
            .remote("origin", &relative_remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_file_remote(&pull_test_setup.repo, &relative_remote_url, true)
        })?;
        let parent_dir = pull_test_setup
            .bare_repo_path
            .parent()
            .unwrap()
            .canonicalize()?;

        assert!(
            output.contains(&format!(
                "Would change 'origin' remote from '{}' to '{}'",
                relative_remote_url, pull_test_setup.canonical_remote_url
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

        assert_eq!(
            relative_remote_url,
            git::get_remote_url(&pull_test_setup.repo)?
        );
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "old-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_both_updates() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("new-name.git", "old-name")?;
        let relative_remote_url = "file://../new-name.git";
        pull_test_setup.repo.remote("origin", relative_remote_url)?;
        let parent_dir = pull_test_setup
            .bare_repo_path
            .parent()
            .unwrap()
            .canonicalize()?;

        let (output, _) = test_helpers::capture_stdout(|| {
            pull_from_file_remote(&pull_test_setup.repo, &relative_remote_url, false)
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

        assert_eq!(
            pull_test_setup.canonical_remote_url,
            git::get_remote_url(&pull_test_setup.repo)?
        );
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "old-name", false)?;
        test_helpers::assert_directory_existence(&pull_test_setup.temp, "new-name", true)?;

        Ok(())
    }

    #[test]
    fn test_pull_invalid_remote_path() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        test_helpers::setup_test_config(temp.path())?;
        let (_repo_dir, repo) = test_helpers::create_main_repo(&temp, "test-repo")?;
        let result = pull_from_file_remote(&repo, "/nonexistent/path", false);

        match result {
            Err(Error::Fs(msg)) => {
                #[cfg(unix)]
                assert!(
                    msg.starts_with("Failed to resolve path: No such file or directory"),
                    "Expected error about nonexistent path, got: {}",
                    msg
                );
            }
            Err(e) => panic!(
                "Expected Error::Fs with message about nonexistent path, got: {:?}",
                e
            ),
            Ok(_) => panic!("Expected error, but operation succeeded"),
        }

        Ok(())
    }

    #[test]
    fn test_pull_relative_and_absolute() -> anyhow::Result<()> {
        let pull_test_setup = setup_for_pull_test("abs-repo.git", "abs-repo")?;
        let canonical_url = pull_test_setup.canonical_remote_url.clone();

        pull_test_setup.repo.remote("origin", "../abs-repo.git")?;
        let result_rel = pull_from_file_remote(&pull_test_setup.repo, "../abs-repo.git", false);
        assert!(result_rel.is_ok());

        pull_test_setup.repo.remote_delete("origin")?;
        pull_test_setup.repo.remote("origin", &canonical_url)?;
        let result_abs = pull_from_file_remote(&pull_test_setup.repo, &canonical_url, false);
        assert!(result_abs.is_ok());

        Ok(())
    }

    struct PushTestSetup {
        temp: assert_fs::TempDir,
        bare_repo_path: std::path::PathBuf,
        repo: git2::Repository,
        canonical_remote_url: String,
        _guard: test_helpers::CurrentDirGuard,
    }

    fn setup_for_push_test(
        bare_repo_name: &str,
        local_repo_name: &str,
    ) -> anyhow::Result<PushTestSetup> {
        let guard = test_helpers::CurrentDirGuard::new();
        let temp = assert_fs::TempDir::new()?;
        test_helpers::setup_test_config(temp.path())?;

        let bare_repo_path = test_helpers::create_bare_repo(&temp, bare_repo_name)?;
        let (repo_dir, repo) = test_helpers::create_main_repo(&temp, local_repo_name)?;

        std::env::set_current_dir(&repo_dir)?;

        let canonical_remote_url = test_helpers::get_canonical_remote_url(&bare_repo_path)?;

        Ok(PushTestSetup {
            temp,
            bare_repo_path,
            repo,
            canonical_remote_url,
            _guard: guard,
        })
    }

    #[test]
    fn test_push_already_matches() -> anyhow::Result<()> {
        let push_test_setup = setup_for_push_test("test-repo.git", "test-repo")?;
        let remote_url = push_test_setup.canonical_remote_url.clone();
        push_test_setup.repo.remote("origin", &remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            push_to_file_remote(&push_test_setup.repo, &remote_url, false)
        })?;

        assert!(
            output.contains("Remote repository name already matches the local directory name"),
            "Expected up-to-date message, got: {}",
            output
        );

        assert_eq!(remote_url, git::get_remote_url(&push_test_setup.repo)?);
        assert!(
            push_test_setup.bare_repo_path.exists(),
            "Repository should still exist"
        );

        Ok(())
    }

    #[test]
    fn test_push_rename_dry_run() -> anyhow::Result<()> {
        let push_test_setup = setup_for_push_test("old-name.git", "new-name")?;
        let remote_url = push_test_setup.canonical_remote_url.clone();
        push_test_setup.repo.remote("origin", &remote_url)?;

        let (output, _) = test_helpers::capture_stdout(|| {
            push_to_file_remote(&push_test_setup.repo, &remote_url, true)
        })?;
        let parent_dir = push_test_setup
            .bare_repo_path
            .parent()
            .unwrap()
            .canonicalize()?;

        assert!(
            output.contains(&format!(
                "Would rename directory from '{}' to '{}'",
                parent_dir.join("old-name.git").display(),
                parent_dir.join("new-name.git").display()
            )),
            "Expected directory rename message, got: {}",
            output
        );
        assert!(
            output.contains(&format!(
                "Would change 'origin' remote from '{}' to",
                remote_url
            )),
            "Expected remote URL update message, got: {}",
            output
        );

        assert_eq!(remote_url, git::get_remote_url(&push_test_setup.repo)?);
        assert!(
            push_test_setup.bare_repo_path.exists(),
            "Original repository should still exist"
        );
        assert!(
            !parent_dir.join("new-name.git").exists(),
            "New repository should not exist yet"
        );

        Ok(())
    }

    #[test]
    fn test_push_rename() -> anyhow::Result<()> {
        let push_test_setup = setup_for_push_test("old-name.git", "new-name")?;
        let remote_url = push_test_setup.canonical_remote_url.clone();
        push_test_setup.repo.remote("origin", &remote_url)?;
        let parent_dir = push_test_setup
            .bare_repo_path
            .parent()
            .unwrap()
            .canonicalize()?;
        let new_repo_path = parent_dir.join("new-name.git");

        let (output, _) = test_helpers::capture_stdout(|| {
            push_to_file_remote(&push_test_setup.repo, &remote_url, false)
        })?;

        assert!(
            output.contains(&format!(
                "Renaming directory from '{}' to '{}'",
                parent_dir.join("old-name.git").display(),
                new_repo_path.display()
            )),
            "Expected directory rename message, got: {}",
            output
        );

        assert!(
            !push_test_setup.bare_repo_path.exists(),
            "Original repository should be gone"
        );
        assert!(new_repo_path.exists(), "New repository should exist");

        let expected_new_url = remote_url.replace("old-name.git", "new-name.git");
        assert_eq!(
            expected_new_url,
            git::get_remote_url(&push_test_setup.repo)?
        );

        Ok(())
    }

    #[test]
    fn test_push_nonexistent_remote() -> anyhow::Result<()> {
        let push_test_setup = setup_for_push_test("existing-repo.git", "local-repo")?;
        let nonexistent_path = push_test_setup.temp.path().join("nonexistent-repo.git");
        let nonexistent_url = format!("file://{}", nonexistent_path.display());
        push_test_setup.repo.remote("origin", &nonexistent_url)?;

        let result = push_to_file_remote(&push_test_setup.repo, &nonexistent_url, false);

        match result {
            Err(e) => {
                assert!(
                    e.to_string().contains("Remote repository does not exist"),
                    "Expected error about nonexistent repository, got: {}",
                    e
                );
                Ok(())
            }
            Ok(_) => panic!("Expected error, but operation succeeded"),
        }
    }
}
