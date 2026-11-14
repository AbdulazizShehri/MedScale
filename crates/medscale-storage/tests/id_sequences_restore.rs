//! Id sequences survive backup/restore (regression found by the Spec 092
//! whole-platform campaign): after a restore, new ids must never collide
//! with restored rows.

use std::fs;
use std::path::{Path, PathBuf};

use medscale_contracts::ingest::BackupManifest;
use medscale_contracts::objects::DigestSha256;
use medscale_storage::{SyntheticVault, backup_vault, restore_vault};

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("medscale-idseq-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn tamper_backup(dest: &Path, edit: impl FnOnce(&mut serde_json::Value)) {
    let snapshot_path = dest.join("metadata.snapshot");
    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&fs::read(&snapshot_path).unwrap()).unwrap();
    edit(&mut snapshot);
    let bytes = serde_json::to_vec(&snapshot).unwrap();
    fs::write(&snapshot_path, &bytes).unwrap();
    let manifest_path = dest.join("manifest.json");
    let mut manifest: BackupManifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.metadata_snapshot_digest = DigestSha256::of(&bytes);
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
}

fn backed_up(root: &Path) -> PathBuf {
    let vault = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    for _ in 0..3 {
        vault.meta.alloc_compute_id("compute-job").unwrap();
    }
    vault.meta.alloc_audio_id("audio-source").unwrap();
    vault
        .meta
        .alloc_data_fabric_id("snapshot_id_seq", "snap")
        .unwrap();
    let dest = root.join("backup");
    backup_vault(&vault, &dest).unwrap();
    dest
}

#[test]
fn restored_vaults_continue_every_id_sequence() {
    let root = temp_root("continue");
    let dest = backed_up(&root);
    restore_vault(&dest, &root.join("restored")).unwrap();
    let restored = SyntheticVault::open("vault-1", &root.join("restored")).unwrap();
    assert_eq!(
        restored
            .meta
            .alloc_compute_id("compute-job")
            .unwrap()
            .as_str(),
        "compute-job-4"
    );
    assert_eq!(
        restored
            .meta
            .alloc_audio_id("audio-source")
            .unwrap()
            .as_str(),
        "audio-source-2"
    );
    let original = SyntheticVault::open("vault-1", &root.join("vault")).unwrap();
    let after: std::collections::BTreeMap<String, u64> = restored
        .meta
        .list_id_sequences()
        .unwrap()
        .into_iter()
        .collect();
    for (key, value) in original.meta.list_id_sequences().unwrap() {
        assert!(
            after.get(&key).is_some_and(|v| *v >= value),
            "{key} restored at least to {value}"
        );
    }
}

#[test]
fn malformed_sequences_are_refused_and_old_backups_still_restore() {
    let root = temp_root("tamper");
    let dest = backed_up(&root);
    tamper_backup(&dest, |s| {
        s["id_sequences"] = serde_json::json!({"compute_seq_compute-job": "three"});
    });
    assert!(restore_vault(&dest, &root.join("restored")).is_err());

    let root = temp_root("next-seq");
    let dest = backed_up(&root);
    tamper_backup(&dest, |s| {
        s["id_sequences"] = serde_json::json!({"next_seq": 1});
    });
    assert!(restore_vault(&dest, &root.join("restored")).is_err());

    // A backup written before the fix has no sequences: it still restores
    // (new ids of an already-used kind may then collide and fail closed).
    let root = temp_root("old");
    let dest = backed_up(&root);
    tamper_backup(&dest, |s| {
        s.as_object_mut().unwrap().remove("id_sequences");
    });
    restore_vault(&dest, &root.join("restored")).unwrap();
}
