-- Existing CAD history remains in 0001 tables; links do not rewrite immutable payloads.
CREATE TABLE conversations (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
 title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 120),
 created_at INTEGER NOT NULL
);
CREATE INDEX conversations_project ON conversations(project_id, created_at);

CREATE TABLE provider_configs (
 project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE RESTRICT,
 provider TEXT NOT NULL CHECK(provider IN ('mock','openai')),
 model TEXT NOT NULL CHECK(length(model) BETWEEN 1 AND 120),
 secret_ref TEXT,
 cloud_consent INTEGER NOT NULL DEFAULT 0 CHECK(cloud_consent IN (0,1)),
 storage TEXT NOT NULL CHECK(storage IN ('windows','session')),
 updated_at INTEGER NOT NULL,
 CHECK(provider != 'mock' OR secret_ref IS NULL)
);

CREATE TABLE agent_runs (
 id TEXT PRIMARY KEY,
 conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE RESTRICT,
 document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE RESTRICT,
 provider_config_id TEXT REFERENCES provider_configs(project_id) ON DELETE RESTRICT,
 mode TEXT NOT NULL CHECK(mode IN ('ask','plan','execute')),
 provider TEXT NOT NULL CHECK(provider IN ('mock','openai')),
 model TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('running','completed','awaiting_approval','planned','cancelled','error','interrupted','rejected','verified','outcome_unknown')),
 error_code TEXT,
 started_at INTEGER NOT NULL,
 finished_at INTEGER,
 limits_json TEXT NOT NULL CHECK(json_valid(limits_json))
);
CREATE UNIQUE INDEX one_active_run_per_conversation ON agent_runs(conversation_id) WHERE status='running';
CREATE INDEX runs_conversation ON agent_runs(conversation_id,started_at);

CREATE TABLE messages (
 id TEXT PRIMARY KEY,
 conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE RESTRICT,
 run_id TEXT REFERENCES agent_runs(id) ON DELETE RESTRICT,
 role TEXT NOT NULL CHECK(role IN ('user','assistant','system')),
 content TEXT NOT NULL CHECK(length(content) <= 65536),
 created_at INTEGER NOT NULL
);
CREATE INDEX messages_conversation ON messages(conversation_id,created_at);

CREATE TABLE run_events (
 run_id TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE RESTRICT,
 sequence INTEGER NOT NULL CHECK(sequence > 0),
 kind TEXT NOT NULL,
 safe_payload_json TEXT NOT NULL CHECK(json_valid(safe_payload_json)),
 created_at INTEGER NOT NULL,
 PRIMARY KEY(run_id,sequence)
);

CREATE TABLE tool_calls (
 id TEXT PRIMARY KEY,
 run_id TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE RESTRICT,
 response_index INTEGER NOT NULL CHECK(response_index >= 0),
 provider_call_id TEXT NOT NULL,
 tool_name TEXT NOT NULL,
 input_hash TEXT NOT NULL,
 safe_input_json TEXT NOT NULL CHECK(json_valid(safe_input_json)),
 safe_output_json TEXT CHECK(safe_output_json IS NULL OR json_valid(safe_output_json)),
 status TEXT NOT NULL CHECK(status IN ('started','completed','failed')),
 started_at INTEGER NOT NULL,
 finished_at INTEGER,
 UNIQUE(run_id,response_index,provider_call_id)
);

CREATE TABLE run_change_sets (
 run_id TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE RESTRICT,
 change_set_id TEXT NOT NULL UNIQUE REFERENCES change_sets(id) ON DELETE RESTRICT,
 PRIMARY KEY(run_id,change_set_id)
);
