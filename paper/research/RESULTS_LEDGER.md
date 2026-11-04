# Research results ledger

This ledger records successful **and failed** paper-evaluation attempts. A failed or methodologically non-qualifying run is retained rather than rewritten into a successful result.

## Pilot P0 — GitHub Actions run 36640758384

- Research branch head associated with the PR: `3484d242a1fc505e1f10372914b0e0636ce30702`
- Scientific base: `1e2b7d94e970256b38bda15fa91f62bc397e825a`
- Workflow: `paper-evaluation`
- Overall result: `FAILED / NON-QUALIFYING PILOT`
- Qualification warning: the initial PR workflow used the default `actions/checkout` behavior and therefore executed GitHub's synthetic PR merge revision (`cba417220d9c160e68f22a07515700bc405403a2`), not the research branch head directly. Because the paper requires exact-revision binding, P0 is retained as pilot evidence only.

### Observed outcomes

| Experiment | Ubuntu | macOS | Windows | Interpretation |
|---|---|---|---|---|
| A0 contract provenance controls | PASS | PASS | PASS | Seven tests completed successfully on each hosted OS in P0, but the run is not promoted to final paper evidence because checkout was the synthetic merge ref. |
| A1 Core provenance binding | PASS | PASS | INFRASTRUCTURE FAILURE BEFORE TEST EXECUTION | Six tests completed successfully on Ubuntu and macOS. Windows failed during native dependency build because the workflow forced Git Bash, which selected an incompatible Perl path for the vendored OpenSSL build. This is not counted as an A1 assertion failure. |
| B0 authority/effect conformance | 4/6, HARNESS FAILURE | 4/6, HARNESS FAILURE | NOT EXECUTED | The two failures were caused by the research harness omitting `Capability::ListOutbox` in sessions that subsequently attempted `ListOutbox`. The other four B0 cases passed on Ubuntu/macOS. The missing grant is a harness setup defect, not evidence of a MedScale product defect. |

### Method defects discovered by P0

1. **Synthetic merge checkout:** default PR checkout did not satisfy the paper's exact-head requirement.
2. **Windows shell contamination:** forcing `shell: bash` bypassed the canonical Windows toolchain environment and caused a Perl/OpenSSL infrastructure failure.
3. **B0 harness capability omission:** two observation steps lacked the observation capability needed by the test itself.
4. **Evidence loss on failure:** checksum and artifact-upload steps were skipped after an experiment step failed.

### Repairs applied after P0

- explicitly checkout `github.event.pull_request.head.sha` for pull-request research runs;
- use a cross-platform Python evidence runner instead of forcing Bash for Cargo experiment steps;
- grant `ListOutbox` only to the B0 cases that need to inspect the outbox, without granting the promotion capability under test;
- run each experiment, hashing, and artifact upload with failure-preserving workflow semantics so partial raw evidence survives future failures;
- record both actual checkout SHA and GitHub event SHA in the environment manifest.

### Claim boundary

P0 may be cited in the research process as evidence that the evaluation harness itself was falsifiable and repaired. It must **not** be used as the manuscript's final cross-platform conformance result.

## Attempt P1 — GitHub Actions run 36641570242

- Intended exact research head: `411390ddf21265ebee820f490c6b76fcee490966`
- Workflow: `paper-evaluation`
- Overall result: `NON-QUALIFYING INFRASTRUCTURE ATTEMPT`
- Ubuntu and macOS completed A0, A1, and B0 successfully.
- Windows completed A0, but the Python evidence wrapper raised `UnicodeEncodeError` while echoing Cargo output containing Unicode diagnostic characters to a CP1252 console. The wrapper failure prevented Windows A1/B0 from becoming qualifying test results; it is not counted as a MedScale assertion failure.
- General CI also identified a `rustfmt` delta in the A1 research harness.
- Repairs: format the A1 harness and force deterministic UTF-8 output/decoding in `paper/artifact/scripts/evidence_io.py`.

P1 is retained to document evaluation-infrastructure falsifiability and is not used as final cross-platform evidence.

## Qualifying run Q1 — GitHub Actions run 36643514848

- Experimental revision / exact checkout: `6a04266ae59bfb75f24d6634becd3bc4d32576c4`
- Scientific base: `1e2b7d94e970256b38bda15fa91f62bc397e825a`
- Workflow: `paper-evaluation`
- Overall result: `SUCCESS`
- Hosted runners: Linux X64, macOS ARM64, Windows X64.
- The environment manifests intentionally record both `checkout_sha` and GitHub's pull-request event SHA. The qualifying revision is the `checkout_sha`; the event SHA (`b00cfce18ff6ed45c023ce1a80813b5c434ded2d`) is the synthetic PR merge trigger and is not treated as the experimental revision.

