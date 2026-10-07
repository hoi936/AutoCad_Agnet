use std::{collections::HashMap, fmt, sync::Mutex};
use zeroize::Zeroizing;

/// Deliberately not Serialize or Display. Debug never prints the value.
pub struct SecretValue(Zeroizing<String>);

impl SecretValue {
    pub fn new(value: String) -> Result<Self, SecretError> {
        let value = Zeroizing::new(value);
        if value.is_empty() || value.len() > 32768 || !value.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(SecretError::InvalidSecret);
        }
        Ok(Self(value))
    }
    /// Backend-only access for the authenticated HTTP request.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretValue([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecretError {
    #[error("Windows Credential Manager chưa khả dụng trên hệ điều hành này.")]
    Unsupported,
    #[error("Không thể truy cập kho khóa; chưa lưu hoặc xóa khóa.")]
    Unavailable,
    #[error("Mã tham chiếu credential không hợp lệ.")]
    InvalidReference,
    #[error("Khóa API trống hoặc có định dạng không hợp lệ.")]
    InvalidSecret,
}

pub trait SecretStore: Send + Sync {
    fn set(&self, secret_ref: &str, secret: SecretValue) -> Result<(), SecretError>;
    fn get(&self, secret_ref: &str) -> Result<Option<SecretValue>, SecretError>;
    fn delete(&self, secret_ref: &str) -> Result<(), SecretError>;
    fn is_configured(&self, secret_ref: &str) -> Result<bool, SecretError> {
        Ok(self.get(secret_ref)?.is_some())
    }
}

fn validate_reference(secret_ref: &str) -> Result<(), SecretError> {
    if secret_ref.is_empty()
        || secret_ref.len() > 160
        || !secret_ref
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
    {
        return Err(SecretError::InvalidReference);
    }
    Ok(())
}

#[derive(Default)]
pub struct WindowsCredentialStore;

impl WindowsCredentialStore {
    pub fn new() -> Self {
        Self
    }

    #[cfg(windows)]
    fn entry(secret_ref: &str) -> Result<keyring::Entry, SecretError> {
        validate_reference(secret_ref)?;
        // Fixed application namespace; never enumerate other credentials.
        let credential = keyring::windows::WinCredential::new_with_target(
            None,
            "com.cad-agent-studio.providers.v1",
            secret_ref,
        )
        .map_err(|_| SecretError::Unavailable)?;
        // Select the native Windows backend explicitly instead of accepting a
        // process-global keyring builder that could point at a mock backend.
        Ok(keyring::Entry::new_with_credential(Box::new(credential)))
    }
}

#[cfg(windows)]
impl SecretStore for WindowsCredentialStore {
    fn set(&self, secret_ref: &str, secret: SecretValue) -> Result<(), SecretError> {
        Self::entry(secret_ref)?
            .set_password(secret.expose())
            .map_err(|_| SecretError::Unavailable)
    }
    fn get(&self, secret_ref: &str) -> Result<Option<SecretValue>, SecretError> {
        match Self::entry(secret_ref)?.get_password() {
            Ok(value) => SecretValue::new(value).map(Some),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(SecretError::Unavailable),
        }
    }
    fn delete(&self, secret_ref: &str) -> Result<(), SecretError> {
        match Self::entry(secret_ref)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(SecretError::Unavailable),
        }
    }
}

#[cfg(not(windows))]
impl SecretStore for WindowsCredentialStore {
    fn set(&self, secret_ref: &str, _secret: SecretValue) -> Result<(), SecretError> {
        validate_reference(secret_ref)?;
        Err(SecretError::Unsupported)
    }
    fn get(&self, secret_ref: &str) -> Result<Option<SecretValue>, SecretError> {
        validate_reference(secret_ref)?;
        Err(SecretError::Unsupported)
    }
    fn delete(&self, secret_ref: &str) -> Result<(), SecretError> {
        validate_reference(secret_ref)?;
        Err(SecretError::Unsupported)
    }
}

/// Only construct after the user explicitly selects session-only storage.
/// Closing the process drops and zeroizes all entries. This is not a vault fallback.
pub struct SessionSecretStore {
    entries: Mutex<HashMap<String, SecretValue>>,
}

impl SessionSecretStore {
    pub fn new_explicit_session() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }
}

