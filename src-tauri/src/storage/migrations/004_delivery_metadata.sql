CREATE TABLE delivery_metadata(mod_id TEXT NOT NULL REFERENCES projects(mod_id),version_id TEXT NOT NULL,task_id TEXT NOT NULL,payload TEXT NOT NULL,PRIMARY KEY(mod_id,version_id));
