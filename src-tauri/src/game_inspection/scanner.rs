use super::models::*;
use crate::storage::paths;
use std::path::Path;
pub fn validate_rules(rule: &InspectionRule) -> Result<(), String> {
    if rule.schema_version != 1 || rule.profiles.is_empty() || rule.profiles.len() > 2 {
        return Err("调查规则版本或数量无效".into());
    }
    let mut kinds = std::collections::BTreeSet::new();
    for profile in &rule.profiles {
        if !matches!(profile.kind.as_str(), "install" | "references")
            || !kinds.insert(&profile.kind)
            || profile.paths.is_empty()
            || profile.paths.len() > 64
        {
            return Err("调查类型或路径数量无效".into());
        }
        for item in &profile.paths {
            let components = item.relative.split('/').collect::<Vec<_>>();
            if components.len() > 4
                || components
                    .iter()
                    .any(|s| s.is_empty() || *s == "." || *s == "..")
                || item.relative.contains(['\\', ':', '?', '[', ']'])
                || components[..components.len() - 1]
                    .iter()
                    .any(|s| s.contains('*'))
                || components
                    .last()
                    .is_some_and(|s| s.matches('*').count() > 1)
                || !matches!(
                    item.fact.as_str(),
                    "engineMarker" | "referenceAssembly" | "loaderMarker"
                )
            {
                return Err("调查仅允许深度不超过 4 的相对路径和文件名末段单个 *".into());
            }
        }
    }
    Ok(())
}
fn matches(pattern: &str, name: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let name = name.to_lowercase();
    if let Some((a, b)) = pattern.split_once('*') {
        name.len() >= a.len() + b.len() && name.starts_with(a) && name.ends_with(b)
    } else {
        pattern == name
    }
}
pub fn scan_directory(root: &Path, profile: &Profile) -> Result<InspectionReport, String> {
    validate_rules(&InspectionRule {
        schema_version: 1,
        profiles: vec![profile.clone()],
    })?;
    if !root.is_absolute() || !root.is_dir() {
        return Err("请选择有效目录".into());
    }
    for parent in root.ancestors() {
        paths::reject_redirect(parent)?;
    }
    let mut report = InspectionReport {
        report_id: String::new(),
        mod_id: String::new(),
        game_id: String::new(),
        kind: profile.kind.clone(),
        inspected_at: String::new(),
        findings: vec![],
        unknowns: vec![
            "gameVersion".into(),
            "apiCompleteness".into(),
            "runtimeBehavior".into(),
        ],
        truncated: false,
        errors: vec![],
    };
    let mut enumerated = 0;
    let mut hashed = 0u64;
    let mut seen = std::collections::BTreeSet::new();
    for rule in &profile.paths {
        let relative = Path::new(&rule.relative);
        let parent = relative.parent().unwrap_or(Path::new(""));
        let name = relative
            .file_name()
            .ok_or("规则文件名无效")?
            .to_string_lossy();
        let mut directory = root.to_path_buf();
        let mut valid = true;
        for component in parent.components() {
            directory.push(component);
            if !directory.exists() {
                valid = false;
                break;
            }
            if paths::reject_redirect(&directory).is_err() {
                report.errors.push(ScanError {
                    relative_path: rule.relative.clone(),
                    code: "format".into(),
                });
                valid = false;
                break;
            }
        }
        if !valid || !directory.is_dir() {
            report.unknowns.push(format!("missing:{}", rule.relative));
            continue;
        }
        let mut candidates = vec![];
        if name.contains('*') {
            match std::fs::read_dir(&directory) {
                Ok(entries) => {
                    for entry in entries {
                        enumerated += 1;
                        if enumerated > 5000 {
                            report.truncated = true;
                            report.errors.push(ScanError {
                                relative_path: rule.relative.clone(),
                                code: "limit".into(),
                            });
                            break;
                        }
                        match entry {
                            Ok(e) => {
                                if matches(&name, &e.file_name().to_string_lossy()) {
                                    candidates.push(e.path());
                                }
                            }
                            Err(_) => report.errors.push(ScanError {
                                relative_path: rule.relative.clone(),
                                code: "permission".into(),
                            }),
                        }
                    }
                }
                Err(_) => report.errors.push(ScanError {
                    relative_path: rule.relative.clone(),
                    code: "permission".into(),
                }),
            }
        } else {
            candidates.push(directory.join(name.as_ref()));
        }
        let mut matched = false;
        for path in candidates {
            let rel = path
                .strip_prefix(root)
                .map_err(|_| "调查路径越界")?
                .to_string_lossy()
                .replace('\\', "/");
            let meta = match std::fs::symlink_metadata(&path) {
                Ok(v) => v,
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        report.errors.push(ScanError {
                            relative_path: rel.clone(),
                            code: "permission".into(),
                        });
                    }
                    continue;
                }
            };
            if paths::reject_redirect(&path).is_err()
                || rule.fact != "loaderMarker" && !meta.is_file()
            {
                report.errors.push(ScanError {
                    relative_path: rel,
                    code: "format".into(),
                });
                continue;
            }
            matched = true;
            if !seen.insert(rel.to_lowercase()) {
                continue;
            }
            let hash = if rule.fact == "referenceAssembly" {
                if meta.len() > 64 * 1024 * 1024
                    || hashed.saturating_add(meta.len()) > 256 * 1024 * 1024
                {
                    report.truncated = true;
                    report.errors.push(ScanError {
                        relative_path: rel,
                        code: "limit".into(),
                    });
                    continue;
                }
                hashed += meta.len();
                match crate::changes::file_hash(&path) {
                    Ok(hash) => Some(hash),
                    Err(_) => {
                        report.errors.push(ScanError {
                            relative_path: rel,
                            code: "permission".into(),
                        });
                        continue;
                    }
                }
            } else {
                None
            };
            report.findings.push(Finding {
                relative_path: rel,
                fact: rule.fact.clone(),
                sha256: hash,
            });
        }
        if !matched {
            report.unknowns.push(format!("missing:{}", rule.relative));
        }
        if enumerated > 5000 {
            break;
        }
    }
    report
        .findings
        .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    report.unknowns.sort();
    report.unknowns.dedup();
    Ok(report)
}
