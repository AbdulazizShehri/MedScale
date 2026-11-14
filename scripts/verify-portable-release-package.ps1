# Spec 058 — fail-closed portable package verifier.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$PackagePath,
    [string]$ExtractRoot = ''
)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot
function Get-Sha256Hex([string]$Path) { return (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant() }
function Invoke-Smoke([string]$Root, [string]$Rel, [string[]]$CommandArgs) {
    $path = Join-Path $Root $Rel
    if (-not $IsWindows) { & chmod +x $path; if ($LASTEXITCODE -ne 0) { throw "chmod failed: $Rel" } }
    & $path @CommandArgs | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "packaged binary smoke failed: $Rel" }
}
$packageFull = if ([IO.Path]::IsPathRooted($PackagePath)) { $PackagePath } else { Join-Path $repoRoot $PackagePath }
if (-not (Test-Path $packageFull)) { throw "package missing: $PackagePath" }
$ownedTemp = [string]::IsNullOrWhiteSpace($ExtractRoot)
$extract = if ($ownedTemp) { Join-Path ([IO.Path]::GetTempPath()) ('medscale-verify-' + [guid]::NewGuid().ToString('N')) } else { $ExtractRoot }
if (Test-Path $extract) { Remove-Item -Recurse -Force $extract }
New-Item -ItemType Directory -Force -Path $extract | Out-Null
try {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::ExtractToDirectory($packageFull, $extract)
    $manifestPath = Join-Path $extract 'package-manifest.json'
    if (-not (Test-Path $manifestPath)) { throw 'package-manifest.json missing' }
    $manifest = Get-Content -Raw $manifestPath | ConvertFrom-Json
    if ($manifest.release_ready -eq $true -or $manifest.signed -eq $true -or $manifest.notarized -eq $true -or $manifest.reproducible_binary_build -eq $true) {
        throw 'package honesty flags invalid'
    }
    $exe = if ($manifest.platform -eq 'windows') { '.exe' } else { '' }
    $mandatory = @("bin/medscale$exe", "bin/medscale-desktop$exe", 'LICENSE','NOTICE.md','README.md','SBOM.cdx.json','package-manifest.json')
    # Spec 095 adds font redistribution evidence; historical Spec 058 packages retain their original inventory.
    $fontLicenseProfile = $manifest.PSObject.Properties['font_license_profile']
    if ($null -ne $fontLicenseProfile) {
        if ([string]$fontLicenseProfile.Value -ne '095-brand-foundation') { throw 'unsupported font license profile' }
        $mandatory += @('licenses/fonts/Inter-OFL.txt', 'licenses/fonts/JetBrainsMono-OFL.txt', 'licenses/fonts/FONT_NOTICE.md')
    }
    $mandatory = $mandatory | Sort-Object
    $actual = Get-ChildItem -Recurse -File $extract | ForEach-Object { $_.FullName.Substring($extract.Length).TrimStart('\','/').Replace('\','/') } | Sort-Object
    if (($mandatory -join "`n") -ne ($actual -join "`n")) { throw "package file inventory mismatch`nexpected=$($mandatory -join ',')`nactual=$($actual -join ',')" }
    $payloadPaths = @($manifest.payload | ForEach-Object { [string]$_.path }) | Sort-Object
    $expectedPayload = @($mandatory | Where-Object { $_ -ne 'package-manifest.json' } | Sort-Object)
    if (($expectedPayload -join "`n") -ne ($payloadPaths -join "`n")) { throw 'manifest payload inventory mismatch' }
    foreach ($entry in $manifest.payload) {
        $path = Join-Path $extract ([string]$entry.path)
        if (-not (Test-Path $path)) { throw "manifest payload missing: $($entry.path)" }
        $actualHash = Get-Sha256Hex $path
        if ($actualHash -ne [string]$entry.sha256) { throw "payload hash mismatch: $($entry.path)" }
    }
    if ($null -ne $fontLicenseProfile) {
        $fontNotice = [System.IO.File]::ReadAllText((Join-Path $extract 'licenses/fonts/FONT_NOTICE.md')).TrimEnd()
        $packageNotice = [System.IO.File]::ReadAllText((Join-Path $extract 'NOTICE.md'))
        if ([string]::IsNullOrWhiteSpace($fontNotice) -or -not $packageNotice.Contains($fontNotice)) {
            throw 'packaged font attribution missing from NOTICE.md'
        }
    }
    $sbom = Get-Content -Raw (Join-Path $extract 'SBOM.cdx.json') | ConvertFrom-Json
    $props = @{}
    foreach ($prop in $sbom.metadata.properties) { $props[[string]$prop.name] = [string]$prop.value }
    if ($props['medscale:source_sha'] -ne [string]$manifest.source.git_sha) { throw 'SBOM source SHA mismatch' }
    if ($props['medscale:tree_sha'] -ne [string]$manifest.source.tree_sha) { throw 'SBOM tree SHA mismatch' }
    if ($props['medscale:cargo_lock_sha256'] -ne [string]$manifest.source.cargo_lock_sha256) { throw 'SBOM Cargo.lock mismatch' }
    if ($props['medscale:public_project_license'] -ne 'Apache-2.0') { throw 'SBOM public license mismatch' }
    Invoke-Smoke $extract "bin/medscale$exe" @('--help')
    Invoke-Smoke $extract "bin/medscale-desktop$exe" @('--smoke')
    Write-Host "PACKAGE_VERIFY_OK=$packageFull"
    Write-Host "PACKAGE_LABEL=$($manifest.package_label)"
    Write-Output $manifest
} finally {
    if ($ownedTemp -and (Test-Path $extract)) { Remove-Item -Recurse -Force $extract }
}
