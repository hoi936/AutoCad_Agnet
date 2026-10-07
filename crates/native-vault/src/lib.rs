//! Small native boundary: DPAPI CurrentUser only. No plaintext disk fallback.
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;
pub struct TokenVault {
    root: PathBuf,
}
impl TokenVault {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn path(&self, reference: &str) -> Result<PathBuf, String> {
        if reference.len() > 128
            || reference.is_empty()
            || !reference
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err("Vault reference invalid".into());
        }
        Ok(self.root.join(reference))
    }
    pub fn put(&self, reference: &str, plaintext: &[u8]) -> Result<(), String> {
        let destination = self.path(reference)?;
        if plaintext.is_empty() || plaintext.len() > 128 * 1024 {
            return Err("Token bundle size invalid".into());
        }
        let ciphertext = protect(plaintext, false)?;
        std::fs::create_dir_all(&self.root).map_err(|_| "Vault directory unavailable")?;
        restrict(&self.root)?;
        let mut file = tempfile::NamedTempFile::new_in(&self.root)
            .map_err(|_| "Vault temporary file failed")?;
        restrict(file.path())?;
        file.write_all(&ciphertext)
            .map_err(|_| "Vault write failed")?;
        file.as_file().sync_all().map_err(|_| "Vault sync failed")?;
        file.persist(destination)
            .map_err(|_| "Vault atomic replacement failed")?;
        Ok(())
    }
    pub fn get(&self, reference: &str) -> Result<Zeroizing<Vec<u8>>, String> {
        let path = self.path(reference)?;
        if std::fs::metadata(&path)
            .map_err(|_| "Vault secret unavailable")?
            .len()
            > 256 * 1024
        {
            return Err("Vault file oversized".into());
        }
        let ciphertext = std::fs::read(path).map_err(|_| "Vault read failed")?;
        protect(&ciphertext, true)
    }
    pub fn delete(&self, reference: &str) -> Result<(), String> {
        match std::fs::remove_file(self.path(reference)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("Vault delete failed".into()),
        }
    }
}
#[cfg(not(windows))]
fn protect(_: &[u8], _: bool) -> Result<Zeroizing<Vec<u8>>, String> {
    Err("DPAPI requires Windows".into())
}
#[cfg(not(windows))]
fn restrict(_: &Path) -> Result<(), String> {
    Err("ACL requires Windows".into())
}
#[cfg(windows)]
fn protect(input: &[u8], decrypt: bool) -> Result<Zeroizing<Vec<u8>>, String> {
    use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::*};
    let blob = CRYPT_INTEGER_BLOB {
        cbData: input.len().try_into().map_err(|_| "Vault input size")?,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // SAFETY: input stays alive for the synchronous DPAPI call. Output is OS allocated,
    // copied using its returned length, zeroed on decrypt, then released exactly once.
    unsafe {
        let ok = if decrypt {
            CryptUnprotectData(
                &blob,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptProtectData(
                &blob,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err("DPAPI protection failed".into());
        }
        if output.pbData.is_null() || output.cbData == 0 || output.cbData > 256 * 1024 {
            if !output.pbData.is_null() {
                LocalFree(output.pbData as _);
            }
            return Err("DPAPI output invalid".into());
        }
        let bytes = std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize);
        let value = Zeroizing::new(bytes.to_vec());
        if decrypt {
            use zeroize::Zeroize;
            bytes.zeroize();
        }
        LocalFree(output.pbData as _);
        Ok(value)
    }
}
#[cfg(windows)]
fn restrict(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{Authorization::*, *},
    };
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let sddl: Vec<u16> = "D:P(A;OICI;FA;;;OW)(A;OICI;FA;;;SY)"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut descriptor = std::ptr::null_mut();
    // SAFETY: terminated UTF-16 buffers; descriptor belongs to LocalAlloc; ACL is
    // borrowed until SetNamedSecurityInfoW returns, then descriptor is freed.
    unsafe {
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err("Vault ACL construction failed".into());
        }
        let mut acl = std::ptr::null_mut();
        let mut present = 0;
        let mut defaulted = 0;
        let ok = GetSecurityDescriptorDacl(descriptor, &mut present, &mut acl, &mut defaulted);
        let status = if ok != 0 && present != 0 {
            SetNamedSecurityInfoW(
                path.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                acl,
                std::ptr::null_mut(),
            )
        } else {
            1
        };
        LocalFree(descriptor);
        if status != 0 {
            return Err("Vault ACL restriction failed".into());
        }
    }
    Ok(())
}
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn large_unicode_rotation_restart_corruption_and_delete() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("vault");
        let vault = TokenVault::new(root.clone());
        let value = "token-fixture-Đăng nhập".repeat(2000);
        vault.put("fixture", value.as_bytes()).unwrap();
        assert!(!std::fs::read(root.join("fixture"))
            .unwrap()
            .windows(b"token-fixture".len())
            .any(|w| w == b"token-fixture"));
        assert_eq!(
            &*TokenVault::new(root.clone()).get("fixture").unwrap(),
            value.as_bytes()
        );
        vault.put("fixture", b"rotated").unwrap();
        assert_eq!(&*vault.get("fixture").unwrap(), b"rotated");
        std::fs::create_dir(root.join("blocked")).unwrap();
        assert!(vault.put("blocked", b"test").is_err());
        assert_eq!(&*vault.get("fixture").unwrap(), b"rotated");
        std::fs::write(root.join("fixture"), b"corrupt").unwrap();
        assert!(vault.get("fixture").is_err());
        vault.delete("fixture").unwrap();
        assert!(vault.get("fixture").is_err());
        assert!(vault.get("../x").is_err());
    }
}
