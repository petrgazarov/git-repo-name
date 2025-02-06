use crate::{Error, Result};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Renames a directory to a new name, keeping it in the same parent directory.
/// Returns an error if the directory cannot be renamed or if the paths are invalid.
pub fn rename_directory(from_path: &Path, new_name: &str) -> Result<()> {
    let current_name = from_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::Fs("Invalid directory name".into()))?;

    if current_name == new_name {
        println!("Directory names already match: {}", new_name);
        return Ok(());
    }

    let parent_path = from_path
        .parent()
        .ok_or_else(|| Error::Fs("Cannot get parent directory".into()))?;
    let new_path = parent_path.join(new_name);

    if new_path.exists() {
        return Err(Error::Fs(format!(
            "Target path '{}' already exists",
            new_path.display()
        )));
    }

    println!(
        "Renaming directory from '{}' to '{}'",
        current_name, new_name
    );
    std::fs::rename(from_path, new_path)
        .map_err(|e| Error::Fs(format!("Failed to rename directory: {}", e)))?;

    Ok(())
}

/// Sets secure file permissions (600 on Unix systems).
/// On non-Unix systems, this is a no-op.
pub fn set_secure_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| Error::Fs(format!("Failed to set file permissions: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;

    #[test]
    fn test_rename_directory() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;

        // Create a test directory
        let old_dir = temp.child("old_name");
        old_dir.create_dir_all()?;

        // Test renaming
        rename_directory(old_dir.path(), "new_name")?;

        // Verify old directory doesn't exist
        assert!(!old_dir.exists());

        // Verify new directory exists
        let new_dir = temp.child("new_name");
        assert!(new_dir.exists());

        Ok(())
    }

    #[test]
    fn test_rename_directory_same_name() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let dir = temp.child("same_name");
        dir.create_dir_all()?;

        rename_directory(dir.path(), "same_name")?;
        assert!(dir.exists());

        Ok(())
    }

    #[test]
    fn test_rename_directory_errors() {
        let temp = assert_fs::TempDir::new().unwrap();

        // Test invalid source path
        let non_existent = temp.child("non_existent");
        assert!(matches!(
            rename_directory(non_existent.path(), "new_name"),
            Err(Error::Fs(_))
        ));

        // Test renaming to existing directory
        let existing = temp.child("existing");
        existing.create_dir_all().unwrap();
        let source = temp.child("source");
        source.create_dir_all().unwrap();

        assert!(matches!(
            rename_directory(source.path(), "existing"),
            Err(Error::Fs(_))
        ));
    }

    #[test]
    #[cfg(unix)]
    fn test_set_secure_permissions() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let test_file = temp.child("test_file");
        test_file.write_str("test content")?;

        set_secure_permissions(test_file.path())?;

        let metadata = test_file.metadata()?;
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);

        Ok(())
    }
}
