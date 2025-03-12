pub mod config;
pub mod git;
pub mod types;
pub mod utils {
    pub mod fs;
}
pub mod remotes {
    pub mod file {
        pub mod operations;
        pub mod url;
    }
    pub mod github {
        pub mod client;
        pub mod operations;
        pub mod url;
    }
}
#[cfg(test)]
pub(crate) mod test_helpers;
use crate::{
    remotes::{file, github},
    types::Result,
};
use std::path::Path;

pub fn pull(dry_run: bool) -> Result<()> {
    let repo = git::get_current_repo()?;
    let remote_url = git::get_remote_url(&repo)?;

    if github::url::is_github_url(&remote_url) {
        github::operations::pull_from_github_remote(&repo, &remote_url, dry_run)
    } else {
        file::operations::pull_from_file_remote(&repo, &remote_url, dry_run)
    }
}

pub fn push(dry_run: bool) -> Result<()> {
    let repo = git::get_current_repo()?;
    let remote_url = git::get_remote_url(&repo)?;

    if github::url::is_github_url(&remote_url) {
        github::operations::push_to_github_remote(&repo, &remote_url, dry_run)
    } else {
        file::operations::push_to_file_remote(&repo, &remote_url, dry_run)
    }
}

pub fn fetch_repo_name() -> Result<String> {
    let repo = git::get_current_repo()?;
    let remote_url = git::get_remote_url(&repo)?;
    let result;

    if github::url::is_github_url(&remote_url) {
        let (owner, repo_name) = github::url::parse_github_url(&remote_url)?;
        let repo_info = github::client::get_repo_info(&owner, &repo_name)?;
        result = format!("{} ({})", repo_info.name, repo_info.clone_url);
    } else {
        let canonical_path = utils::fs::resolve_canonical_path(Path::new(&remote_url))?;
        let name = git::extract_repo_name_from_path(&canonical_path)?;
        result = format!("{} ({})", name, canonical_path);
    }
    println!("{}", result);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;
    use git2::Repository;

    #[test]
    fn test_fetch_repo_name_filesystem() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        test_helpers::setup_test_config(temp.path())?;

        let test_urls = [
            ("../upstream_repo.git"),
            // Non-canonicalized paths. On MacOS, the temp directory is simlinked,
            // so this is a good test for the canonicalization logic.
            (&format!("{}", temp.path().join("upstream_repo.git").display())),
            (&format!("file://{}", temp.path().join("upstream_repo.git").display())),
        ];

        test_helpers::create_bare_repo(&temp, "upstream_repo.git")?;

        let original_dir = std::env::current_dir()?;
        for url in test_urls {
            let (main_repo_dir, repo) = test_helpers::create_main_repo(&temp, "main-repo")?;

            // Needed for relative path test case to work
            std::env::set_current_dir(&main_repo_dir)?;

            repo.remote("origin", url)?;

            // Canonicalized file URL
            let expected_url = format!(
                "file://{}",
                temp.path()
                    .join("upstream_repo.git")
                    .canonicalize()?
                    .display()
            );
            let name = fetch_repo_name()?;
            assert_eq!(name, format!("upstream_repo ({})", expected_url));

            std::env::set_current_dir(&original_dir)?;
            std::fs::remove_dir_all(&main_repo_dir)?;
        }

        std::env::set_current_dir(&original_dir)?;

        Ok(())
    }

    #[test]
    fn test_fetch_repo_name_github() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        test_helpers::setup_test_config(temp.path())?;
        test_helpers::mock_github_repo("owner", "owner", "test-repo", "upstream-repo");

        let test_urls = [
            "https://github.com/owner/test-repo.git",
            "git@github.com:owner/test-repo.git",
            "git://github.com/owner/test-repo.git",
        ];

        let original_dir = std::env::current_dir()?;
        for (i, url) in test_urls.iter().enumerate() {
            let main_repo_dir = temp.path().join(format!("main-repo-{}", i));
            std::fs::create_dir(&main_repo_dir)?;
            let repo = Repository::init(&main_repo_dir)?;
            std::env::set_current_dir(&main_repo_dir)?;

            repo.remote("origin", url)?;
            let name = fetch_repo_name()?;
            assert_eq!(
                name,
                "upstream-repo (https://github.com/owner/upstream-repo.git)"
            );

            std::env::set_current_dir(&original_dir)?;
            std::fs::remove_dir_all(&main_repo_dir)?;
        }

        // Clean up
        std::env::set_current_dir(&original_dir)?;
        std::env::remove_var("GITHUB_API_BASE_URL");

        Ok(())
    }
}
