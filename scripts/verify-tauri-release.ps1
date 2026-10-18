# Spec 101 T101-05: fail-closed verifier for a release set made by
# scripts/package-tauri-release.ps1. Exits non-zero on any missing, unexpected
# or tampered file, on a source-binding mismatch, or on a readiness flag.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Root,
    [string]$ExpectedSourceSha = ''
)
$ErrorActionPreference = 'Stop'
$Root = [IO.Path]::GetFullPath($Root)
$failures = [Collections.Generic.List[string]]::new()
function Sha([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
function Rel([string]$Path) { [IO.Path]::GetRelativePath($Root, $Path).Replace('\', '/') }

$sumsPath = Join-Path $Root 'SHA256SUMS'
$manifestPath = Join-Path $Root 'package-manifest.json'
foreach ($required in $sumsPath, $manifestPath, (Join-Path $Root 'LICENSE'), (Join-Path $Root 'NOTICE.md'), (Join-Path $Root 'SBOM.cdx.json')) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { $failures.Add("missing required file: $(Rel $required)") }
}
if ($failures.Count -eq 0) {
    $listed = @{}
    foreach ($line in (Get-Content -LiteralPath $sumsPath)) {
        if ($line -notmatch '^([0-9a-f]{64})  (.+)$') { $failures.Add("malformed SHA256SUMS line: $line"); continue }
        $listed[$Matches[2]] = $Matches[1]
    }
    $actual = @(Get-ChildItem -LiteralPath $Root -Recurse -File | Where-Object { $_.FullName -ne $sumsPath } | ForEach-Object { Rel $_.FullName })
    foreach ($rel in $actual) {
        if (-not $listed.ContainsKey($rel)) { $failures.Add("unexpected file: $rel"); continue }
        if ((Sha (Join-Path $Root $rel)) -ne $listed[$rel]) { $failures.Add("digest mismatch: $rel") }
    }
    foreach ($rel in $listed.Keys) { if ($actual -notcontains $rel) { $failures.Add("listed but missing: $rel") } }

    $m = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($m.schema -ne 'medscale.tauri-release-manifest.v1') { $failures.Add('unknown manifest schema') }
    foreach ($flag in 'signed', 'notarized', 'release_ready', 'reproducible_binary_build') {
        if ($m.$flag -ne $false) { $failures.Add("manifest flag must be false: $flag") }
    }
    if ($m.signing -ne 'NOT_GRANTED') { $failures.Add('signing must be NOT_GRANTED') }
    if ($m.source_sha -notmatch '^[0-9a-f]{40}$' -or $m.tree_sha -notmatch '^[0-9a-f]{40}$') { $failures.Add('source/tree SHA malformed') }
    if ($ExpectedSourceSha -and $m.source_sha -ne $ExpectedSourceSha) { $failures.Add("source SHA $($m.source_sha) != expected $ExpectedSourceSha") }
    $installerPath = Join-Path $Root $m.installer.path
    if (-not (Test-Path -LiteralPath $installerPath -PathType Leaf)) { $failures.Add("installer missing: $($m.installer.path)") }
    elseif ((Sha $installerPath) -ne $m.installer.sha256 -or (Get-Item -LiteralPath $installerPath).Length -ne $m.installer.bytes) { $failures.Add('installer digest/size differs from manifest') }
    if ((Sha (Join-Path $Root 'SBOM.cdx.json')) -ne $m.sbom_sha256) { $failures.Add('SBOM digest differs from manifest') }
    if ((Sha (Join-Path $Root 'NOTICE.md')) -ne $m.notice_sha256) { $failures.Add('NOTICE digest differs from manifest') }
    $sbomSource = ((Get-Content -LiteralPath (Join-Path $Root 'SBOM.cdx.json') -Raw | ConvertFrom-Json).metadata.properties | Where-Object name -eq 'medscale:source_sha').value
    if ($sbomSource -ne $m.source_sha) { $failures.Add("SBOM source_sha $sbomSource != manifest $($m.source_sha)") }
    foreach ($p in @($m.payload)) {
        if ($p.path -eq 'package-manifest.json') { continue }
        $f = Join-Path $Root $p.path
        if (-not (Test-Path -LiteralPath $f -PathType Leaf) -or (Sha $f) -ne $p.sha256) { $failures.Add("manifest payload mismatch: $($p.path)") }
    }
}
if ($failures.Count) {
    $failures | ForEach-Object { Write-Output "FAIL $_" }
    Write-Output "TAURI_RELEASE_VERIFY=FAIL ($($failures.Count))"
    exit 1
}
Write-Output "TAURI_RELEASE_VERIFY=PASS root=$Root"
