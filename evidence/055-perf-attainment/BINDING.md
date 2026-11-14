# Binding - Spec 055 Perf Attainment Dossier

- source_sha: 8cac4171427a7fea30f99adc88461f050942dbc8
- tree_sha: dd24a6b2884db2fad78ef5fc29c51b936ba1c670
- cargo_lock_sha256: 7D6F1597DB5419C07AEE3A3F91CC7E28EE30A57E011BB98D34FEE6FDDF2D3775
- toolchain_rustc: 1.97.1
- toolchain_cargo: 1.97.1
- target: x86_64-pc-windows-msvc
- build_profile: test/dev (dossier binds CI-scale harness; release-profile runs deferred to qualified hardware)
- os: Windows (dossier host); multi-OS figures: not measured here
- harness_lineage: specs 027/042/045/050 (perf harness + delivery-plan scale + lexical 10k + host measurement path)
- warmups: harness defaults (see scripts/run-host-perf-measurement.ps1)
- run_count: CI-scale runs per workflow; operator host runs recorded in run sidecars when present