### Cross-platform outcomes

| Experiment | Unique cases | Linux | macOS | Windows | Total platform executions |
|---|---:|---:|---:|---:|---:|
| A0 contract provenance controls | 7 | 7/7 PASS | 7/7 PASS | 7/7 PASS | 21/21 PASS |
| A1 Core provenance/authority binding | 6 | 6/6 PASS | 6/6 PASS | 6/6 PASS | 18/18 PASS |
| B0 authority/effect conformance | 6 | 6/6 PASS | 6/6 PASS | 6/6 PASS | 18/18 PASS |
| **Total** | **19** | **19/19 PASS** | **19/19 PASS** | **19/19 PASS** | **57/57 PASS** |

The 57 figure is repeated platform execution of 19 unique cases, not 57 independent unique test designs.

### What Q1 exercises

- A0: source/content identity separation, digest mutation detection, conservative unknown evidence semantics, and invalid/nonexistent/retracted support controls.
- A1: strict session requirement, capability denial, exact source/transform/version/output digest binding, cross-scope binding/read refusal, and nonexistent-source refusal.
- B0: proposal-versus-promotion authority separation, reviewer identity retention, payload-bound external action intent, refusal of illegal effect transitions, reconciliation before retry from unknown external effect, and the explicit NPHIES external gate requirement.

### Retained artifact identity

| OS | Artifact ID | Artifact archive SHA-256 | Environment/runtime evidence |
|---|---:|---|---|
| Linux | `11067444321` | `8575a05792f1b4941d4fa65940da0fb02d38f6328fc0a7325400be2abfdc5112` | checkout `6a04266...`; rustc/cargo 1.97.1; Python 3.12.3 |
| macOS | `11067474345` | `64db3cccf604e241632b2ff6894ab67200621466da7036ac1fef1ceb170a8e82` | checkout `6a04266...`; rustc/cargo 1.97.1; Python 3.14.7 |
| Windows | `11068681168` | `2a2609a69b28bf1d41c115a043944129da11354b668e8bf80060caafda153c04` | checkout `6a04266...`; rustc/cargo 1.97.1; Python 3.12.10 |

Raw-log checksums retained inside each artifact:

- Linux: A0 `4ef88bded708ba3d4396d0e9d90d013d1133d1d7e8d738c39d64d70b9939e414`; A1 `6cec8dad427b7f4075f00652e24bf7df86920a16223759a2a9b776535a525204`; B0 `2939d89aeab84e66cfb3e67232493d3f71b20516350610a430c890ec2ffe1e3a`.
- macOS: A0 `b54ae3694a97583d625227678e6b25bf6470ea5fc821c1dd235fdb3a9527a7d0`; A1 `944a21dfa3d44890684b6e2fe1fcb8f8adb01f5b0daf285e13aba1904516f184`; B0 `b666c3b8b5eb38bf36d3ffb46a17ac7f7ba1dd48c9b54e41611ff3d50555ce33`.
- Windows: A0 `620cce25769d8df5bc3872bd8505df43a89c5f87c248e31a760194eb1bcfbef0`; A1 `b4222eab315a04680b69dfb3dddbbf10414d15cced3ef751649f5783777ecca2`; B0 `33fc67a7d0ba03c994bcab393b1f887cd91c5d979d7fee41384ac080d7b572c5`.

### Interpretation boundary

Q1 establishes cross-platform conformance for the 19 specified synthetic cases at experimental revision `6a04266...`. It does **not** establish universal security, clinical safety, clinical validity, regulatory fitness, production-PHI readiness, platform qualification, or the empirical probability of attacks or clinical failures. The mutation/refusal cases are a deliberately constructed conformance suite.

## Pre-existing integrated recovery evidence C0 — Spec 092

C0 is not a new paper-only experiment. It is canonical integrated evidence at the frozen scientific base and is treated separately from Q1.

- Spec 092 classified 35 evidence areas using conservative evidence labels.
- Its integrated `whole_platform_092` path exercised one synthetic vault across qualified planes, then performed backup, restore into a fresh directory, consistency verification, restart/reopen, and post-restart semantic checks.
- Exact-head CI run `36360503149` completed 6/6 required jobs; recorded test counts were Ubuntu `967/0/1`, Windows `963/0/1`, and macOS `965/0/1` (passed/failed/ignored as recorded in closure evidence).
- Qualification discovered two defects before closure: restore ID-sequence loss causing post-restore collision, and a nondeterministic privacy-matcher false positive. Both were repaired with regression evidence; failed runs were retained.
- The canonical closure keeps `PLATFORM_QUALIFIED=false`, `CLINICAL_VALIDATION_STATUS=NOT_PERFORMED`, and `REGULATORY_STATUS=NOT_PERFORMED`.

