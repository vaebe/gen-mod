CREATE TABLE project_contexts(mod_id TEXT PRIMARY KEY REFERENCES projects(mod_id), revision INTEGER NOT NULL, payload TEXT NOT NULL);
CREATE TABLE context_snapshots(snapshot_id TEXT PRIMARY KEY, mod_id TEXT NOT NULL REFERENCES projects(mod_id), task_id TEXT NOT NULL UNIQUE, payload TEXT NOT NULL);
CREATE TABLE context_proposals(proposal_id TEXT PRIMARY KEY, mod_id TEXT NOT NULL REFERENCES projects(mod_id), status TEXT NOT NULL, payload TEXT NOT NULL);
