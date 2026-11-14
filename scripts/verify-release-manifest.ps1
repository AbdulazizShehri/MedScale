# Spec 047 — Verify unsigned release-manifest dry-run against SBOM lock binding
# and package-checksum lock digest. Optional live git match via -RequireLiveGit.
#
# Does NOT sign, publish, or claim RELEASE_READY.
#
# Usage:
#   pwsh ./scripts/verify-release-manifest.ps1
#   pwsh ./scripts/verify-release-manifest.ps1 -RequireLiveGit
#   pwsh ./scripts/release-dry-run.ps1; pwsh ./scripts/verify-release-manifest.ps1 -RequireLiveGit
#
# Exit codes:
#   0 = cross-checks pass (honesty flags remain false)
#   1 = mismatch / honesty violation / missing artifact

[CmdletBinding()]
param(
    [string]$ManifestPath = "evidence/039-release-honesty-packets/release-manifest.scaffold.json",
    [switch]$RequireLiveGit
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

function Get-Sha256Hex {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant()
}

function Get-SbomLockHash {
    param([Parameter(Mandatory = $true)]$Sbom)
    foreach ($p in $Sbom.metadata.properties) {
        if ($p.name -eq 'medscale:cargo_lock_sha256') {
            return [string]$p.value
        }
    }
    return $null
}

$manifestFull = Join-Path $repoRoot $ManifestPath
if (-not (Test-Path $manifestFull)) {
    Write-Error "Manifest missing: $manifestFull — run release-dry-run.ps1 first."
    exit 1
}

$manifest = Get-Content -Raw -Path $manifestFull | ConvertFrom-Json
$failures = @()

if ($manifest.release_ready -eq $true) {
    $failures += "honesty: release_ready must be false for Spec 047 unsigned dry-run"
}
if ($manifest.signed -eq $true) {
    $failures += "honesty: signed must be false (no signing credentials in Spec 047)"
}
if ($manifest.notarized -eq $true) {
    $failures += "honesty: notarized must be false"
}

$lockPath = Join-Path $repoRoot 'Cargo.lock'
if (-not (Test-Path $lockPath)) {
    $failures += "missing Cargo.lock"
    $liveLock = ''
} else {
    $liveLock = Get-Sha256Hex -Path $lockPath
}

$manifestLock = [string]$manifest.source.cargo_lock_sha256
if ([string]::IsNullOrWhiteSpace($manifestLock)) {
    $failures += "source.cargo_lock_sha256 missing — run release-dry-run.ps1"
} elseif ($manifestLock -ne $liveLock) {
    $failures += "source.cargo_lock_sha256 mismatch expected=$liveLock got=$manifestLock"
}

if ($RequireLiveGit) {
    $liveSource = (git -C $repoRoot rev-parse HEAD).Trim()
    $liveTree = (git -C $repoRoot log -1 --format=%T).Trim()
    if ([string]$manifest.source.git_sha -ne $liveSource) {
        $failures += "source.git_sha mismatch expected=$liveSource got=$($manifest.source.git_sha)"
    }
    if ([string]$manifest.source.tree_sha -ne $liveTree) {
        $failures += "source.tree_sha mismatch expected=$liveTree got=$($manifest.source.tree_sha)"
    }
} else {
    if ([string]::IsNullOrWhiteSpace([string]$manifest.source.git_sha)) {
        $failures += "source.git_sha missing — run release-dry-run.ps1"
    }
    if ([string]::IsNullOrWhiteSpace([string]$manifest.source.tree_sha)) {
        $failures += "source.tree_sha missing — run release-dry-run.ps1"
    }
}

$sbomRel = [string]$manifest.sbom_path
if ([string]::IsNullOrWhiteSpace($sbomRel)) {
    $failures += "manifest.sbom_path missing"
} else {
    $sbomFull = Join-Path $repoRoot $sbomRel
    if (-not (Test-Path $sbomFull)) {
        $failures += "SBOM missing: $sbomRel"
    } else {
        $sbom = Get-Content -Raw -Path $sbomFull | ConvertFrom-Json
        $sbomLock = Get-SbomLockHash -Sbom $sbom
        if ([string]::IsNullOrWhiteSpace($sbomLock)) {
            $failures += "SBOM missing medscale:cargo_lock_sha256 property"
        } elseif ($sbomLock -ne $liveLock) {
            $failures += "SBOM cargo_lock_sha256 mismatch expected=$liveLock got=$sbomLock"
        }
        if ($sbomLock -ne $manifestLock) {
            $failures += "SBOM lock hash != manifest.source.cargo_lock_sha256"
        }
    }
}

$checksumsRel = [string]$manifest.checksums_path
if ([string]::IsNullOrWhiteSpace($checksumsRel)) {
    $failures += "manifest.checksums_path missing"
} else {
    $checksumsFull = Join-Path $repoRoot $checksumsRel
    if (-not (Test-Path $checksumsFull)) {
        $failures += "checksums missing: $checksumsRel"
    } else {
        $checksums = Get-Content -Raw -Path $checksumsFull | ConvertFrom-Json
        $lockEntry = $checksums.entries | Where-Object { $_.kind -eq 'lockfile' } | Select-Object -First 1
        if ($null -eq $lockEntry) {
            $failures += "checksums missing lockfile entry"
        } elseif ([string]$lockEntry.sha256 -ne $liveLock) {
            $failures += "checksums lockfile mismatch expected=$liveLock got=$($lockEntry.sha256)"
        }
        if ($checksums.release_ready -eq $true) {
            $failures += "honesty: checksums.release_ready must be false"
        }
    }
}

$nativeRel = [string]$manifest.native_deps_path
if (-not [string]::IsNullOrWhiteSpace($nativeRel)) {
    $nativeFull = Join-Path $repoRoot $nativeRel
    if (-not (Test-Path $nativeFull)) {
        $failures += "native_deps_path missing: $nativeRel"
    } else {
        $native = Get-Content -Raw -Path $nativeFull | ConvertFrom-Json
        if ($native.model_assets_included -eq $true) {
            $failures += "honesty: native inventory must keep model_assets_included=false"
        }
        if ($native.release_ready -eq $true) {
            $failures += "honesty: native inventory release_ready must be false"
        }
    }
} else {
    $failures += "native_deps_path missing"
}

if ($null -eq $manifest.build_environment) {
    $failures += "build_environment missing"
}

if ($failures.Count -gt 0) {
    Write-Host "VERIFY FAILED ($($failures.Count) issues)"
    $failures | ForEach-Object { Write-Host " - $_" }
    Write-Host "LIMITATION: unsigned dry-run verifier; RELEASE_READY remains false."
    exit 1
}

Write-Host "VERIFY OK: release-manifest dry-run cross-checks passed ($ManifestPath)"
if ($RequireLiveGit) {
    Write-Host "Live git identity matched."
} else {
    Write-Host "Note: git SHA/tree not re-checked against HEAD (pass -RequireLiveGit after a fresh dry-run)."
}
Write-Host "LIMITATION: unsigned scaffold; RELEASE_READY remains false."
exit 0
