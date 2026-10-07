use crate::{
    now,
    permit::{payload_hash, Permit, PermitAuthority},
};
use cad_contracts::*;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
};

#[derive(Clone, Serialize, Deserialize, Default)]
struct Snapshot {
    schema_version: u32,
    documents: BTreeMap<String, StoredDocument>,
    receipts: BTreeMap<String, ExecutionReceipt>,
}
#[derive(Clone, Serialize, Deserialize)]
struct StoredDocument {
    project_id: String,
    revision: i64,
    geometry: Option<Geometry>,
}
pub struct MockCadGateway {
    snapshot: Snapshot,
    path: PathBuf,
    session: String,
    authority: PermitAuthority,
    _lock: File,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub session_id: String,
    pub key: Vec<u8>,
    pub snapshot_path: PathBuf,
    pub seeds: Vec<DocumentContext>,
    pub allow_initial_import: bool,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MutationRequest {
    pub change: ChangeSet,
    pub permit: Permit,
}
fn io_error<E>(_error: E) -> AppError {
    AppError::new(
        "SNAPSHOT_ERROR",
        "Không đọc/ghi được snapshot CAD; giữ nguyên dữ liệu để khôi phục.",
    )
}
impl MockCadGateway {
    pub fn bootstrap(b: Bootstrap) -> Result<Self, AppError> {
        if !b.snapshot_path.is_absolute() || b.seeds.len() > 1000 || b.session_id.len() > 100 {
            return Err(AppError::new(
                "INVALID_BOOTSTRAP",
                "Thông tin phiên không hợp lệ.",
            ));
        }
        let parent = b.snapshot_path.parent().ok_or_else(|| io_error(()))?;
        // Parent is an app-owned directory created by Core. Refuse symlink components.
        for ancestor in parent.ancestors() {
            if ancestor
                .symlink_metadata()
                .map_err(io_error)?
                .file_type()
                .is_symlink()
            {
                return Err(io_error(()));
            }
        }
        if let Ok(meta) = b.snapshot_path.symlink_metadata() {
            if meta.file_type().is_symlink() || meta.len() > 16 * 1024 * 1024 {
                return Err(io_error(()));
            }
        }
        let lock_path = b.snapshot_path.with_extension("lock");
        if lock_path
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(io_error(()));
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(io_error)?;
        lock.try_lock_exclusive()
            .map_err(|_| AppError::new("CAD_BUSY", "Snapshot đang được instance khác sử dụng."))?;
        let exists = b.snapshot_path.exists();
        if !exists && !b.allow_initial_import {
            return Err(AppError::new(
                "RECOVERY_REQUIRED",
                "Snapshot đã mất; không khởi tạo lại từ cache SQLite.",
            ));
        }
        let mut snapshot = if exists {
            serde_json::from_slice::<Snapshot>(&std::fs::read(&b.snapshot_path).map_err(io_error)?)
                .map_err(io_error)?
        } else {
            Snapshot {
                schema_version: 1,
                ..Snapshot::default()
            }
        };
        if snapshot.schema_version != 1 {
            return Err(io_error(()));
        }
        for seed in b.seeds {
            if seed.mode != "mock"
                || seed.document_id != format!("mock:{}", seed.project_id)
                || seed.revision < 0
            {
                return Err(io_error(()));
            }
            if let Some(g) = &seed.geometry {
                geometry::verify(g, &seed.document_id)?;
            }
            if let std::collections::btree_map::Entry::Vacant(entry) =
                snapshot.documents.entry(seed.document_id)
            {
                if exists && (seed.revision != 0 || seed.geometry.is_some()) {
                    return Err(AppError::new(
                        "RECOVERY_REQUIRED",
                        "Document không có trong snapshot; cần đối chiếu lịch sử.",
                    ));
                }
                entry.insert(StoredDocument {
                    project_id: seed.project_id,
                    revision: seed.revision,
                    geometry: seed.geometry,
                });
            }
        }
        let gateway = Self {
            snapshot,
            path: b.snapshot_path,
            session: b.session_id,
            authority: PermitAuthority::from_key(b.key)?,
            _lock: lock,
        };
        gateway.persist(&gateway.snapshot)?;
        Ok(gateway)
    }
    fn persist(&self, snapshot: &Snapshot) -> Result<(), AppError> {
        let bytes = serde_json::to_vec(snapshot).map_err(io_error)?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(AppError::new(
                "CAD_LIMIT",
                "Snapshot vượt giới hạn; không thay đổi dữ liệu.",
            ));
        }
        let parent = self.path.parent().ok_or_else(|| io_error(()))?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
        temp.write_all(&bytes).map_err(io_error)?;
        temp.as_file().sync_all().map_err(io_error)?;
        temp.persist(&self.path).map_err(io_error)?;
        #[cfg(unix)]
        File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(io_error)?;
        Ok(())
    }
    pub fn context(&self, project_id: &str) -> Result<DocumentContext, AppError> {
        let document_id = format!("mock:{project_id}");
        let d = self
            .snapshot
            .documents
            .get(&document_id)
            .ok_or_else(|| AppError::new("NOT_FOUND", "Không tìm thấy CAD document."))?;
        Ok(DocumentContext {
            document_id,
            project_id: d.project_id.clone(),
            session_id: self.session.clone(),
            revision: d.revision,
            geometry: d.geometry.clone(),
            mode: "mock".into(),
            units: Units::Millimeters,
        })
    }
    pub fn ensure_document(&mut self, project: &str, proof: &[u8]) -> Result<(), AppError> {
        if uuid::Uuid::parse_str(project).is_err()
            || !self
                .authority
                .verify_document_proof(project, &self.session, proof)
        {
            return Err(AppError::new(
                "PERMIT_DENIED",
                "Không có quyền tạo document.",
            ));
        }
        let key = format!("mock:{project}");
        if self.snapshot.documents.contains_key(&key) {
            return Ok(());
        }
        if self.snapshot.documents.len() >= 1000 {
            return Err(AppError::new("CAD_LIMIT", "Quá nhiều document mô phỏng."));
        }
        let mut next = self.snapshot.clone();
        next.documents.insert(
            key,
            StoredDocument {
                project_id: project.into(),
                revision: 0,
                geometry: None,
            },
        );
        self.persist(&next)?;
        self.snapshot = next;
        Ok(())
    }
    pub fn operation(&self, id: &str) -> Option<ExecutionReceipt> {
        self.snapshot.receipts.get(id).cloned()
    }
    pub fn apply(&mut self, request: MutationRequest) -> Result<ExecutionReceipt, AppError> {
        let c = request.change;
        let p = &c.payload;
        self.authority
            .verify(&request.permit, &c.id, p, &self.session, now())?;
        if c.payload_hash != payload_hash(p)?
            || p.mode != RunMode::Execute
            || c.status != "pending_approval"
            || p.schema_version != 1
        {
            return Err(AppError::new(
                "APPROVAL_DENIED",
                "Phương án ghi không hợp lệ.",
            ));
        }
        if let Some(receipt) = self.operation(&c.id) {
            if receipt.payload_hash != c.payload_hash {
                return Err(AppError::new(
                    "REPLAY_MISMATCH",
                    "Operation ID không khớp hash.",
                ));
            }
            return Ok(receipt);
        }
        let context = self.context(&p.project_id)?;
        if p.document_id != context.document_id
            || p.base_revision != context.revision
            || p.before != context.geometry
        {
            return Err(AppError::new("CONFLICT", "CAD đã thay đổi từ khi preview."));
        }
        if let Some(g) = &p.after {
            geometry::verify(g, &p.document_id)?;
        }
        if self.snapshot.receipts.len() >= 10000 {
            return Err(AppError::new(
                "CAD_LIMIT",
                "Lịch sử snapshot đầy; cần bảo trì trước khi ghi.",
            ));
        }
        let receipt = ExecutionReceipt {
            operation_id: c.id.clone(),
            change_set_id: c.id.clone(),
            payload_hash: c.payload_hash.clone(),
            observed_revision: p.base_revision + 1,
            status: "verified".into(),
            saved_to_dwg: false,
        };
        let mut next = self.snapshot.clone();
        let d = next
            .documents
            .get_mut(&p.document_id)
            .ok_or_else(|| io_error(()))?;
        d.geometry = p.after.clone();
        d.revision += 1;
        next.receipts.insert(c.id, receipt.clone());
        self.persist(&next)?;
        self.snapshot = next;
        Ok(receipt)
    }
}
