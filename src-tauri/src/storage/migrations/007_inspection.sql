CREATE TABLE inspection_reports(report_id TEXT PRIMARY KEY,mod_id TEXT NOT NULL REFERENCES projects(mod_id),directory TEXT NOT NULL,payload TEXT NOT NULL);
