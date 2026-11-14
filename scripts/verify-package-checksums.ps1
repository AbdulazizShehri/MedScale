# Verify package checksum manifest (Spec 037).
# Compares an existing package-checksums.json against freshly computed SHA-256 digests.
# Does NOT sign, publish, or claim RELEASE_READY.
#
# Usage:
#   pwsh ./scripts/verify-package-checksums.ps1
#   pwsh ./scripts/verify-package-checksums.ps1 -ManifestPath evidence/027-perf-sbom-release-evidence/package-checksums.json
#
# Exit codes:
#   0 = all listed digests match
#   1 = mismatch / missing file / invalid manifest

[CmdletBinding()]
param(
    [string]$ManifestPath = "evidence/027-perf-sbom-release-evidence/package-checksums.json"
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$manifestFull = Join-Path $repoRoot $ManifestPath
if (-not (Test-Path $manifestFull)) {
    Write-Error "Manifest missing: $manifestFull — run generate-package-checksums.ps1 first."
    exit 1
}

function Get-Sha256Hex {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant()
}

$manifest = Get-Content -Raw -Path $manifestFull | ConvertFrom-Json
if ($null -eq $manifest.entries) {
    Write-Error "Manifest has no entries"
    exit 1
}

$failures = @()
foreach ($entry in $manifest.entries) {
    if ($entry.kind -eq 'lockfile') {
        $path = Join-Path $repoRoot $entry.path
        if (-not (Test-Path $path)) {
            $failures += "missing $($entry.path)"
            continue
        }
        $got = Get-Sha256Hex -Path $path
        if ($got -ne $entry.sha256) {
            $failures += "lockfile mismatch $($entry.path) expected=$($entry.sha256) got=$got"
        }
        continue
    }

    if ($entry.kind -eq 'crate_source_tree') {
        if ($null -eq $entry.files) { continue }
        foreach ($f in $entry.files) {
            $path = Join-Path $repoRoot $f.path
            if (-not (Test-Path $path)) {
                $failures += "missing $($f.path)"
                continue
            }
            $got = Get-Sha256Hex -Path $path
            if ($got -ne $f.sha256) {
                $failures += "file mismatch $($f.path)"
            }
        }
    }
}

if ($failures.Count -gt 0) {
    Write-Host "VERIFY FAILED ($($failures.Count) issues)"
    $failures | ForEach-Object { Write-Host " - $_" }
    Write-Host "LIMITATION: unsigned scaffold; RELEASE_READY remains false."
    exit 1
}

Write-Host "VERIFY OK: digests match $ManifestPath"
Write-Host "LIMITATION: unsigned scaffold; RELEASE_READY remains false."
exit 0
