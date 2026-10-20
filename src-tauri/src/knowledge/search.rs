use super::models::*;
use crate::storage::repository::decode;
use rusqlite::{params, Connection};
pub fn search_notes(
    db: &Connection,
    game: &str,
    version: Option<&str>,
    terms: &[String],
    limit: usize,
) -> Result<Vec<KnowledgeNote>, String> {
    let mut q=db.prepare(&format!("{} WHERE (game_id=?1 OR (game_id='*' AND user_edited=1 AND json_extract(payload,'$.evidenceKind')='manual' AND json_extract(payload,'$.versionScope')='independent')) AND status='active' AND allow_reuse=1 AND (json_extract(payload,'$.versionScope') IN ('independent','unknown') OR (?2 IS NOT NULL AND json_extract(payload,'$.versionScope')='exact' AND json_extract(payload,'$.gameVersion')=?2))", super::repository::NOTE_SELECT)).map_err(|e|e.to_string())?;
    let rows = q
        .query_map(params![game, version], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut ranked = vec![];
    for row in rows {
        let mut note: KnowledgeNote = decode(row.map_err(|e| e.to_string())?)?;
        let haystack = format!(
            "{} {} {}",
            note.input.title, note.input.route, note.input.content
        )
        .to_lowercase();
        let hits = terms
            .iter()
            .filter(|s| !s.trim().is_empty() && haystack.contains(&s.to_lowercase()))
            .count();
        if !terms.is_empty() && hits == 0 {
            continue;
        }
        let exact = usize::from(note.input.version_scope == "exact");
        note.input.content = note.input.content.chars().take(2000).collect();
        ranked.push((exact, hits, note));
    }
    ranked.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(b.1.cmp(&a.1))
            .then(a.2.note_id.cmp(&b.2.note_id))
    });
    Ok(ranked
        .into_iter()
        .take(limit.min(5))
        .map(|(_, _, v)| v)
        .collect())
}
