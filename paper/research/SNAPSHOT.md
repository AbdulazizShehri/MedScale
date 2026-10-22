# Research snapshot

## Frozen scientific base

- Repository: `TheHalfMoon/MedScale`
- Commit: `1e2b7d94e970256b38bda15fa91f62bc397e825a`
- Merge: PR #170, `Close Spec 094 canonically and record the final completion audit`
- Snapshot tree: `aa15dc0881307e9b9ad34db6cccfb2f12451a349`
- Final post-main CI run: `36480424698` — successful on the frozen main revision.

The paper branch is intentionally separate from product/UI work. Later UI or cosmetic changes do not alter the scientific base.

## Frozen honesty values

```text
REPOSITORY_IMPLEMENTATION_COMPLETE = true
RESEARCH_OS_PROGRAM_COMPLETE       = true
RELEASE_READY                      = false
PRIVATE_DATA_READY                 = false
PLATFORM_QUALIFIED                 = false
CLINICAL_VALIDATION_STATUS         = NOT_PERFORMED
REGULATORY_STATUS                  = NOT_PERFORMED
```

## Snapshot-change rule

Changing the paper snapshot requires an explicit research decision recorded in this file with:

1. old and new commit SHAs;
2. scientific reason;
3. affected experiments/claims;
4. rerun requirements;
5. whether prior raw results remain valid.

No silent snapshot drift is permitted.
