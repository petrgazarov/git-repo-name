pub mod config;
mod fs;
pub mod git;
pub mod github;

use config::CONFIG;

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
#[derive(Debug, Clone, Copy, PartialEq)]
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
                github::sync_github_repo(&repo, &remote_url, dry_run)
            } else {
                sync_generic_repo(&repo, &remote_url, dry_run)
            }
        }
        Source::Local => {
            println!("Using local directory name as source");
            Ok(())
        }
    }
}

pub fn sync_generic_repo(repo: &git2::Repository, remote_url: &str, dry_run: bool) -> Result<()> {
    let new_name = git::extract_repo_name(remote_url)?;

    if dry_run {
        println!("Would rename repository to: {}", new_name);
        return Ok(());
    }

    let repo_path = repo
        .workdir()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?;
    fs::rename_directory(repo_path, &new_name)
}

pub fn fetch_repo_name() -> Result<String> {
    let repo = git::get_current_repo()?;
    let remote = CONFIG.get_remote()?;
    let remote_url = git::get_remote_url(&repo, &remote)?;

    if github::is_github_url(&remote_url) {
        let (owner, repo_name) = github::parse_github_url(&remote_url)?;
        let repo_info = github::get_repo_info(&owner, &repo_name)?;
        Ok(repo_info.name)
    } else {
        git::extract_repo_name(&remote_url)
    }
}
