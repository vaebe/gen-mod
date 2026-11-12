use super::io::{copy_to_staging, move_no_replace};

#[test]
fn import_preserves_arbitrary_bytes_and_never_overwrites() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("original.bin");
    let staging = root.path().join("staging.tmp");
    let target = root.path().join("target.bin");
    let bytes = [0, 255, 7, 0, 128, 13, 10];
    std::fs::write(&original, bytes).unwrap();
    assert_eq!(copy_to_staging(&original, &staging).unwrap().len(), 64);
    move_no_replace(&staging, &target).unwrap();
    assert_eq!(std::fs::read(&original).unwrap(), bytes);
    assert_eq!(std::fs::read(&target).unwrap(), bytes);
    assert!(!staging.exists());
    std::fs::write(&original, b"changed").unwrap();
    copy_to_staging(&original, &staging).unwrap();
    assert!(move_no_replace(&staging, &target).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), bytes);
    assert_eq!(std::fs::read(&staging).unwrap(), b"changed");
}

#[test]
fn staging_cannot_overwrite_a_previous_file() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("input");
    let staging = root.path().join("staged");
    std::fs::write(&original, b"new").unwrap();
    std::fs::write(&staging, b"keep").unwrap();
    assert!(copy_to_staging(&original, &staging).is_err());
    assert_eq!(std::fs::read(staging).unwrap(), b"keep");
}

#[test]
fn directory_input_never_creates_staging() {
    let root = tempfile::tempdir().unwrap();
    let staging = root.path().join("staged");
    assert!(copy_to_staging(root.path(), &staging).is_err());
    assert!(!staging.exists());
}

#[test]
fn paths_reject_escape_reserved_names_and_internal_directories() {
    let root = tempfile::tempdir().unwrap();
    for path in ["../outside", "image.png:stream", "CON", "folder/NUL.txt", ".delivery/file", ".git/config", "bin/file"] {
        assert!(super::checked_path(root.path(), path, &["bin".into()]).is_err(), "{path}");
    }
    assert!(super::target_path(root.path(), "", "nested/file", &[]).is_err());
    assert!(super::target_path(root.path(), "", "input.bin", &[]).is_ok());
}

fn fixture_operation(root: &std::path::Path) -> (String, super::SourceOperation) {
    std::fs::create_dir(root.join("source")).unwrap();
    let operation_id = uuid::Uuid::new_v4().to_string();
    let item_id = uuid::Uuid::new_v4().to_string();
    let operation = super::SourceOperation {
        directory: false,
        operation_id: operation_id.clone(), source_relative_path: None, target_relative_path: "input.bin".into(),
        temporary_relative_path: Some(format!(".source-imports/{operation_id}/{item_id}.tmp")),
        before_hash: None, result_hash: None, phase: "allocated".into(),
    };
    (item_id, operation)
}

#[test]
fn interrupted_import_never_replays_external_input_or_exposes_partial_source() {
    let root = tempfile::tempdir().unwrap();
    let (id, mut operation) = fixture_operation(root.path());
    assert_eq!(super::recover_operation(root.path(), &id, "source_import", &operation, &[]).unwrap(), "interrupted");
    super::create_staging_parents(root.path(), &operation.operation_id).unwrap();
    let staged = super::staging_path(root.path(), &id, &operation).unwrap();
    std::fs::write(&staged, b"partial").unwrap();
    assert_eq!(super::recover_operation(root.path(), &id, "source_import", &operation, &[]).unwrap(), "requires_review");
    assert!(staged.exists());
    assert_eq!(std::fs::read_dir(root.path().join("source")).unwrap().count(), 0);
    operation.phase = "ready".into();
    operation.result_hash = Some(crate::changes::file_hash(&staged).unwrap());
    assert_eq!(super::recover_operation(root.path(), &id, "source_import", &operation, &[]).unwrap(), "interrupted");
    assert!(!staged.exists());
}

