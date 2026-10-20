use super::{models::*, scanner::*};
fn rules() -> InspectionRule {
    serde_json::from_str(include_str!(
        "../../../skills/games/duckov/duckov-mod/references/inspection.json"
    ))
    .unwrap()
}
#[test]
fn markers_do_not_claim_game_version() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "UnityEngine.CoreModule.dll",
        "UnityEngine.IMGUIModule.dll",
        "TeamSoda.Example.dll",
    ] {
        std::fs::write(dir.path().join(name), b"fixture marker only").unwrap();
    }
    let result = scan_directory(dir.path(), &rules().profiles[0]).unwrap();
    assert_eq!(result.findings.len(), 3);
    assert!(result.unknowns.contains(&"gameVersion".into()));
    assert!(!result.truncated);
}
#[test]
fn rejects_traversal_and_directory_wildcards() {
    let mut r = rules();
    r.profiles[0].paths[0].relative = "../outside".into();
    assert!(validate_rules(&r).is_err());
    r.profiles[0].paths[0].relative = "*/file.dll".into();
    assert!(validate_rules(&r).is_err());
}

#[test]
fn excessive_reference_size_is_reported_as_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    let file = std::fs::File::create(dir.path().join("UnityEngine.CoreModule.dll")).unwrap();
    file.set_len(64 * 1024 * 1024 + 1).unwrap();
    let report = scan_directory(dir.path(), &rules().profiles[0]).unwrap();
    assert!(report.truncated);
    assert!(report.errors.iter().any(|e| e.code == "limit"));
    assert!(report.findings.is_empty());
}
