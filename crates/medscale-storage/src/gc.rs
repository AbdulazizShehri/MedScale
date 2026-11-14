//! Blob GC mark / tombstone / sweep.

use medscale_contracts::objects::DigestSha256;

use crate::blob::FsBlobStore;
use crate::sqlite_meta::SqliteMetaStore;

/// GC statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GcStats {
    pub marked: u64,
    pub tombstoned: u64,
    pub swept: u64,
}

/// Mark all digests referenced by visible metadata, tombstone unmarked live blobs, sweep tombstones.
pub fn run_gc(meta: &SqliteMetaStore, blobs: &FsBlobStore, epoch: u64) -> Result<GcStats, String> {
    meta.clear_gc_marks().map_err(|e| e.to_string())?;
    let mut marked = 0_u64;
    for source in meta.list_sources().map_err(|e| e.to_string())? {
        if source.visible {
            meta.mark_digest(&source.digest, epoch)
                .map_err(|e| e.to_string())?;
            marked += 1;
        }
    }
    let mut tombstoned = 0_u64;
    for digest in blobs.list_live_digests().map_err(|e| e.to_string())? {
        if !meta.is_marked(&digest).map_err(|e| e.to_string())? {
            blobs.tombstone(&digest).map_err(|e| e.to_string())?;
            tombstoned += 1;
        }
    }
    let swept = blobs.sweep_tombstones().map_err(|e| e.to_string())?;
    let _ = DigestSha256::of(b"");
    Ok(GcStats {
        marked,
        tombstoned,
        swept,
    })
}
