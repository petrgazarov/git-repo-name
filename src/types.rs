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
