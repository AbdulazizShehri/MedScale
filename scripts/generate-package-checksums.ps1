# Generate a sha256 checksum manifest for workspace crate sources + Cargo.lock (Spec 027).
# This is NOT a signed release package.
#
# Usage:
#   pwsh ./scripts/generate-package-checksums.ps1
#   pwsh ./scripts/generate-package-checksums.ps1 -OutPath evidence/027-perf-sbom-release-evidence/package-checksums.json

[CmdletBinding()]
param(
    [string]$OutPath = "evidence/027-perf-sbom-release-evidence/package-checksums.json"
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

function Get-Sha256Hex {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant()
}

$entries = @()

$lockPath = Join-Path $repoRoot 'Cargo.lock'
if (-not (Test-Path $lockPath)) {
    throw "Cargo.lock missing at $lockPath"
}
$entries += [ordered]@{
    path   = 'Cargo.lock'
    kind   = 'lockfile'
    sha256 = (Get-Sha256Hex -Path $lockPath)
}

$crateRoots = Get-ChildItem -Path (Join-Path $repoRoot 'crates') -Directory
foreach ($crateDir in $crateRoots) {
    $relCrate = "crates/$($crateDir.Name)"
    $files = Get-ChildItem -Path $crateDir.FullName -Recurse -File |
        Where-Object {
            $_.FullName -notmatch '[\\/]target[\\/]' -and
            $_.Name -ne '.DS_Store'
        } |
        Sort-Object FullName

    $fileDigests = @()
    foreach ($f in $files) {
        $rel = $f.FullName.Substring($repoRoot.Length).TrimStart('\', '/').Replace('\', '/')
        $fileDigests += [ordered]@{
            path   = $rel
            sha256 = (Get-Sha256Hex -Path $f.FullName)
        }
    }

    # Aggregate digest over sorted "path=sha256" lines for a stable crate tree identity.
    $aggInput = ($fileDigests | ForEach-Object { "$($_.path)=$($_.sha256)" }) -join "`n"
    $aggBytes = [System.Text.Encoding]::UTF8.GetBytes($aggInput)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $hash = $sha.ComputeHash($aggBytes)
        $aggHex = ([System.BitConverter]::ToString($hash) -replace '-', '').ToLowerInvariant()
    }
    finally {
        $sha.Dispose()
    }

    $entries += [ordered]@{
        path              = $relCrate
        kind              = 'crate_source_tree'
        file_count        = $fileDigests.Count
        tree_sha256       = $aggHex
        files             = $fileDigests
    }
}

$manifest = [ordered]@{
    schema_version = 1
    spec_id        = '027-perf-sbom-release-evidence'
    generated_at   = (Get-Date).ToUniversalTime().ToString('o')
    release_ready  = $false
    signed         = $false
    note           = 'Workspace source + lock digests only; not a reproducible binary package or signed release.'
    entries        = $entries
}

$outFull = Join-Path $repoRoot $OutPath
$outDir = Split-Path -Parent $outFull
if (-not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

($manifest | ConvertTo-Json -Depth 8) | Set-Content -Path $outFull -Encoding utf8
Write-Host "Wrote $outFull ($($entries.Count) top-level entries)"
Write-Host "LIMITATION: unsigned checksum scaffold; RELEASE_READY remains false."
