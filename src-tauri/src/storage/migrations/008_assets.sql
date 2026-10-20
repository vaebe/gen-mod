CREATE TABLE project_assets(asset_id TEXT PRIMARY KEY,mod_id TEXT NOT NULL REFERENCES projects(mod_id),payload TEXT NOT NULL);
