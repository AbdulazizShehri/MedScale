# Spec 049 — verify package upgrade/rollback *scaffold* against unsigned dry-run artifacts.
#
# This does NOT install, upgrade, or roll back real packages. It only checks that
# baseline release-manifest + checksum scaffolds are present, unsigned, and
# internally consistent, and that a candidate dry-run snapshot (if present) does
# not claim RELEASE_READY.
#
# Usage:
#   pwsh ./scripts/verify-package-upgrade-rollback.ps1
#   pwsh ./scripts/verify-package-upgrade-rollback.ps1 -CandidateManifest evidence/047-release-dry-run-verifier/release-manifest.dry-run.json
#
# Exit 0 = scaffold checks pass; Exit 1 = honesty/consistency failure.

[CmdletBinding()]
param(
    [string]$BaselineManifest = "evidence/039-release-honesty-packets/release-manifest.scaffold.json",
    [string]$BaselineChecksums = "evidence/027-perf-sbom-release-evidence/package-checksums.json",
    [string]$CandidateManifest = "evidence/047-release-dry-run-verifier/release-manifest.dry-run.json"
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

function Get-Sha256Hex {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant()
}

$failures = @()

$baselineFull = Join-Path $repoRoot $BaselineManifest
$checksumsFull = Join-Path $repoRoot $BaselineChecksums
if (-not (Test-Path $baselineFull)) {
    $failures += "baseline manifest missing: $BaselineManifest"
}
if (-not (Test-Path $checksumsFull)) {
    $failures += "baseline checksums missing: $BaselineChecksums"
}

$liveLock = ''
$lockPath = Join-Path $repoRoot 'Cargo.lock'
if (Test-Path $lockPath) {
    $liveLock = Get-Sha256Hex -Path $lockPath
} else {
    $failures += "Cargo.lock missing"
}

if ((Test-Path $baselineFull) -and (Test-Path $checksumsFull) -and $liveLock) {
    $baseline = Get-Content -Raw -Path $baselineFull | ConvertFrom-Json
    $checksums = Get-Content -Raw -Path $checksumsFull | ConvertFrom-Json

    if ($baseline.release_ready -eq $true) {
        $failures += "honesty: baseline release_ready must be false"
    }
    if ($baseline.signed -eq $true) {
        $failures += "honesty: baseline signed must be false (no installers)"
    }
    if ($checksums.release_ready -eq $true) {
        $failures += "honesty: checksums.release_ready must be false"
    }

    $manifestLock = [string]$baseline.source.cargo_lock_sha256
    if ([string]::IsNullOrWhiteSpace($manifestLock)) {
        $failures += "baseline cargo_lock_sha256 missing — run release-dry-run.ps1"
    } elseif ($manifestLock -ne $liveLock) {
        $failures += "baseline lock digest != live Cargo.lock"
    }

    $lockEntry = $checksums.entries | Where-Object { $_.kind -eq 'lockfile' } | Select-Object -First 1
    if ($null -eq $lockEntry) {
        $failures += "checksums missing lockfile entry"
    } elseif ([string]$lockEntry.sha256 -ne $liveLock) {
        $failures += "checksums lock digest != live Cargo.lock"
    }

    # Upgrade scaffold: candidate dry-run must also refuse RELEASE_READY and share lock identity.
    $candidateFull = Join-Path $repoRoot $CandidateManifest
    if (Test-Path $candidateFull) {
        $candidate = Get-Content -Raw -Path $candidateFull | ConvertFrom-Json
        if ($candidate.release_ready -eq $true) {
            $failures += "honesty: candidate dry-run release_ready must be false"
        }
        $candLock = [string]$candidate.source.cargo_lock_sha256
        if ($candLock -ne $liveLock) {
            $failures += "candidate lock digest != live Cargo.lock (upgrade scaffold requires lock continuity)"
        }
        Write-Host "Candidate dry-run present: $CandidateManifest (source/tree may differ; lock continuity required)."
    } else {
        Write-Host "Note: candidate dry-run snapshot absent ($CandidateManifest); baseline-only scaffold check."
    }

    # Rollback scaffold: document that rollback == re-verify baseline digests (no installer mutation).
    Write-Host "Rollback scaffold: re-run verify-release-manifest.ps1 + verify-package-checksums.ps1 against baseline; no package manager rollback (installers absent)."
}

if ($failures.Count -gt 0) {
    Write-Host "UPGRADE/ROLLBACK SCAFFOLD VERIFY FAILED ($($failures.Count) issues)"
    $failures | ForEach-Object { Write-Host " - $_" }
    Write-Host "LIMITATION: scaffold only; RELEASE_READY remains false; no real package upgrade/rollback."
    exit 1
}

Write-Host "UPGRADE/ROLLBACK SCAFFOLD VERIFY OK"
Write-Host "LIMITATION: no MSI/MSIX/DMG/deb installers; release_package_upgrade_rollback_proof still missing."
exit 0
