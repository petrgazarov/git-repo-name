use crate::{Error, Result};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Renames a directory to a new name, keeping it in the same parent directory.
pub fn rename_directory(current_path: &Path, new_name: &str, dry_run: bool) -> Result<()> {
    let parent_path = current_path
        .parent()
        .ok_or_else(|| Error::Fs("Cannot get parent directory".into()))?;
    let new_path = parent_path.join(new_name);

    if dry_run {
        println!(
            "Would rename directory '{}' to '{}'",
            current_path.display(),
            new_path.display()
        );
        return Ok(());
    }

    println!(
        "Renaming directory '{}' to '{}'...",
        current_path.display(),
        new_path.display()
    );

    if new_path.exists() {
        return Err(Error::Fs(format!(
            "Target path '{}' already exists",
            new_path.display()
        )));
    }

    std::fs::rename(current_path, new_path)
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
        let old_dir = temp.child("old_name");
        old_dir.create_dir_all()?;

        rename_directory(old_dir.path(), "new_name", false)?;

        assert!(!old_dir.exists());
        let new_dir = temp.child("new_name");
        assert!(new_dir.exists());

        Ok(())
    }

    #[test]
    fn test_rename_directory_errors() {
        let temp = assert_fs::TempDir::new().unwrap();

        // Test invalid source path
        let non_existent = temp.child("non_existent");
        assert!(matches!(
            rename_directory(non_existent.path(), "new_name", false),
            Err(Error::Fs(_))
        ));

        // Test renaming to existing directory
        let existing = temp.child("existing");
        existing.create_dir_all().unwrap();
        let source = temp.child("source");
        source.create_dir_all().unwrap();

        assert!(matches!(
            rename_directory(source.path(), "existing", false),
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
