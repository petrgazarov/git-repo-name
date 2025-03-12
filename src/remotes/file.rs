use crate::{
    git,
    types::{Error, Result},
    utils::fs,
};
use git2::Repository;
use path_clean::PathClean;
use std::path::Path;

pub fn sync_from_file_remote(repo: &Repository, remote_url: &str, dry_run: bool) -> Result<()> {
    let local_directory_name = repo
        .workdir()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?
        .file_name()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?
        .to_str()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?
        .to_string();
    let canonical_path = fs::resolve_canonical_path(Path::new(&remote_url))?;
    let resolved_repo_name = git::extract_repo_name_from_path(&canonical_path)?;

    let repo_path = repo
        .workdir()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?;

    let resolved_remote_url = format_new_remote_url(remote_url, &canonical_path)?;
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

/// Formats a new path from a canonical path, keeping the format of the original remote URL.
pub fn format_new_remote_url(original_remote_url: &str, canonical_path: &str) -> Result<String> {
    // If the original URL is relative and it is equivalent to the given canonical_path (without canonicalization),
    // then just return the original URL.
    let original_path = Path::new(original_remote_url);
    if original_path.is_relative() {
        let joined = std::env::current_dir()?.join(original_path);
        let normalized = joined.clean();
        let normalized_str = normalized
            .to_str()
            .ok_or_else(|| Error::Fs("Failed to convert path to string".into()))?;
        let expanded_full = format!("file://{}", normalized_str);
        if expanded_full == canonical_path {
            return Ok(original_remote_url.to_string());
        }
    }

    // Otherwise, format based on whether the original URL has a file:// prefix.
    if original_remote_url.trim_start().starts_with("file://") {
        Ok(canonical_path.to_string())
    } else {
        Ok(canonical_path.trim_start_matches("file://").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_new_remote_url() -> anyhow::Result<()> {
        // Calculate canonical path for relative path test
        let current_dir = std::env::current_dir()?;
        let norm = current_dir.join("repo.git").clean();
        let norm_str = norm
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Conversion error"))?;
        let canonical_expected = format!("file://{}", norm_str);

        let test_cases = vec![
            // (original_remote_url, canonical_path, expected_result)
            (
                "file:///old/path/repo.git",
                "file:///new/path/repo.git",
                "file:///new/path/repo.git",
            ),
            (
                "/old/path/repo.git",
                "file:///new/path/repo.git",
                "/new/path/repo.git",
            ),
            // When canonical path matches the expanded original path
            ("repo.git", &canonical_expected, "repo.git"),
            // When canonical path is different from the expanded original path
            (
                "repo.git",
                "file:///different/path/repo.git",
                "/different/path/repo.git",
            ),
        ];

        for (original, canonical, expected) in test_cases {
            let result = format_new_remote_url(original, canonical)?;
            assert_eq!(result, expected);
        }

        Ok(())
    }
}

#[cfg(test)]
mod sync_from_file_remote_tests {
    use super::*;
    use crate::test_helpers;

    /// Test fixture to reduce repetition in tests
    struct SyncTestFixture {
        temp: assert_fs::TempDir,
        bare_repo_path: std::path::PathBuf,
        repo: git2::Repository,
        canonical_remote_url: String,
        _guard: test_helpers::CurrentDirGuard,
    }

    impl SyncTestFixture {
        // Create a new test fixture with the given repo names
        fn new(bare_repo_name: &str, local_repo_name: &str) -> anyhow::Result<Self> {
            let guard = test_helpers::CurrentDirGuard::new();
            let temp = assert_fs::TempDir::new()?;
            test_helpers::setup_test_config(temp.path())?;

            let bare_repo_path = test_helpers::create_bare_repo(&temp, bare_repo_name)?;
            let (repo_dir, repo) = test_helpers::create_main_repo(&temp, local_repo_name)?;

            std::env::set_current_dir(&repo_dir)?;

            let canonical_remote_url = test_helpers::get_canonical_remote_url(&bare_repo_path)?;

            Ok(Self {
                temp,
                bare_repo_path,
                repo,
                canonical_remote_url,
                _guard: guard,
            })
        }

        // Run the sync operation and return the output
        fn run_sync(&self, remote_url: &str, dry_run: bool) -> anyhow::Result<String> {
            let (output, _) = test_helpers::capture_stdout(|| {
                sync_from_file_remote(&self.repo, remote_url, dry_run)
            })?;

            Ok(output)
        }

        // Helper to check remote URL
        fn assert_remote_url(&self, expected_url: &str) -> anyhow::Result<()> {
            let remote_url = git::get_remote_url(&self.repo)?;
            assert_eq!(remote_url, expected_url);
            Ok(())
        }
    }

    #[test]
    fn test_sync_up_to_date_dry_run() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("same-repo.git", "same-repo")?;
        let remote_url = fixture.canonical_remote_url.clone();
        fixture.repo.remote("origin", &remote_url)?;

        let output = fixture.run_sync(&remote_url, true)?;

        assert!(
            output.contains("Directory name and remote URL already up-to-date"),
            "Expected up-to-date message, got: {}",
            output
        );

        fixture.assert_remote_url(&remote_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "same-repo", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_up_to_date() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("same-repo.git", "same-repo")?;
        let remote_url = fixture.canonical_remote_url.clone();
        fixture.repo.remote("origin", &remote_url)?;

        fixture.run_sync(&remote_url, false)?;

        fixture.assert_remote_url(&remote_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "same-repo", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_remote_url_update_dry_run() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("test-repo.git", "test-repo")?;
        let relative_remote_url = "file://../test-repo.git";
        fixture.repo.remote("origin", relative_remote_url)?;

        let output = fixture.run_sync(relative_remote_url, true)?;

        assert!(
            output.contains(&format!(
                "Would change 'origin' remote from '{}' to '{}'",
                relative_remote_url, fixture.canonical_remote_url
            )),
            "Expected remote URL update message, got: {}",
            output
        );

        fixture.assert_remote_url(relative_remote_url)?;

        Ok(())
    }

    #[test]
    fn test_sync_remote_url_update() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("test-repo.git", "test-repo")?;
        let relative_remote_url = "file://../test-repo.git";
        fixture.repo.remote("origin", relative_remote_url)?;

        fixture.run_sync(relative_remote_url, false)?;

        fixture.assert_remote_url(&fixture.canonical_remote_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "test-repo", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_directory_rename_dry_run() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("new-name.git", "old-name")?;
        let remote_url = fixture.canonical_remote_url.clone();
        fixture.repo.remote("origin", &remote_url)?;

        let output = fixture.run_sync(&remote_url, true)?;
        let parent_dir = fixture.bare_repo_path.parent().unwrap().canonicalize()?;

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

        test_helpers::assert_directory_existence(&fixture.temp, "old-name", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_directory_rename() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("new-name.git", "old-name")?;
        let remote_url = fixture.canonical_remote_url.clone();
        fixture.repo.remote("origin", &remote_url)?;

        fixture.run_sync(&remote_url, false)?;

        test_helpers::assert_directory_existence(&fixture.temp, "old-name", false)?;
        test_helpers::assert_directory_existence(&fixture.temp, "new-name", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_both_updates_dry_run() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("new-name.git", "old-name")?;
        let relative_remote_url = "file://../new-name.git";
        fixture.repo.remote("origin", relative_remote_url)?;

        let output = fixture.run_sync(relative_remote_url, true)?;
        let parent_dir = fixture.bare_repo_path.parent().unwrap().canonicalize()?;

        assert!(
            output.contains(&format!(
                "Would change 'origin' remote from '{}' to '{}'",
                relative_remote_url, fixture.canonical_remote_url
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

        fixture.assert_remote_url(relative_remote_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "old-name", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_both_updates() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("new-name.git", "old-name")?;
        let relative_remote_url = "file://../new-name.git";
        fixture.repo.remote("origin", relative_remote_url)?;

        fixture.run_sync(relative_remote_url, false)?;

        fixture.assert_remote_url(&fixture.canonical_remote_url)?;
        test_helpers::assert_directory_existence(&fixture.temp, "old-name", false)?;
        test_helpers::assert_directory_existence(&fixture.temp, "new-name", true)?;

        Ok(())
    }

    #[test]
    fn test_sync_invalid_remote_path() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        test_helpers::setup_test_config(temp.path())?;
        let (_repo_dir, repo) = test_helpers::create_main_repo(&temp, "test-repo")?;
        let result = sync_from_file_remote(&repo, "/nonexistent/path", false);

        // Assert the specific error type and message
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
    fn test_sync_relative_and_absolute() -> anyhow::Result<()> {
        let fixture = SyncTestFixture::new("abs-repo.git", "abs-repo")?;
        let canonical_url = fixture.canonical_remote_url.clone();

        // Test with relative path
        fixture.repo.remote("origin", "../abs-repo.git")?;
        let result_rel = sync_from_file_remote(&fixture.repo, "../abs-repo.git", false);
        assert!(result_rel.is_ok());

        // Test with absolute path
        fixture.repo.remote_delete("origin")?;
        fixture.repo.remote("origin", &canonical_url)?;
        let result_abs = sync_from_file_remote(&fixture.repo, &canonical_url, false);
        assert!(result_abs.is_ok());

        Ok(())
    }
}
