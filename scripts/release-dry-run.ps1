# Spec 047 — Release dry-run: bind live source/tree/lock digests into the unsigned
# release-manifest scaffold, record build-environment identity, and write an
# admissions-derived native-deps honesty inventory.
#
# Does NOT sign, notarize, produce installers, or claim RELEASE_READY.
#
# Usage:
#   pwsh ./scripts/release-dry-run.ps1
#   pwsh ./scripts/release-dry-run.ps1 -SkipRefreshScaffolds

[CmdletBinding()]
param(
    [switch]$SkipRefreshScaffolds,
    [string]$ManifestOut = "evidence/039-release-honesty-packets/release-manifest.scaffold.json",
    [string]$EvidenceDir = "evidence/047-release-dry-run-verifier"
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

function Get-Sha256Hex {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant()
}

$lockPath = Join-Path $repoRoot 'Cargo.lock'
if (-not (Test-Path $lockPath)) {
    throw "Cargo.lock missing; refuse unbound release dry-run"
}

$sourceSha = (git -C $repoRoot rev-parse HEAD).Trim()
# PowerShell-safe tree SHA (avoid HEAD^{tree} caret escaping)
$treeSha = (git -C $repoRoot log -1 --format=%T).Trim()
$lockHash = Get-Sha256Hex -Path $lockPath

$rustcVersion = ''
try {
    $rustcVersion = (rustc --version 2>$null)
    if ($null -eq $rustcVersion) { $rustcVersion = '' } else { $rustcVersion = $rustcVersion.Trim() }
} catch {
    $rustcVersion = ''
}

$osName = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()

if (-not $SkipRefreshScaffolds) {
    Write-Host "Refreshing SBOM scaffold + package checksums..."
    & pwsh -NoProfile -File (Join-Path $repoRoot 'scripts/generate-sbom-scaffold.ps1')
    if ($LASTEXITCODE -ne 0) { throw "generate-sbom-scaffold.ps1 failed" }
    & pwsh -NoProfile -File (Join-Path $repoRoot 'scripts/generate-package-checksums.ps1')
    if ($LASTEXITCODE -ne 0) { throw "generate-package-checksums.ps1 failed" }
}

$nativeInventory = [ordered]@{
    schema_version           = 1
    spec_id                  = '047-release-dry-run-verifier'
    generated_at             = (Get-Date).ToUniversalTime().ToString('o')
    release_ready            = $false
    model_assets_included    = $false
    pack_weights_included    = $false
    note                     = 'Admissions-derived native/FFI honesty inventory only; not a complete release SBOM; model/Pack assets missing.'
    admitted_native_components = @(
        [ordered]@{
            id           = 'sqlcipher-via-rusqlite'
            admission    = 'docs/engineering/admissions/005-sqlcipher-rusqlite.md'
            kind         = 'embedded_native'
            measured_note = 'cipher_version measured in Spec 023 evidence (community build via rusqlite bundled-sqlcipher)'
        }
        [ordered]@{
            id        = 'landlock'
            admission = 'docs/engineering/admissions/026-landlock.md'
            kind      = 'linux_sandbox_api'
        }
        [ordered]@{
            id        = 'macos-sandbox-init-ffi'
            admission = 'docs/engineering/admissions/031-macos-sandbox-init-ffi.md'
            kind      = 'macos_ffi'
        }
        [ordered]@{
            id        = 'windows-sys'
            admission = 'docs/engineering/admissions/030-windows-sys.md'
            kind      = 'windows_api'
        }
    )
}

$evidenceFull = Join-Path $repoRoot $EvidenceDir
if (-not (Test-Path $evidenceFull)) {
    New-Item -ItemType Directory -Force -Path $evidenceFull | Out-Null
}

$nativeOut = Join-Path $evidenceFull 'native-deps-inventory.json'
($nativeInventory | ConvertTo-Json -Depth 8) | Set-Content -Path $nativeOut -Encoding utf8
Write-Host "Wrote $nativeOut"

$manifest = [ordered]@{
    schema_version = 1
    spec_id        = '047-release-dry-run-verifier'
    release_ready  = $false
    signed         = $false
    notarized      = $false
    note           = 'Unsigned release-manifest dry-run (Spec 047). Digests bound to live checkout; never claim RELEASE_READY from this file alone.'
    source         = [ordered]@{
        git_sha            = $sourceSha
        tree_sha           = $treeSha
        cargo_lock_sha256  = $lockHash
    }
    build_environment = [ordered]@{
        os          = "$osName"
        arch        = "$arch"
        rustc       = "$rustcVersion"
        note        = 'Host identity at dry-run time; not a reproducible binary build attestation.'
    }
    artifacts = @()
    sbom_path       = 'evidence/027-perf-sbom-release-evidence/sbom-scaffold.json'
    checksums_path  = 'evidence/027-perf-sbom-release-evidence/package-checksums.json'
    native_deps_path = ($EvidenceDir.Replace('\', '/') + '/native-deps-inventory.json')
    privacy = [ordered]@{
        private_data_ready  = $false
        real_phi_authorized = $false
    }
    sandbox = [ordered]@{
        platform_qualified                      = $false
        windows_appcontainer_network_measured   = $true
        windows_appcontainer_lpac_measured       = $true
    }
    rights = [ordered]@{
        public_source_license_choice = 'Apache-2.0'
    }
}

$manifestFull = Join-Path $repoRoot $ManifestOut
$manifestDir = Split-Path -Parent $manifestFull
if (-not (Test-Path $manifestDir)) {
    New-Item -ItemType Directory -Force -Path $manifestDir | Out-Null
}
($manifest | ConvertTo-Json -Depth 8) | Set-Content -Path $manifestFull -Encoding utf8
Write-Host "Wrote $manifestFull"

$snapshot = Join-Path $evidenceFull 'release-manifest.dry-run.json'
Copy-Item -Force -Path $manifestFull -Destination $snapshot
Write-Host "Snapshot $snapshot"

Write-Host "DRY-RUN OK: source=$sourceSha tree=$treeSha lock=$lockHash"
Write-Host "LIMITATION: unsigned scaffold; RELEASE_READY remains false; model assets not included."
