CREATE TABLE knowledge_notes(note_id TEXT PRIMARY KEY,mod_id TEXT NOT NULL REFERENCES projects(mod_id),game_id TEXT NOT NULL,revision INTEGER NOT NULL,status TEXT NOT NULL,payload TEXT NOT NULL);
CREATE INDEX knowledge_game_status ON knowledge_notes(game_id,status);
