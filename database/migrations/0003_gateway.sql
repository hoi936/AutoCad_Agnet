-- Durable send journal is deliberately separate from the immutable proposal/status
-- table to preserve 0001 history and its existing CHECK constraint.
CREATE TABLE gateway_state (
 key TEXT PRIMARY KEY, value TEXT NOT NULL
);
CREATE TABLE gateway_operations (
 change_set_id TEXT PRIMARY KEY REFERENCES change_sets(id) ON DELETE RESTRICT,
 operation_id TEXT NOT NULL UNIQUE,
 payload_hash TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('sending','outcome_unknown','verified','recovery_required','not_applied')),
 updated_at INTEGER NOT NULL
);
CREATE INDEX gateway_recovery ON gateway_operations(state,updated_at);