C0 supports an implemented recovery/verification claim at the frozen snapshot but is not converted into a clinical, regulatory, or release-readiness claim.

## Qualifying run Q2 — C1 repeated fresh-vault recovery

- Experimental revision / exact checkout: `e1cf8935f3920c95cf7229221619fbfb24fd0fa4`
- Scientific base: `1e2b7d94e970256b38bda15fa91f62bc397e825a`
- Workflow: `paper-evaluation`
- Run: `36645800815`
- Overall result: `SUCCESS`
- Preregistered protocol: `paper/research/C1_PROTOCOL.md`
- Hosted runners: Linux X64, macOS ARM64, Windows X64.
- GitHub event SHA recorded in all three manifests: `a9adcd50c48f3d686c75766a7d581bfcde5afe29`; the experimental revision is the explicit checkout SHA above, not the synthetic event revision.

### C1 primary endpoint

The production Spec 092 recovery path `every_plane_survives_backup_restore_and_restart_through_core` was invoked in three separate Cargo test processes per hosted operating system. Each invocation exercises the integrated synthetic vault, backup, restoration into a fresh directory, consistency verification, Core reopen/restart, and post-restart assertions.

| OS | Repeat 1 | Repeat 2 | Repeat 3 | Result |
|---|---:|---:|---:|---:|
| Linux | PASS | PASS | PASS | 3/3 |
| macOS | PASS | PASS | PASS | 3/3 |
| Windows | PASS | PASS | PASS | 3/3 |
| **Total** |  |  |  | **9/9 PASS** |

The A0, A1, and B0 steps also completed successfully in Q2, but Q2's preregistered new endpoint is the nine C1 recovery repetitions; Q1 remains the primary evidence record for the 19-case A/B campaign.

### Retained artifact identity

| OS | Artifact ID | GitHub artifact SHA-256 | Runtime evidence |
|---|---:|---|---|
| Linux | `11068433415` | `402bcba68d6d3a54190e14f86a62f82bd1068d4cfbb6d02e9b80371d08c5e4dd` | checkout `e1cf8935...`; rustc/cargo 1.97.1; Python 3.12.3 |
| macOS | `11067938615` | `553d72f82e7c4a8c6eecc40a7f639719e63b943319a5aa23172c5d5aff3503ac` | checkout `e1cf8935...`; rustc/cargo 1.97.1; Python 3.14.7 |
| Windows | `11068558567` | `c68c69dd98522306faba2e590c239b0447da6b98c802e296adaa292c37baf7bf` | checkout `e1cf8935...`; rustc/cargo 1.97.1; Python 3.12.10 |

C1 raw-log SHA-256 checksums:

- Linux: R1 `cad0eb8693cd84ec15446d9be412e2266e7a43017c94776def4e26a80fa49ea3`; R2 `1abc77fa53fe02ab7890d24a51b03ba862e2628aa05314a7797baa04a02aec17`; R3 `a140452d10c7af367679785d05fc36c9e7f8b81181c287925303ad0217df3524`.
- macOS: R1 `c4296680dab1bc7d7ba27213773060021e2494a35789de00f4427599192e9815`; R2 `6285587d782af8f3770bfe19e00c29aeb2dc3ec0d19a27af7d6c94a830f5b4a1`; R3 `2a26326e77d89c4f0bc40af0e9691aa7077ee762555cec745bb88f1a610160ef`.
- Windows: R1 `27319cf2139a8449c5e77c4856c3194b9c55d65d89bb4463a55d75cf8b416ebd`; R2 `dc9b4722bd602fb7a6c8c61db9c5141002363d806bd5c60b8e94bfa803a48cec`; R3 `a5bd91ee8d36cef79f9630f2e70941429442606a8dfe412c3a355eec78eae8c9`.

### Interpretation boundary

Q2 supports the bounded statement preregistered before result inspection: the specified integrated synthetic backup/restore/restart path satisfied its production assertions in three separate process-level repetitions on each of the three hosted operating systems at exact revision `e1cf8935...`. It does **not** establish durability under arbitrary crashes or storage corruption, production disaster-recovery fitness, universal determinism, clinical safety or validity, regulatory compliance, real-PHI readiness, platform qualification, or release readiness.
