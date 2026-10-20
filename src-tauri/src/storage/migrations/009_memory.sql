CREATE TABLE knowledge_notes_next(
 note_id TEXT PRIMARY KEY,
 origin_mod_id TEXT,
 game_id TEXT NOT NULL,
 revision INTEGER NOT NULL,
 status TEXT NOT NULL,
 allow_reuse INTEGER NOT NULL,
 user_edited INTEGER NOT NULL DEFAULT 0,
 memory_key TEXT NOT NULL UNIQUE,
 payload TEXT NOT NULL,
 created_at TEXT,
 updated_at TEXT
);
INSERT INTO knowledge_notes_next
 (note_id,origin_mod_id,game_id,revision,status,allow_reuse,user_edited,memory_key,payload)
 SELECT note_id,mod_id,game_id,revision,status,
   COALESCE(json_extract(payload,'$.allowReuse'),0),1,'legacy:'||note_id,payload
 FROM knowledge_notes;
DROP TABLE knowledge_notes;
ALTER TABLE knowledge_notes_next RENAME TO knowledge_notes;
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
