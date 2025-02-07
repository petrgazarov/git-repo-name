use crate::{Error, Result};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Resolves a path to its canonical form, following symlinks.
/// If the path is a file:// URL, extracts and resolves the path portion.
pub fn resolve_canonical_path(path: &Path) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();
    let path_to_resolve = if path_str.starts_with("file://") {
        Path::new(&path_str[7..])
    } else {
        path
    };

    let canonical = path_to_resolve
        .canonicalize()
        .map_err(|e| Error::Fs(format!("Failed to resolve path: {}", e)))?;

    Ok(PathBuf::from(format!("file://{}", canonical.display())))
}

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
    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[test]
    fn test_resolve_canonical_path() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let real_dir = temp.child("real_dir");
        real_dir.create_dir_all()?;

        // Test regular path
        let resolved = resolve_canonical_path(real_dir.path())?;
        let expected = format!("file://{}", real_dir.path().canonicalize()?.display());
        assert_eq!(resolved.to_string_lossy(), expected);

        // Test file:// URL
        let file_url = format!("file://{}", real_dir.path().display());
        let resolved_url = resolve_canonical_path(Path::new(&file_url))?;
        assert_eq!(resolved_url.to_string_lossy(), expected);

        #[cfg(unix)]
        {
            let symlink_path = temp.child("link_dir");
            symlink(real_dir.path(), symlink_path.path())?;

            let resolved = resolve_canonical_path(symlink_path.path())?;
            assert_eq!(resolved.to_string_lossy(), expected);
        }

        Ok(())
    }

    #[test]
    fn test_rename_directory() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let old_dir = temp.child("old_name");
        old_dir.create_dir_all()?;

        rename_directory(old_dir.path(), "new_name")?;

        assert!(!old_dir.exists());
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
