-- Preserve immutable payloads and historical records; invalidation is separate metadata.
CREATE TABLE proposal_invalidations (
 change_set_id TEXT PRIMARY KEY REFERENCES change_sets(id) ON DELETE RESTRICT,
 reason TEXT NOT NULL CHECK(reason IN ('superseded','new_request','provider_changed'))
);
-- Historical pending previews require a fresh plan after upgrading.
INSERT INTO proposal_invalidations SELECT id,'superseded' FROM change_sets
 WHERE status IN ('pending_approval','planned')
 AND NOT EXISTS(SELECT 1 FROM gateway_operations g WHERE g.change_set_id=change_sets.id AND g.state!='verified');
UPDATE change_sets SET status='rejected' WHERE id IN (SELECT change_set_id FROM proposal_invalidations);
-- Stricter than per-conversation: one active pending proposal per drawing, including forms.
CREATE TRIGGER supersede_document_proposals BEFORE INSERT ON change_sets
BEGIN
 INSERT OR IGNORE INTO proposal_invalidations
 SELECT id,'superseded' FROM change_sets WHERE document_id=NEW.document_id
 AND status IN ('pending_approval','planned')
 AND NOT EXISTS(SELECT 1 FROM gateway_operations g WHERE g.change_set_id=change_sets.id AND g.state!='verified');
 UPDATE change_sets SET status='rejected' WHERE document_id=NEW.document_id
 AND status IN ('pending_approval','planned')
 AND NOT EXISTS(SELECT 1 FROM gateway_operations g WHERE g.change_set_id=change_sets.id AND g.state!='verified');
END;
