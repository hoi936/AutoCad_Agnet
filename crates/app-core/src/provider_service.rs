use crate::{Result, Studio};
use cad_contracts::{AppError, ProviderConfig};
use cad_model_provider::{
    MockProvider, ModelProvider, OpenAiProvider, SecretStore, SecretValue, SessionSecretStore,
    WindowsCredentialStore,
};
use std::sync::{Arc, OnceLock};

/// No command exposes get_secret. Only the HTTP adapter receives SecretValue.
pub struct ProviderService {
    studio: Arc<Studio>,
    vault: Arc<dyn SecretStore>,
    session: OnceLock<SessionSecretStore>,
}
impl ProviderService {
    pub fn new(studio: Arc<Studio>) -> Self {
        Self {
            studio,
            vault: Arc::new(WindowsCredentialStore::new()),
            session: OnceLock::new(),
        }
    }
    pub fn with_vault(studio: Arc<Studio>, vault: Arc<dyn SecretStore>) -> Self {
        Self {
            studio,
            vault,
            session: OnceLock::new(),
        }
    }
    fn store(&self, config: &ProviderConfig) -> Result<&dyn SecretStore> {
        match config.storage.as_str() {
            "windows" => Ok(self.vault.as_ref()),
            "session" => Ok(self
                .session
                .get_or_init(SessionSecretStore::new_explicit_session)),
            _ => Err(AppError::new(
                "INVALID_STORAGE",
                "Nơi lưu khóa chưa được hỗ trợ.",
            )),
        }
    }
    pub async fn config(&self, project: &str) -> Result<ProviderConfig> {
        let mut config = self.studio.provider_get_config(project).await?;
        config.configured = if config.provider == "mock" {
            true
        } else {
            self.store(&config)?
                .is_configured(config.secret_ref.as_deref().ok_or_else(missing)?)
                .map_err(secret_error)?
        };
        Ok(config)
    }
    pub async fn save_config(
        &self,
        project: &str,
        provider: &str,
        model: &str,
        consent: bool,
        storage: &str,
    ) -> Result<ProviderConfig> {
        self.studio
            .provider_save_config(project, provider, model, consent, storage)
            .await?;
        self.config(project).await
    }
    pub async fn set_secret(&self, project: &str, value: String) -> Result<()> {
        let secret = SecretValue::new(value).map_err(secret_error)?;
        let config = self.studio.provider_get_config(project).await?;
        if config.provider != "openai" {
            return Err(missing());
        }
        self.store(&config)?
            .set(config.secret_ref.as_deref().ok_or_else(missing)?, secret)
            .map_err(secret_error)
    }
    pub async fn delete_secret(&self, project: &str) -> Result<()> {
        let config = self.studio.provider_get_config(project).await?;
        if config.provider != "openai" {
            return Err(missing());
        }
        self.store(&config)?
            .delete(config.secret_ref.as_deref().ok_or_else(missing)?)
            .map_err(secret_error)
    }
    pub async fn model_for(&self, project: &str) -> Result<Arc<dyn ModelProvider>> {
        let config = self.studio.provider_get_config(project).await?;
        if config.provider == "mock" {
            return Ok(Arc::new(MockProvider::new()));
        }
        if !config.cloud_consent {
            return Err(AppError::new(
                "CLOUD_CONSENT_REQUIRED",
                "Chưa đồng ý gửi dữ liệu dự án tới OpenAI.",
            ));
        }
        let secret = self
            .store(&config)?
            .get(config.secret_ref.as_deref().ok_or_else(missing)?)
            .map_err(secret_error)?
            .ok_or_else(missing)?;
        Ok(Arc::new(
            OpenAiProvider::new(secret).map_err(|e| AppError::new(e.code, e.message))?,
        ))
    }
}
fn missing() -> AppError {
    AppError::new(
        "PROVIDER_NOT_CONFIGURED",
        "Thiếu cấu hình hoặc API key. Nhập khóa trong Settings; không tự chuyển sang mock.",
    )
}
fn secret_error(e: cad_model_provider::SecretError) -> AppError {
    AppError::new("VAULT_ERROR", &e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn session_opt_in_missing_key_consent_and_no_db_secret() {
        let dir = tempfile::tempdir().unwrap();
        let studio = Arc::new(Studio::open(&dir.path().join("db")).await.unwrap());
        let project = studio
            .projects_create("Vault fixture".into())
            .await
            .unwrap();
        let service = ProviderService::new(studio.clone());
        service
            .save_config(&project.id, "openai", "fixture-model", true, "session")
            .await
            .unwrap();
        assert!(service.model_for(&project.id).await.is_err());
        let sentinel = "sk-unique-never-persist-this-sentinel";
        service
            .set_secret(&project.id, sentinel.into())
            .await
            .unwrap();
        assert!(service.config(&project.id).await.unwrap().configured);
        let config_json =
            serde_json::to_string(&service.config(&project.id).await.unwrap()).unwrap();
        assert!(!config_json.contains(sentinel));
        let bytes = std::fs::read(dir.path().join("db")).unwrap();
        assert!(!bytes
            .windows(sentinel.len())
            .any(|w| w == sentinel.as_bytes()));
        service
            .save_config(&project.id, "openai", "fixture-model", false, "session")
            .await
            .unwrap();
        assert_eq!(
            service.model_for(&project.id).await.err().unwrap().code,
            "CLOUD_CONSENT_REQUIRED"
        );
        service.delete_secret(&project.id).await.unwrap();
        assert!(!service.config(&project.id).await.unwrap().configured);
    }
}
