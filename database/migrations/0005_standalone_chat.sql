-- Account/chat lifecycle independent of projects, drawings and CAD transports.
CREATE TABLE llm_accounts (
 id TEXT PRIMARY KEY, auth_mode TEXT NOT NULL CHECK(auth_mode IN ('api_key','chatgpt')),
 issuer TEXT, subject TEXT, client_id TEXT, label TEXT NOT NULL,
 secret_ref TEXT NOT NULL UNIQUE, scope TEXT NOT NULL DEFAULT '', expires_at INTEGER,
 selected_model TEXT, status TEXT NOT NULL, catalog_json TEXT NOT NULL DEFAULT '[]',
 catalog_at INTEGER, created_at INTEGER NOT NULL,
 UNIQUE(issuer,subject,client_id)
);
CREATE TABLE app_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE standalone_conversations (
 id TEXT PRIMARY KEY, account_id TEXT NOT NULL REFERENCES llm_accounts(id),
 title TEXT NOT NULL, created_at INTEGER NOT NULL
);
CREATE TABLE standalone_runs (
 id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL REFERENCES standalone_conversations(id),
 account_id TEXT NOT NULL REFERENCES llm_accounts(id), model TEXT NOT NULL,
 status TEXT NOT NULL, error_code TEXT, text TEXT NOT NULL DEFAULT '',
 tool_trace_json TEXT NOT NULL DEFAULT '[]', started_at INTEGER NOT NULL, finished_at INTEGER
);
CREATE UNIQUE INDEX standalone_one_running ON standalone_runs(conversation_id) WHERE status='running';
CREATE TABLE standalone_messages (
 id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL REFERENCES standalone_conversations(id),
 run_id TEXT NOT NULL REFERENCES standalone_runs(id), role TEXT NOT NULL CHECK(role IN ('user','assistant')),
 content TEXT NOT NULL, created_at INTEGER NOT NULL
);
CREATE INDEX standalone_message_history ON standalone_messages(conversation_id,created_at,id);