#[test]
fn committed_import_is_recognized_but_changed_target_is_preserved() {
    let root = tempfile::tempdir().unwrap();
    let (id, mut operation) = fixture_operation(root.path());
    let target = root.path().join("source/input.bin");
    std::fs::write(&target, b"saved").unwrap();
    operation.phase = "ready".into();
    operation.result_hash = Some(crate::changes::file_hash(&target).unwrap());
    assert_eq!(super::recover_operation(root.path(), &id, "source_import", &operation, &[]).unwrap(), "reconciled");
    std::fs::write(&target, b"later user edit").unwrap();
    assert_eq!(super::recover_operation(root.path(), &id, "source_import", &operation, &[]).unwrap(), "requires_review");
    assert_eq!(std::fs::read(&target).unwrap(), b"later user edit");
    operation.temporary_relative_path = Some("../outside".into());
    assert!(super::recover_operation(root.path(), &id, "source_import", &operation, &[]).is_err());
}

#[test]
fn move_recovery_preserves_conflicting_paths() {
    let root = tempfile::tempdir().unwrap();
    let (id, mut operation) = fixture_operation(root.path());
    let original = root.path().join("source/old.bin");
    let target = root.path().join("source/input.bin");
    std::fs::write(&original, b"saved").unwrap();
    operation.source_relative_path = Some("old.bin".into());
    operation.temporary_relative_path = None;
    operation.result_hash = Some(crate::changes::file_hash(&original).unwrap());
    operation.phase = "ready".into();
    assert_eq!(super::recover_operation(root.path(), &id, "source_move", &operation, &[]).unwrap(), "interrupted");
    move_no_replace(&original, &target).unwrap();
    assert_eq!(super::recover_operation(root.path(), &id, "source_move", &operation, &[]).unwrap(), "reconciled");
    std::fs::write(&original, b"new user file").unwrap();
    assert_eq!(super::recover_operation(root.path(), &id, "source_move", &operation, &[]).unwrap(), "requires_review");
    assert_eq!(std::fs::read(original).unwrap(), b"new user file");
}

#[test]
fn directory_move_preserves_contents_and_recovers_with_empty_directories() {
    let root = tempfile::tempdir().unwrap();
    let (id, mut operation) = fixture_operation(root.path());
    let source = root.path().join("source");
    let original = source.join("old");
    let target = source.join("new");
    std::fs::create_dir_all(original.join("empty")).unwrap();
    std::fs::write(original.join("file.bin"), [0, 255, 10]).unwrap();
    operation.directory = true;
    operation.source_relative_path = Some("old".into());
    operation.target_relative_path = "new".into();
    operation.temporary_relative_path = None;
    operation.phase = "ready".into();
    operation.result_hash = Some(super::directory_fingerprint(&source, "old", Some("new"), &[]).unwrap());
    assert_eq!(super::recover_operation(root.path(), &id, "source_move", &operation, &[]).unwrap(), "interrupted");
    super::io::move_entry_no_replace(&original, &target).unwrap();
    assert_eq!(std::fs::read(target.join("file.bin")).unwrap(), [0, 255, 10]);
    assert_eq!(super::recover_operation(root.path(), &id, "source_move", &operation, &[]).unwrap(), "reconciled");
    std::fs::remove_dir(target.join("empty")).unwrap();
    assert_eq!(super::recover_operation(root.path(), &id, "source_move", &operation, &[]).unwrap(), "requires_review");
}

#[test]
fn directory_move_rejects_existing_destination_and_reserved_descendants() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("old/cache")).unwrap();
    std::fs::create_dir(root.path().join("new")).unwrap();
    assert!(super::io::move_entry_no_replace(&root.path().join("old"), &root.path().join("new")).is_err());
    assert!(root.path().join("old/cache").is_dir());
    assert!(super::directory_fingerprint(root.path(), "old", Some("new"), &["old/cache".into()]).is_err());
    assert!(super::directory_fingerprint(root.path(), "old", Some("new"), &["new/cache".into()]).is_err());
}
