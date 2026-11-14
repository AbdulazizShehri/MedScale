# Migration and Recovery — Spec 083

Additive: storage schema v11 -> v12. `CURRENT_META_SCHEMA_VERSION` becomes 12.

```text
knowledge_manifests (insert-once; unique (project_id, version))
knowledge_chunks    (insert-once; primary key (manifest_id, seq))
knowledge_receipts  (insert-once)
knowledge_canvases  (insert-once; primary key (canvas_id, revision))
```

An index version (its manifest and every chunk) commits in one transaction
and must be the Project's next version. Reading a version's chunks re-checks
the count, sequence, per-chunk invariants, source membership and digest.
A Canvas revision must follow the latest revision of its Canvas and never
change Project.

Cross-row invariants (`verify_knowledge_consistency`, run on restore):
- every row names a Project in its own realm and scope;
- index versions are contiguous 1..n per Project;
- every receipt that ran names a manifest of its own Project with the
  recorded chunk digest;
- canvas revisions are contiguous 1..n and never change Project.

A v11 backup restores with empty knowledge tables. A crash during the v12
migration fails closed (`MigrationIncomplete(12)`). Tests of Specs 078-082
that rewind a vault also drop the v12 tables.

The knowledge tables have no foreign keys into Spec 075-082 tables and no
delete path, so nothing here can cascade into a source. Archiving a data
source (Spec 075) tombstones its indexed spans on the next read; a rebuild
drops them.
