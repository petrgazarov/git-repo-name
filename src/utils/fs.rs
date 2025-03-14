use crate::types::{Error, Result};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Renames a directory to a new name, keeping it in the same parent directory.
pub fn rename_directory(current_path: &Path, new_name: &str, dry_run: bool) -> Result<()> {
    let parent_path = current_path
        .parent()
        .ok_or_else(|| Error::Fs("Cannot get parent directory".into()))?;
    let new_path = parent_path.join(new_name);

    // Convert paths to strings and remove any trailing slashes for display
    let current_display = current_path
        .to_string_lossy()
        .trim_end_matches('/')
        .to_string();
    let new_display = new_path.to_string_lossy().trim_end_matches('/').to_string();

    if dry_run {
        println!(
            "Would rename directory from '{}' to '{}'",
            current_display, new_display
        );
        return Ok(());
    }

    println!(
        "Renaming directory from '{}' to '{}'...",
        current_display, new_display
    );

    if new_path.exists() {
        return Err(Error::Fs(format!(
            "Target path '{}' already exists",
            new_display
        )));
    }

    std::fs::rename(current_path, &new_path)
        .map_err(|e| Error::Fs(format!("Failed to rename directory: {}", e)))?;

    Ok(())
}

/// Sets secure file permissions (600 on Unix systems)
pub fn set_secure_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| Error::Fs(format!("Failed to set file permissions: {}", e)))?;

    Ok(())
}

/// Resolves a file path to its canonical form, following symlinks.
pub fn resolve_canonical_path(path: &Path) -> Result<String> {
    let path_str = path.to_string_lossy();
    let path_to_resolve = if path_str.starts_with("file://") {
        Path::new(&path_str[7..])
    } else {
        path
    };

    let canonical = path_to_resolve
        .canonicalize()
        .map_err(|e| Error::Fs(format!("Failed to resolve path: {}", e)))?;

    Ok(format!("file://{}", canonical.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::path::Path;

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

        let non_existent = temp.child("non_existent");
        assert!(matches!(
            rename_directory(non_existent.path(), "new_name", false),
            Err(Error::Fs(_))
        ));

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
    fn test_set_secure_permissions_on_unix() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let test_file = temp.child("test_file");
        test_file.write_str("test content")?;

        set_secure_permissions(test_file.path())?;

        let metadata = test_file.metadata()?;
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);

        Ok(())
    }

    #[test]
    fn test_resolve_canonical_path() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let real_dir = temp.child("real_dir");
        real_dir.create_dir_all()?;

        let resolved = resolve_canonical_path(real_dir.path())?;
        let expected = format!("file://{}", real_dir.path().canonicalize()?.display());
        assert_eq!(resolved, expected);

        let file_url = format!("file://{}", real_dir.path().display());
        let resolved_url = resolve_canonical_path(Path::new(&file_url))?;
        assert_eq!(resolved_url, expected);

        #[cfg(unix)]
        {
            let symlink_path = temp.child("link_dir");
            symlink(real_dir.path(), symlink_path.path())?;

            let resolved = resolve_canonical_path(symlink_path.path())?;
            assert_eq!(resolved, expected);
        }

        Ok(())
    }
}
