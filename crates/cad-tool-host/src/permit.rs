use cad_contracts::{AppError, ChangePayload};
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

type HmacSha256 = Hmac<Sha256>;
pub fn payload_hash(p: &ChangePayload) -> Result<String, AppError> {
    serde_json::to_vec(p)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|_| AppError::new("INVALID_PAYLOAD", "Không mã hóa được phương án."))
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Permit {
    pub operation_id: String,
    pub document_id: String,
    pub session_id: String,
    pub revision: i64,
    pub payload_hash: String,
    pub scope: String,
    pub expires_at: i64,
    pub mac: Vec<u8>,
}
pub struct PermitAuthority {
    key: Zeroizing<Vec<u8>>,
}
impl PermitAuthority {
    pub fn random() -> Self {
        let mut key = vec![0; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        Self {
            key: Zeroizing::new(key),
        }
    }
    pub fn from_key(key: Vec<u8>) -> Result<Self, AppError> {
        if key.len() != 32 {
            return Err(AppError::new("INVALID_BOOTSTRAP", "Phiên không hợp lệ."));
        }
        Ok(Self {
            key: Zeroizing::new(key),
        })
    }
    // Only sent once over the private child's stdin, never CLI/env/UI/log.
    pub fn bootstrap_key(&self) -> Vec<u8> {
        self.key.to_vec()
    }
    fn signed_bytes(p: &Permit) -> Vec<u8> {
        serde_json::to_vec(&(
            &p.operation_id,
            &p.document_id,
            &p.session_id,
            p.revision,
            &p.payload_hash,
            &p.scope,
            p.expires_at,
        ))
        .expect("serializable claims")
    }
    pub fn issue(
        &self,
        operation_id: &str,
        p: &ChangePayload,
        expires_at: i64,
    ) -> Result<Permit, AppError> {
        let mut permit = Permit {
            operation_id: operation_id.into(),
            document_id: p.document_id.clone(),
            session_id: p.session_id.clone(),
            revision: p.base_revision,
            payload_hash: payload_hash(p)?,
            scope: if p.reverts.is_some() {
                "cad.revert"
            } else {
                "cad.apply"
            }
            .into(),
            expires_at,
            mac: vec![],
        };
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("valid HMAC key");
        mac.update(&Self::signed_bytes(&permit));
        permit.mac = mac.finalize().into_bytes().to_vec();
        Ok(permit)
    }
    pub fn verify(
        &self,
        permit: &Permit,
        id: &str,
        p: &ChangePayload,
        session: &str,
        now: i64,
    ) -> Result<(), AppError> {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("valid key");
        mac.update(&Self::signed_bytes(permit));
        if mac.verify_slice(&permit.mac).is_err()
            || permit.operation_id != id
            || permit.document_id != p.document_id
            || permit.session_id != session
            || p.session_id != session
            || permit.revision != p.base_revision
            || permit.payload_hash != payload_hash(p)?
            || permit.expires_at <= now
            || permit.expires_at > now + 120
            || p.expires_at <= now
            || permit.scope
                != if p.reverts.is_some() {
                    "cad.revert"
                } else {
                    "cad.apply"
                }
        {
            return Err(AppError::new(
                "PERMIT_DENIED",
                "Permit không hợp lệ, hết hạn hoặc sai phiên/document.",
            ));
        }
        Ok(())
    }
    pub fn document_proof(&self, project: &str, session: &str) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("valid key");
        mac.update(b"ensure_document\0");
        mac.update(project.as_bytes());
        mac.update(b"\0");
        mac.update(session.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }
    pub fn verify_document_proof(&self, project: &str, session: &str, proof: &[u8]) -> bool {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("valid key");
        mac.update(b"ensure_document\0");
        mac.update(project.as_bytes());
        mac.update(b"\0");
        mac.update(session.as_bytes());
        mac.verify_slice(proof).is_ok()
    }
}
