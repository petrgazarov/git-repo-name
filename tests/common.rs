use assert_fs::TempDir;
use ini::Ini;
use std::path::{Path, PathBuf};

/// Sets up a test config directory with a mock GitHub token.
/// Returns the path to the config directory.
pub fn setup_test_config(temp_dir: &Path) -> anyhow::Result<()> {
    let config_dir = temp_dir.join(".config/git-repo-name");
    std::fs::create_dir_all(&config_dir)?;
    std::env::set_var("XDG_CONFIG_HOME", temp_dir.join(".config"));
    let config_file = config_dir.join("config");

    let mut conf = Ini::new();
    conf.with_section(Some("github")).set("token", "mock-token");
    conf.with_section(None::<String>)
        .set("default_remote", "origin");
    conf.write_to_file(&config_file)?;

    Ok(())
}

/// Creates a bare repository in the given directory.
/// Returns the path to the repository.
pub fn create_bare_repo(temp: &TempDir, name: &str) -> anyhow::Result<PathBuf> {
    let repo_dir = temp.path().join(name);
    std::fs::create_dir_all(&repo_dir)?;
    git2::Repository::init_bare(&repo_dir)?;
    Ok(repo_dir)
}

/// Creates a main repository in the given directory.
/// Returns the path to the repository.
pub fn create_main_repo(temp: &TempDir, dir: &str) -> anyhow::Result<(PathBuf, git2::Repository)> {
    let repo_dir = temp.path().join(dir);
    std::fs::create_dir(&repo_dir)?;
    let repo = git2::Repository::init(&repo_dir)?;
    Ok((repo_dir, repo))
}
