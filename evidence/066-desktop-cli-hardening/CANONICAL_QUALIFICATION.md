# Canonical Qualification — Spec 066

State: `CLOSED_CANONICAL`

## Exact-head gate
- Final head: `ed5cbbb06e2ae20eb52cde4a7d69fd6253fbf5b3`.
- PR: #110.
- Exact-head run: `34934782208`; all six required jobs passed.
- Merge: `31e7c6c83a67c297939243ec13c4522e6b6e4b4f`; normal protected merge, no bypass.

Selected exact-head artifact digests:
- portable Windows: `sha256:30f017eeaf06015ba410126319a6763f91d27ef42eeb75b8e4e0d4eb49176f00`
- portable Ubuntu: `sha256:17fc183160d25d88f86eba8c71a20b0deb1ab62dd8cdd428171a1b9307d0deeb`
- portable macOS: `sha256:530bfd303017ad09c67dc691028f50ec0544b4766a96689d98f6c0ededdcacd5`
- perf delivery-plan scale: `sha256:42a555f6ca56f3a0a6a65fd8609192e15c8ddf18c565919ce2ad6886eb448445`

## Post-merge main gate
- Main run: `34935549696`; all six required jobs passed on merge `31e7c6c83a67c297939243ec13c4522e6b6e4b4f`.
- portable Windows: `sha256:44fdbbb140b581293705e6720cde5fa0c66bd5bff32b26c03ff93e9e805b95b5`
- portable Ubuntu: `sha256:3373f0e85f4e6a8e6966ae30042be8a6f9b0653e8a3b30167c4a4e0edc832598`
- portable macOS: `sha256:fede65d97130ed7ee7dee857d89fdff10b77e06ab6ffa22ed39d5cba9ef20b89`
- perf delivery-plan scale: `sha256:d50e9851ef0815bf91912ba15b2125258d4bf24bd9225f238171dba8c7ce873f`

OpenCodeReview v1.12.1 delegation supplemented the review for supported YAML/Rust changes; unsupported Slint/docs were reviewed manually. Required CI and repository governance remained authoritative.
