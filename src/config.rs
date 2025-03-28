use crate::{
    types::{Error, Result},
    utils,
};
use ini::Ini;
use once_cell::sync::Lazy;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::new().expect("Failed to initialize config"));

pub struct Config {
    config_dir: PathBuf,
    config_values: RwLock<ConfigValues>,
}

/// Internal configuration values that are loaded from the config file.
#[derive(Clone)]
struct ConfigValues {
    github_token: Option<String>,
    // Current remote, None means use default_remote
    remote: Option<String>,
    default_remote: String,
}

impl Config {
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_config_dir()?;

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| Error::Config(format!("Failed to create config directory: {}", e)))?;
        }

        let config = Self {
            config_dir,
            config_values: RwLock::new(ConfigValues {
                github_token: None,
                default_remote: "origin".to_string(),
                remote: None,
            }),
        };

        // Check if config file exists and load it if it does
        let config_file = config.get_config_file_path();
        if config_file.exists() {
            let ini = Ini::load_from_file(&config_file)
                .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;
            config.load_from_ini(&ini)?;
        } else {
            // Create initial config file
            config.write_to_disk()?;
        }

        Ok(config)
    }

    fn get_config_dir() -> Result<PathBuf> {
        let base_dir = if cfg!(unix) {
            env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
                .ok_or_else(|| Error::Config("Could not determine config directory".into()))?
        } else {
            dirs::config_dir()
                .ok_or_else(|| Error::Config("Could not determine config directory".into()))?
        };

        Ok(base_dir.join("git-repo-name"))
    }

    fn load_from_ini(&self, ini: &Ini) -> Result<()> {
        let mut values = self.config_values.write().unwrap();
        values.github_token = ini
            .get_from(Some("github"), "token")
            .map(String::from)
            .filter(|s| !s.is_empty());
        values.default_remote = ini
            .get_from(None::<String>, "default_remote")
            .unwrap_or("origin")
            .to_string();
        Ok(())
    }

    fn write_to_disk(&self) -> Result<()> {
        let values = self.config_values.read().unwrap();
        let mut ini = Ini::new();

        if let Some(token) = &values.github_token {
            ini.with_section(Some("github"))
                .set("token".to_string(), token.clone());
        }

        // Write default remote
        ini.with_section(None::<String>)
            .set("default_remote".to_string(), values.default_remote.clone());

        let config_file = self.get_config_file_path();
        if let Some(parent) = config_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        ini.write_to_file(&config_file)
            .map_err(|e| Error::Config(format!("Failed to write config file: {}", e)))?;

        utils::fs::set_secure_permissions(&config_file)?;

        Ok(())
    }

    fn get_config_file_path(&self) -> PathBuf {
        self.config_dir.join("config")
    }

    pub fn get_github_token(&self) -> Result<String> {
        let values = self.config_values.read().unwrap();
        values
            .github_token
            .clone()
            .ok_or_else(|| Error::Config("No GitHub token found in configuration".into()))
    }

    pub fn set_github_token(&self, token: &str) -> Result<()> {
        let mut values = self.config_values.write().unwrap();
        values.github_token = Some(token.to_string());
        drop(values);
        self.write_to_disk()
    }

    pub fn get_remote(&self) -> Result<String> {
        let values = self.config_values.read().unwrap();
        Ok(values
            .remote
            .as_ref()
            .unwrap_or(&values.default_remote)
            .clone())
    }

    pub fn set_remote(&self, remote: String) {
        let mut values = self.config_values.write().unwrap();
        values.remote = Some(remote);
    }

    pub fn get_default_remote(&self) -> Result<String> {
        let values = self.config_values.read().unwrap();
        Ok(values.default_remote.clone())
    }

    pub fn set_default_remote(&self, remote: &str) -> Result<()> {
        let mut values = self.config_values.write().unwrap();
        values.default_remote = remote.to_string();
        drop(values);
        self.write_to_disk()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use predicates::prelude::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn test_config_github_token() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let config = Config {
            config_dir: temp.path().to_path_buf(),
            config_values: RwLock::new(ConfigValues {
                github_token: None,
                default_remote: "origin".to_string(),
                remote: None,
            }),
        };
        config.write_to_disk()?;

        config.set_github_token("test-token")?;

        let config_dir = temp.path();
        assert!(config_dir.exists());
        assert!(config_dir.is_dir());

        let config_file = temp.child("config");
        config_file.assert(predicate::path::exists());
        config_file.assert(predicate::path::is_file());

        // Verify file permissions (600) on Unix systems
        #[cfg(unix)]
        {
            let metadata = config_file.metadata()?;
            assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        }

        let new_config = Config {
            config_dir: temp.path().to_path_buf(),
            config_values: RwLock::new(ConfigValues {
                github_token: None,
                default_remote: "origin".to_string(),
                remote: None,
            }),
        };
        let ini = Ini::load_from_file(&config_file)?;
        new_config.load_from_ini(&ini)?;

        assert_eq!(new_config.get_github_token()?, "test-token");

        Ok(())
    }

    #[test]
    fn test_remote() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let config = Config {
            config_dir: temp.path().to_path_buf(),
            config_values: RwLock::new(ConfigValues {
                github_token: None,
                default_remote: "origin".to_string(),
                remote: None,
            }),
        };
        config.write_to_disk()?;

        assert_eq!(config.get_remote()?, "origin");

        config.set_default_remote("upstream")?;
        assert_eq!(config.get_remote()?, "upstream");

        let new_config = Config {
            config_dir: temp.path().to_path_buf(),
            config_values: RwLock::new(ConfigValues {
                github_token: None,
                default_remote: "origin".to_string(),
                remote: None,
            }),
        };
        let ini = Ini::load_from_file(&temp.child("config"))?;
        new_config.load_from_ini(&ini)?;
        assert_eq!(new_config.get_remote()?, "upstream");

        new_config.set_remote("temporary".to_string());
        assert_eq!(new_config.get_remote()?, "temporary");

        Ok(())
    }

    #[test]
    fn test_malformed_config_file() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        let config_file = temp.child("git-repo-name").child("config");
        if let Some(parent) = config_file.path().parent() {
            std::fs::create_dir_all(parent)?;
        }
        config_file.write_str("not a valid ini file")?;

        let config_result = Config::new();
        assert!(
            config_result.is_err(),
            "Expected error due to malformed config file"
        );

        // Clean up the environment variable
        env::remove_var("XDG_CONFIG_HOME");

        Ok(())
    }

    #[test]
    fn test_config_file_initial_creation() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        let config_file = temp.child("git-repo-name").child("config");
        config_file.assert(predicate::path::missing());

        let config = Config::new()?;
        config_file.assert(predicate::path::exists());

        assert_eq!(config.get_remote()?, "origin");
        env::remove_var("XDG_CONFIG_HOME");
        Ok(())
    }

    #[test]
    fn test_config_creates_parent_directories() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let config_dir = temp
            .path()
            .join("var")
            .join("lib")
            .join("nonexistent")
            .join("git-repo-name");
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        let config = Config {
            config_dir: config_dir,
            config_values: RwLock::new(ConfigValues {
                github_token: None,
                default_remote: "origin".to_string(),
                remote: None,
            }),
        };

        config.write_to_disk()?;

        let config_file = temp
            .path()
            .join("var")
            .join("lib")
            .join("nonexistent")
            .join("git-repo-name")
            .join("config");
        assert!(config_file.exists());
        assert!(config_file.is_file());
        assert!(config_file.parent().unwrap().exists());
        assert!(config_file.parent().unwrap().is_dir());

        Ok(())
    }
}
