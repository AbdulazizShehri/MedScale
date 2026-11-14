# Exact-Head Qualification — Spec 080

```text
BASE_SHA   = 561f97feabe6beb7f079e2af679a4119b50368ce
BRANCH     = spec/080-governed-browse (PR #139)
CODE_HEAD  = 5daec975ed76b78381d9eb6b976b077933ae90cd
CI_RUN     = 35861492981 (pull_request), 6/6 success, bound to CODE_HEAD
TOOLCHAIN  = rustc/cargo 1.97.1
```

| Job | Conclusion |
|---|---|
| supply-chain policy present | success |
| cargo-deny | success |
| perf delivery-plan scale (windows) | success |
| rust (ubuntu-latest) | success |
| rust (macos-latest) | success |
| rust (windows-latest) | success |

Each `rust (*)` job runs fmt check, dependency direction, Clippy with
`-D warnings`, the full workspace tests, and portable package qualification.
The ubuntu job log (fetched with `gh api .../actions/jobs/<id>/logs`) shows
117 `test result` lines, 743 passed, 0 failed, 0 `FAILED`/`panicked` lines,
and each new or extended Spec 080 test by name with `ok`:
`url_paths_with_normalization_ambiguity_are_refused`,
`allowlist_entries_validate_and_match`,
`validate_url_accepts_only_https_dns_hosts_on_443`,
`forbidden_addresses_cover_private_and_special_ranges`,
`every_redirect_hop_is_re_evaluated`,
`restore_rejects_hand_edited_080_snapshots`,
`browse_commands_run_through_core_across_fresh_sessions`,
`browse_view_models_flow_through_a_real_core_session`.

The commit that adds this evidence creates a new head, which needs its own
required CI run. That run is recorded in `POST_MERGE_VERIFICATION.md` and
`CLOSURE.md`.

Earlier heads on this branch (historical only):

| Head | Run | Result |
|---|---|---|
| 231e0e1 (promotion) | 35819965441 | 6/6 success |
| 8691277 (first green code head) | 35828932513 | 6/6 success; superseded by `5daec97` |
