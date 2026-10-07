CREATE TABLE projects (
 id TEXT PRIMARY KEY, name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 120), created_at INTEGER NOT NULL
);
CREATE TABLE documents (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL UNIQUE REFERENCES projects(id) ON DELETE RESTRICT,
 mode TEXT NOT NULL CHECK(mode = 'mock'), revision INTEGER NOT NULL DEFAULT 0 CHECK(revision >= 0),
 geometry_json TEXT CHECK(geometry_json IS NULL OR json_valid(geometry_json))
);
CREATE TABLE change_sets (
 id TEXT PRIMARY KEY, document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE RESTRICT,
 payload_json TEXT NOT NULL CHECK(json_valid(payload_json)), payload_hash TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('pending_approval','planned','verified','rejected','conflict','expired')),
 created_at INTEGER NOT NULL
);
CREATE INDEX changes_document ON change_sets(document_id, created_at);
CREATE TABLE approvals (
 id TEXT PRIMARY KEY, change_set_id TEXT NOT NULL UNIQUE REFERENCES change_sets(id) ON DELETE RESTRICT,
 payload_hash TEXT NOT NULL, session_id TEXT NOT NULL, decided_at INTEGER NOT NULL,
 decision TEXT NOT NULL CHECK(decision = 'approved')
);
CREATE TABLE execution_receipts (
 operation_id TEXT PRIMARY KEY, change_set_id TEXT NOT NULL UNIQUE REFERENCES change_sets(id) ON DELETE RESTRICT,
 receipt_json TEXT NOT NULL CHECK(json_valid(receipt_json))
);
CREATE TABLE audit_events (
 id TEXT PRIMARY KEY, target_id TEXT NOT NULL, action TEXT NOT NULL, created_at INTEGER NOT NULL
);
CREATE INDEX audit_time ON audit_events(created_at);
