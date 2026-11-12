CREATE TABLE projects(mod_id TEXT PRIMARY KEY, payload TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 0, archived INTEGER NOT NULL DEFAULT 0);
CREATE TABLE sessions(mod_id TEXT NOT NULL REFERENCES projects(mod_id), session_id TEXT NOT NULL, payload TEXT NOT NULL, draft TEXT NOT NULL DEFAULT '', thinking_level TEXT NOT NULL DEFAULT 'medium', generation INTEGER NOT NULL DEFAULT 0, last_activity_at TEXT, deleted INTEGER NOT NULL DEFAULT 0, provider_id TEXT, model_id TEXT, PRIMARY KEY(mod_id,session_id));
CREATE TABLE tasks(task_id TEXT PRIMARY KEY, mod_id TEXT NOT NULL, session_id TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL, finished_at TEXT, payload TEXT NOT NULL, FOREIGN KEY(mod_id,session_id) REFERENCES sessions(mod_id,session_id));
CREATE INDEX tasks_session_time ON tasks(mod_id,session_id,created_at);
CREATE TABLE runtime_logs(log_id INTEGER PRIMARY KEY, mod_id TEXT NOT NULL, session_id TEXT NOT NULL, task_id TEXT, timestamp TEXT NOT NULL, payload TEXT NOT NULL);
CREATE INDEX runtime_logs_session ON runtime_logs(mod_id,session_id,log_id);
CREATE TABLE steering_inputs(input_id TEXT PRIMARY KEY, mod_id TEXT NOT NULL, session_id TEXT NOT NULL, task_id TEXT NOT NULL, generation INTEGER NOT NULL, submitted_at TEXT NOT NULL, status TEXT NOT NULL, payload TEXT NOT NULL);
CREATE TABLE session_stats(mod_id TEXT NOT NULL, session_id TEXT NOT NULL, sampled_at TEXT NOT NULL, payload TEXT NOT NULL, PRIMARY KEY(mod_id,session_id), FOREIGN KEY(mod_id,session_id) REFERENCES sessions(mod_id,session_id));
CREATE TABLE file_operations(operation_id TEXT PRIMARY KEY, mod_id TEXT NOT NULL, kind TEXT NOT NULL, status TEXT NOT NULL, payload TEXT NOT NULL);
CREATE TABLE project_contexts(mod_id TEXT PRIMARY KEY REFERENCES projects(mod_id), revision INTEGER NOT NULL, payload TEXT NOT NULL);
CREATE TABLE context_snapshots(snapshot_id TEXT PRIMARY KEY, mod_id TEXT NOT NULL REFERENCES projects(mod_id), task_id TEXT NOT NULL UNIQUE, payload TEXT NOT NULL);
CREATE TABLE delivery_metadata(mod_id TEXT NOT NULL REFERENCES projects(mod_id),version_id TEXT NOT NULL,task_id TEXT NOT NULL,payload TEXT NOT NULL,PRIMARY KEY(mod_id,version_id));
CREATE TABLE feedback_drafts(draft_id TEXT PRIMARY KEY,mod_id TEXT NOT NULL,version_id TEXT NOT NULL,revision INTEGER NOT NULL,status TEXT NOT NULL,payload TEXT NOT NULL,FOREIGN KEY(mod_id,version_id) REFERENCES delivery_metadata(mod_id,version_id));
CREATE TABLE verification_records(record_id TEXT PRIMARY KEY,source_draft_id TEXT NOT NULL UNIQUE REFERENCES feedback_drafts(draft_id),mod_id TEXT NOT NULL,version_id TEXT NOT NULL,confirmed_at TEXT NOT NULL,payload TEXT NOT NULL,supersedes TEXT REFERENCES verification_records(record_id),FOREIGN KEY(mod_id,version_id) REFERENCES delivery_metadata(mod_id,version_id));
CREATE TABLE inspection_reports(report_id TEXT PRIMARY KEY,mod_id TEXT NOT NULL REFERENCES projects(mod_id),directory TEXT NOT NULL,payload TEXT NOT NULL);
CREATE TABLE knowledge_notes(
 note_id TEXT PRIMARY KEY,
 origin_mod_id TEXT,
 game_id TEXT NOT NULL,
 revision INTEGER NOT NULL,
 status TEXT NOT NULL,
 allow_reuse INTEGER NOT NULL,
 user_edited INTEGER NOT NULL DEFAULT 0,
 memory_key TEXT NOT NULL UNIQUE,
 payload TEXT NOT NULL,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE INDEX knowledge_reuse ON knowledge_notes(game_id,status,allow_reuse);
CREATE TABLE memory_content_links(
 game_id TEXT NOT NULL,
 content_sha256 TEXT NOT NULL,
 note_id TEXT NOT NULL REFERENCES knowledge_notes(note_id),
 PRIMARY KEY(game_id,content_sha256,note_id)
);
CREATE TABLE task_memory_uses(
 task_id TEXT NOT NULL REFERENCES tasks(task_id),
 mod_id TEXT NOT NULL,
 note_id TEXT NOT NULL,
 revision INTEGER NOT NULL,
 payload TEXT NOT NULL,
 PRIMARY KEY(task_id,note_id,revision)
);
CREATE INDEX memory_uses_project ON task_memory_uses(mod_id);
CREATE TABLE project_removals(mod_id TEXT PRIMARY KEY,operation_id TEXT NOT NULL UNIQUE,removed_at TEXT NOT NULL);
CREATE TABLE submission_requests (
    request_id TEXT PRIMARY KEY,
    mod_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    status TEXT NOT NULL,
    task_id TEXT,
    error TEXT,
    message_timestamp INTEGER,
    message_fingerprint TEXT,
    FOREIGN KEY(mod_id,session_id) REFERENCES sessions(mod_id,session_id) ON DELETE CASCADE
);