impl SecretStore for SessionSecretStore {
    fn set(&self, secret_ref: &str, secret: SecretValue) -> Result<(), SecretError> {
        validate_reference(secret_ref)?;
        self.entries
            .lock()
            .map_err(|_| SecretError::Unavailable)?
            .insert(secret_ref.into(), secret);
        Ok(())
    }
    fn get(&self, secret_ref: &str) -> Result<Option<SecretValue>, SecretError> {
        validate_reference(secret_ref)?;
        self.entries
            .lock()
            .map_err(|_| SecretError::Unavailable)?
            .get(secret_ref)
            .map(|value| SecretValue::new(value.expose().to_owned()))
            .transpose()
    }
    fn delete(&self, secret_ref: &str) -> Result<(), SecretError> {
        validate_reference(secret_ref)?;
        self.entries
            .lock()
            .map_err(|_| SecretError::Unavailable)?
            .remove(secret_ref);
        Ok(())
    }
}

#[cfg(any(test, feature = "test-support"))]
pub struct MemorySecretStore(SessionSecretStore);

#[cfg(any(test, feature = "test-support"))]
impl Default for MemorySecretStore {
    fn default() -> Self {
        Self(SessionSecretStore::new_explicit_session())
    }
}

#[cfg(any(test, feature = "test-support"))]
impl SecretStore for MemorySecretStore {
    fn set(&self, key: &str, value: SecretValue) -> Result<(), SecretError> {
        self.0.set(key, value)
    }
    fn get(&self, key: &str) -> Result<Option<SecretValue>, SecretError> {
        self.0.get(key)
    }
    fn delete(&self, key: &str) -> Result<(), SecretError> {
        self.0.delete(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_delete_and_debug_never_reveal_secret() {
        let store = MemorySecretStore::default();
        let secret = SecretValue::new("sk-test-private-sentinel".into()).unwrap();
        assert!(!format!("{secret:?}").contains(secret.expose()));
        store.set("provider:fixture", secret).unwrap();
        assert!(store.is_configured("provider:fixture").unwrap());
        store
            .set(
                "provider:fixture",
                SecretValue::new("replacement".into()).unwrap(),
            )
            .unwrap();
        assert_eq!(
            store.get("provider:fixture").unwrap().unwrap().expose(),
            "replacement"
        );
        store.delete("provider:fixture").unwrap();
        assert!(!store.is_configured("provider:fixture").unwrap());
        assert!(matches!(
            store.get("../another-app"),
            Err(SecretError::InvalidReference)
        ));
    }

    #[cfg(not(windows))]
    #[test]
    fn unsupported_vault_never_falls_back_to_plaintext() {
        let store = WindowsCredentialStore::new();
        assert_eq!(
            store.set("test", SecretValue::new("sentinel".into()).unwrap()),
            Err(SecretError::Unsupported)
        );
        assert!(matches!(store.get("test"), Err(SecretError::Unsupported)));
        assert_eq!(store.delete("test"), Err(SecretError::Unsupported));
    }

    #[test]
    fn poisoned_session_store_reports_vault_error() {
        let store = SessionSecretStore::new_explicit_session();
        let _ = std::panic::catch_unwind(|| {
            let _guard = store.entries.lock().unwrap();
            panic!("fixture");
        });
        assert_eq!(
            store.set("test", SecretValue::new("sentinel".into()).unwrap()),
            Err(SecretError::Unavailable)
        );
        assert!(matches!(store.get("test"), Err(SecretError::Unavailable)));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "Explicit Windows credential integration; writes only a unique test namespace entry"]
    fn windows_credential_roundtrip_and_cleanup() {
        let store = WindowsCredentialStore::new();
        let key = format!(
            "integration-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        struct Cleanup<'a>(&'a WindowsCredentialStore, String);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                let _ = self.0.delete(&self.1);
            }
        }
        let _cleanup = Cleanup(&store, key.clone());
        store
            .set(
                &key,
                SecretValue::new("fixture-only-not-a-real-key".into()).unwrap(),
            )
            .unwrap();
        assert!(store.is_configured(&key).unwrap());
        store
            .set(&key, SecretValue::new("replaced-fixture".into()).unwrap())
            .unwrap();
        assert_eq!(
            store.get(&key).unwrap().unwrap().expose(),
            "replaced-fixture"
        );
        store.delete(&key).unwrap();
        assert!(!store.is_configured(&key).unwrap());
    }
}
