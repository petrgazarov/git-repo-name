pub mod config;
mod fs;
pub mod git;
pub mod remotes {
    pub mod file;
    pub mod github;
}
#[cfg(test)]
mod test_helpers;
use crate::remotes::{file, github};
use config::CONFIG;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error: not a git repository")]
    NotAGitRepo,

    #[error("Error: no remote named '{0}' configured")]
    NoRemote(String),

    #[error("Invalid GitHub URL format: {0}")]
    InvalidGitHubUrl(String),

    #[error("GitHub API error: {0}")]
    GitHubApi(String),

    #[error("Error: {0}")]
    Config(String),

    #[error("Filesystem error: {0}")]
    Fs(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Error: {0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Represents the source of truth for the repository name
#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum Source {
    Remote,
    Local,
}

impl TryFrom<&str> for Source {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "remote" => Ok(Source::Remote),
            "local" => Ok(Source::Local),
            _ => Err(Error::Config(format!(
                "Invalid source value: '{}'. Valid values are 'remote' or 'local'",
                s
            ))),
        }
    }
}

pub fn sync(source: Source, dry_run: bool) -> Result<()> {
    let repo = git::get_current_repo()?;
    let remote = CONFIG.get_remote()?;
    let remote_url = git::get_remote_url(&repo, &remote)?;

    match source {
        Source::Remote => {
            if github::is_github_url(&remote_url) {
                github::sync_from_github_remote(&repo, &remote_url, dry_run)
            } else {
                file::sync_from_file_remote(&repo, &remote_url, dry_run)
            }
        }
        Source::Local => {
            println!("TODO: Implement local source sync");
            Ok(())
        }
    }
}

pub fn fetch_repo_name() -> Result<String> {
    let repo = git::get_current_repo()?;
    let remote = CONFIG.get_remote()?;
    let remote_url = git::get_remote_url(&repo, &remote)?;
    let result;

    if github::is_github_url(&remote_url) {
        let (owner, repo_name) = github::parse_github_url(&remote_url)?;
        let repo_info = github::get_repo_info(&owner, &repo_name)?;
        result = format!("{} ({})", repo_info.name, repo_info.clone_url);
    } else {
        let canonical_path = fs::resolve_canonical_path(Path::new(&remote_url))?;
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

            std::fs::remove_dir_all(&main_repo_dir)?;
        }

        std::env::set_current_dir(&original_dir)?;

        Ok(())
    }

    #[test]
    fn test_fetch_repo_name_github() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        test_helpers::setup_test_config(temp.path())?;

        // Set up mock server
        let mut server = mockito::Server::new();
        let _ = server
            .mock("GET", "/repos/owner/test-repo")
            .match_header("authorization", "token mock-token")
            .with_status(200)
            .with_body(
                r#"{
                "name": "upstream-repo",
                "full_name": "owner/upstream-repo",
                "clone_url": "https://github.com/owner/upstream-repo.git"
            }"#,
            )
            .create();

        std::env::set_var("GITHUB_API_BASE_URL", &server.url());

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

            std::fs::remove_dir_all(&main_repo_dir)?;
        }

        // Clean up
        std::env::set_current_dir(&original_dir)?;
        std::env::remove_var("GITHUB_API_BASE_URL");

        Ok(())
    }
}
