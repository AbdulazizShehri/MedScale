# Reproducibility artifact

This directory will contain the executable artifact for the MedScale paper.

## Scientific snapshot

`1e2b7d94e970256b38bda15fa91f62bc397e825a`

## Required properties

- synthetic data only;
- exact command capture;
- environment/toolchain manifest;
- raw results kept distinct from generated manuscript outputs;
- checksums for raw result bundles;
- deterministic figure/table generation where possible;
- no manually invented performance or accuracy numbers;
- no secrets, PHI, private credentials, or private partner data.

## Planned directories

- `manifests/`: frozen environment and run metadata;
- `fixtures/`: deterministic synthetic cases;
- `scripts/`: experiment and analysis entry points;
- `raw/`: append-only raw machine results;
- `generated/`: tables/figures generated from raw results;
- `checksums/`: integrity manifests.

The experiment harness will be added only after the exact production validation surfaces for each planned mutation have been mapped. This prevents an experiment from accidentally testing a research-only mock while being described as a production invariant.
