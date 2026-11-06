CREATE TABLE submission_requests (
    request_id TEXT PRIMARY KEY,
    mod_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    status TEXT NOT NULL,
    task_id TEXT,
    error TEXT,
    FOREIGN KEY(mod_id,session_id) REFERENCES sessions(mod_id,session_id) ON DELETE CASCADE
);
