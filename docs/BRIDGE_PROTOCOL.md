# CAD Agent AutoCAD bridge protocol v1 — R2 read-only

Local Windows named pipe `cadagent-r2-{acad_pid}`. Name/PID are discovery hints, never
identity proof. Frame: 4-byte little-endian unsigned length followed by UTF-8 JSON, 1–65536
bytes. Maximum 1 request in flight per connection, 5-second read/write and 10-second
AutoCAD context timeout; no unsolicited frame. Response echoes `request_id` and
`session_id`. `version` must equal 1. Unknown operation or field fails closed.

Request envelope: `{version,request_id,session_id,operation,document_id?,args}`.
First request `pair` contains one-time pairing code shown by the plugin in an AutoCAD
modal window, plus the expected acad PID. The plugin returns an unpredictable session ID
and capabilities (all read-only, `write=false`); code is invalidated immediately. Subsequent
requests require that session ID and are rejected after disconnect/restart. App must
verify Windows named-pipe server PID and a successful document readback before reporting
connected. Do not put the code in SQLite, logs, process arguments, MCP args or LLM context.

Read operations: `get_status`, `list_documents`, `get_document_context`, `get_selection`,
`list_layers`, `query_entities`, `validate_entities`, `get_operation_status`. Reads require
an explicit document ID except status/list. Query has bounded filters and max 100 entities.
No write/apply command exists in the R2 live server. Errors have stable codes such as
`NOT_PAIRED`, `SESSION_CHANGED`, `DOCUMENT_CHANGED`, `DOCUMENT_UNAVAILABLE`,
`CAD_BUSY`, `READ_ONLY_MODE`, `INVALID_ARGUMENTS`, `FRAME_TOO_LARGE`.

The pipe listener only parses/fragments frames. AutoCAD database reads execute in an
AutoCAD main-thread/command context through a short queued dispatcher, with read-only
transactions. No pipe I/O or LLM wait while a document lock/transaction is held. Document
identity is session-scoped and independent of filename; revision changes when document
state changes, with no DWG metadata write. Every result is bounded before reaching LLM.

Protocol/transport fixtures do not certify plugin load or AutoCAD live acceptance.
