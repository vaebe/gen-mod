use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentFinding {
    pub code: String,
    pub level: String,
    pub path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentCheckReport {
    pub rules_version: String,
    pub findings: Vec<ContentFinding>,
}
pub fn check_bytes(
    relative: &str,
    bytes: &[u8],
    known_secrets: &[String],
    restricted_hashes: &BTreeSet<String>,
) -> Vec<ContentFinding> {
    use sha2::{Digest, Sha256};
    let mut findings = vec![];
    let mut add = |code: &str| {
        findings.push(ContentFinding {
            code: code.into(),
            level: "block".into(),
            path: relative.into(),
        })
    };
    let name = relative
        .rsplit('/')
        .next()
        .unwrap_or(relative)
        .to_lowercase();
    if name == ".env" || name.ends_with(".env") {
        add("env_file");
    }
    if [
        "-----BEGIN PRIVATE KEY-----",
        "-----BEGIN RSA PRIVATE KEY-----",
        "-----BEGIN EC PRIVATE KEY-----",
        "-----BEGIN OPENSSH PRIVATE KEY-----",
        "-----BEGIN ENCRYPTED PRIVATE KEY-----",
    ]
    .iter()
    .any(|header| bytes.windows(header.len()).any(|v| v == header.as_bytes()))
    {
        add("private_key");
    }
    if known_secrets
        .iter()
        .filter(|s| !s.is_empty())
        .any(|secret| bytes.windows(secret.len()).any(|v| v == secret.as_bytes()))
    {
        add("known_secret");
    }
    if restricted_hashes.contains(&format!("{:x}", Sha256::digest(bytes))) {
        add("reference_copy");
    }
    findings
}
pub fn check_files(
    root: &std::path::Path,
    files: &[(String, String)],
    secrets: &[String],
    restricted: &BTreeSet<String>,
) -> Result<ContentCheckReport, String> {
    use std::io::Read;
    let mut findings = vec![];
    for (source, target) in files {
        crate::changes::relative_path(target)?;
        let path = super::registry::checked_path(root, source)?;
        let mut file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
        let mut buffer = vec![0; 65536];
        let mut tail = vec![];
        let overlap = secrets.iter().map(String::len).max().unwrap_or(0).max(64);
        let size = file.metadata().map_err(|e| e.to_string())?.len();
        let mut hash = sha2::Sha256::new();
        use sha2::Digest;
        let mut first = true;
        loop {
            let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            if first
                && size > 64 * 1024 * 1024
                && !buffer[..count].contains(&0)
                && std::str::from_utf8(&buffer[..count]).is_ok()
            {
                return Err(format!("文本文件超过 64 MiB，无法完成内容检查：{target}"));
            }
            first = false;
            hash.update(&buffer[..count]);
            tail.extend_from_slice(&buffer[..count]);
            findings.extend(check_bytes(target, &tail, secrets, &Default::default()));
            let keep = tail.len().saturating_sub(overlap);
            tail.drain(..keep);
        }
        if size == 0 {
            findings.extend(check_bytes(target, &[], secrets, &Default::default()));
        }
        if restricted.contains(&format!("{:x}", hash.finalize())) {
            findings.push(ContentFinding {
                code: "reference_copy".into(),
                level: "block".into(),
                path: target.into(),
            });
        }
    }
    findings.sort_by(|a, b| (&a.path, &a.code).cmp(&(&b.path, &b.code)));
    findings.dedup_by(|a, b| a.path == b.path && a.code == b.code);
    Ok(ContentCheckReport {
        rules_version: "1".into(),
        findings,
    })
}
pub fn restricted_for_task(
    app: &tauri::AppHandle,
    task: &crate::pi::tasks::TaskContext,
) -> Result<BTreeSet<String>, String> {
    if task.context_snapshot_id.is_none() {
        return Ok(Default::default());
    }
    Ok(crate::project_context::repository::snapshot_for_task_on(
        &crate::storage::database::connect(app)?,
        &task.mod_id,
        &task.task_id,
    )?
    .context
    .reference_sets
    .into_iter()
    .flat_map(|s| s.files.into_iter().map(|f| f.sha256))
    .collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocks_actual_credentials_without_disclosing_them() {
        let secret = "fixture-secret-only".to_string();
        let found = check_bytes(
            "Example/config.txt",
            secret.as_bytes(),
            &[secret.clone()],
            &Default::default(),
        );
        assert!(found
            .iter()
            .any(|f| f.code == "known_secret" && f.level == "block"));
        assert!(!serde_json::to_string(&found).unwrap().contains(&secret));
    }
    #[test]
    fn distinguishes_reference_copy_from_legal_mod() {
        use sha2::{Digest, Sha256};
        let bytes = b"fixture reference";
        let hashes = BTreeSet::from([format!("{:x}", Sha256::digest(bytes))]);
        assert!(check_bytes("renamed.dll", bytes, &[], &hashes)
            .iter()
            .any(|f| f.code == "reference_copy"));
        assert!(check_bytes("Example.dll", b"my own binary", &[], &hashes).is_empty());
    }
    #[test]
    fn blocks_env_and_private_key() {
        assert!(check_bytes("mod/.env", b"x", &[], &Default::default())
            .iter()
            .any(|f| f.code == "env_file"));
        assert!(check_bytes(
            "mod/key.txt",
            b"-----BEGIN RSA PRIVATE KEY-----",
            &[],
            &Default::default()
        )
        .iter()
        .any(|f| f.code == "private_key"));
    }

    #[test]
    fn scans_secrets_across_chunks_and_rejects_unscannable_large_text() {
        use std::io::{Seek, SeekFrom, Write};
        let dir = tempfile::tempdir().unwrap();
        let secret = "fixture-key-crosses-chunk".to_string();
        let mut bytes = vec![b'a'; 65530];
        bytes.extend_from_slice(secret.as_bytes());
        std::fs::write(dir.path().join("config.txt"), bytes).unwrap();
        let report = check_files(
            dir.path(),
            &[("config.txt".into(), "mod/config.txt".into())],
            &[secret.clone()],
            &Default::default(),
        )
        .unwrap();
        assert!(report.findings.iter().any(|f| f.code == "known_secret"));
        assert!(!serde_json::to_string(&report).unwrap().contains(&secret));
        let mut file = std::fs::File::create(dir.path().join("large.txt")).unwrap();
        file.write_all(&vec![b'a'; 65536]).unwrap();
        file.seek(SeekFrom::Start(64 * 1024 * 1024)).unwrap();
        file.write_all(b"a").unwrap();
        assert!(check_files(
            dir.path(),
            &[("large.txt".into(), "mod/large.txt".into())],
            &[],
            &Default::default()
        )
        .is_err());
    }
}
