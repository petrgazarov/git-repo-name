use crate::{Error, Result};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[cfg(windows)]
extern "system" {
    fn LocalFree(hMem: isize) -> isize;
}

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

    std::fs::rename(current_path, new_path)
        .map_err(|e| Error::Fs(format!("Failed to rename directory: {}", e)))?;

    Ok(())
}

/// Sets secure file permissions (600 on Unix systems).
/// On Windows, creates an ACL that only allows the current user to access the file.
pub fn set_secure_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| Error::Fs(format!("Failed to set file permissions: {}", e)))?;

    #[cfg(windows)]
    {
        use std::ptr;
        use windows::core::PWSTR;
        use windows::Win32::Foundation::{GetLastError, LocalFree, WIN32_ERROR};
        use windows::Win32::Security::Authorization::{
            SetEntriesInAclW, SetNamedSecurityInfoW, ACCESS_MODE, EXPLICIT_ACCESS_W,
            OBJECT_SECURITY_INFORMATION, SE_FILE_OBJECT, TRUSTEE_FORM, TRUSTEE_TYPE, TRUSTEE_W,
        };
        use windows::Win32::Security::{
            ACL, DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
        };
        use windows::Win32::Storage::FileSystem::{FILE_GENERIC_READ, FILE_GENERIC_WRITE};
        use windows::Win32::System::UserProfile::GetUserNameW;

        unsafe {
            // Get current user name
            let mut name_buffer = [0u16; 256];
            let mut size = name_buffer.len() as u32;
            if !GetUserNameW(PWSTR(name_buffer.as_mut_ptr()), &mut size) {
                return Err(format!(
                    "Failed to get current username: error code {}",
                    GetLastError().0
                )
                .into());
            }

            // Create an EXPLICIT_ACCESS entry for the current user with read/write rights
            let mut ea = EXPLICIT_ACCESS_W::default();
            ea.grfAccessPermissions = FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0;
            ea.grfAccessMode = ACCESS_MODE::SET_ACCESS;
            ea.grfInheritance = 0; // No inheritance
            ea.Trustee = TRUSTEE_W {
                pMultipleTrustee: ptr::null_mut(),
                MultipleTrusteeOperation: 0, // NO_MULTIPLE_TRUSTEE
                TrusteeForm: TRUSTEE_FORM::TRUSTEE_IS_NAME,
                TrusteeType: TRUSTEE_TYPE::TRUSTEE_IS_USER,
                ptstrName: PWSTR(name_buffer.as_mut_ptr()),
            };

            // Create a new ACL containing this single ACE
            let mut new_acl_ptr: *mut ACL = ptr::null_mut();
            let result = SetEntriesInAclW(Some(&[ea]), None, &mut new_acl_ptr);
            if result != WIN32_ERROR(0) {
                return Err(format!("Failed to create ACL: error code {:?}", result).into());
            }

            // Convert path to wide string for Windows API
            let path_str = path.to_string_lossy().to_string();
            let mut path_wide: Vec<u16> = path_str.encode_utf16().collect();
            path_wide.push(0); // Null terminate

            // Apply the ACL to the file, replacing existing ACL and disabling inheritance
            let security_info = DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION;
            let result = SetNamedSecurityInfoW(
                PWSTR(path_wide.as_mut_ptr()),
                SE_FILE_OBJECT,
                OBJECT_SECURITY_INFORMATION(security_info),
                ptr::null(), // psidOwner
                ptr::null(), // psidGroup
                new_acl_ptr, // pDacl
                ptr::null(), // pSacl
            );

            LocalFree(new_acl_ptr as isize);
            if result != WIN32_ERROR(0) {
                return Err(
                    format!("Failed to set file permissions: error code {:?}", result).into(),
                );
            }
        }
    }

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
    #[cfg(windows)]
    fn test_set_secure_permissions_windows() -> anyhow::Result<()> {
        use std::ptr;
        use windows::core::PWSTR;
        use windows::Win32::Foundation::{LocalFree, WIN32_ERROR};
        use windows::Win32::Security::Authorization::{
            GetNamedSecurityInfoW, OBJECT_SECURITY_INFORMATION, SE_FILE_OBJECT,
        };
        use windows::Win32::Security::{ACL, DACL_SECURITY_INFORMATION};

        let temp = assert_fs::TempDir::new()?;
        let test_file = temp.child("secure.txt");
        test_file.write_str("Secret data")?;

        super::set_secure_permissions(test_file.path())?;

        unsafe {
            let path_str = test_file.path().to_string_lossy().to_string();
            let mut path_wide: Vec<u16> = path_str.encode_utf16().collect();
            path_wide.push(0); // Null terminate

            let mut dacl_ptr: *mut ACL = ptr::null_mut();
            let mut security_descriptor: *mut _ = ptr::null_mut();

            let result = GetNamedSecurityInfoW(
                PWSTR(path_wide.as_mut_ptr()),
                SE_FILE_OBJECT,
                OBJECT_SECURITY_INFORMATION(DACL_SECURITY_INFORMATION),
                ptr::null_mut(),          // ppsidOwner
                ptr::null_mut(),          // ppsidGroup
                &mut dacl_ptr,            // ppDacl
                ptr::null_mut(),          // ppSacl
                &mut security_descriptor, // ppSecurityDescriptor
            );

            assert_eq!(
                result,
                WIN32_ERROR(0),
                "Failed to get security info with error code {:?}",
                result
            );
            assert!(!dacl_ptr.is_null(), "DACL should not be null");

            LocalFree(security_descriptor as isize);
        }

        Ok(())
    }

    #[test]
    fn test_resolve_canonical_path() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let real_dir = temp.child("real_dir");
        real_dir.create_dir_all()?;

        // Test regular path
        let resolved = resolve_canonical_path(real_dir.path())?;
        let expected = format!("file://{}", real_dir.path().canonicalize()?.display());
        assert_eq!(resolved, expected);

        // Test file:// URL
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
