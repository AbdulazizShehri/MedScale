# Generate NOTICE / third-party attribution inventory (Spec 032).
# Builds from cargo metadata + deny.toml license allowlist.
# Public SPDX decisions remain founder-owned. Apache-2.0 was selected on 2026-09-14;
# this generator binds that decision into NOTICE inventory without claiming RELEASE_READY.
#
# Usage:
#   pwsh ./scripts/generate-notice-inventory.ps1
#   pwsh ./scripts/generate-notice-inventory.ps1 -JsonOut evidence/032-privacy-probes-notice-perf/notice-inventory.json

[CmdletBinding()]
param(
    [string]$JsonOut = "evidence/032-privacy-probes-notice-perf/notice-inventory.json",
    [string]$MarkdownOut = "docs/legal/NOTICE_INVENTORY.md"
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

function Get-DenyAllowlist {
    param([string]$DenyPath)
    $lines = Get-Content -Path $DenyPath
    $inLicenses = $false
    $inAllow = $false
    $allow = New-Object System.Collections.Generic.List[string]
    foreach ($line in $lines) {
        $t = $line.Trim()
        if ($t -eq '[licenses]') { $inLicenses = $true; $inAllow = $false; continue }
        if ($t.StartsWith('[') -and $t -ne '[licenses]') {
            if ($inLicenses -and $inAllow) { break }
            $inLicenses = $false
            $inAllow = $false
            continue
        }
        if ($inLicenses -and $t.StartsWith('allow')) { $inAllow = $true; continue }
        if ($inLicenses -and $inAllow) {
            if ($t -eq ']' -or $t -eq ']') { $inAllow = $false; continue }
            if ($t.StartsWith('"') -or $t.StartsWith("'")) {
                $lic = $t.Trim(',').Trim('"').Trim("'")
                if ($lic) { [void]$allow.Add($lic) }
            }
        }
    }
    return $allow
}

Write-Host "Generating NOTICE inventory via cargo metadata + deny.toml allowlist..."
$allowlist = Get-DenyAllowlist -DenyPath (Join-Path $repoRoot 'deny.toml')
$metadataJson = cargo metadata --format-version 1 --locked | Out-String
$metadata = $metadataJson | ConvertFrom-Json

$entries = @()

foreach ($pkg in $metadata.packages) {
    $isWorkspace = $false
    foreach ($wm in $metadata.workspace_members) {
        if ($wm -eq $pkg.id) { $isWorkspace = $true; break }
    }
    $license = if ($pkg.license) { [string]$pkg.license } else { '' }
    $entries += [ordered]@{
        name            = $pkg.name
        version         = $pkg.version
        license         = $license
        source          = if ($pkg.source) { [string]$pkg.source } else { 'path/workspace' }
        workspace_crate = $isWorkspace
        publish         = if ($null -ne $pkg.publish) { $pkg.publish } else { $null }
    }
}

$gitSha = (& git rev-parse HEAD 2>$null)
if (-not $gitSha) { $gitSha = 'unknown' }
$gitTree = (& git rev-parse 'HEAD^{tree}' 2>$null)
if (-not $gitTree) { $gitTree = 'unknown' }

$inventory = [ordered]@{
    schema_version            = 1
    spec_id                   = '032-privacy-probes-notice-perf'
    generated_utc             = (Get-Date).ToUniversalTime().ToString('o')
    git_sha                   = "$gitSha"
    git_tree                  = "$gitTree"
    rights_license_decision   = $true
    medscale_spdx_chosen      = $true
    medscale_spdx_identifier  = 'Apache-2.0'
    release_ready             = $false
    deny_allowlist_licenses   = @($allowlist)
    package_count             = $entries.Count
    packages                  = $entries
    honesty                   = @(
        'NOTICE inventory is attribution prep only.',
        'Founder selected Apache-2.0 for MedScale first-party source on 2026-09-14.',
        'PUBLIC_SOURCE_LICENSE_CHOICE EXTERNAL_GATES is DECIDED.',
        'Workspace crates remain publish=false; public source license metadata is Apache-2.0.',
        'Spec 060 admits Slint 1.16.1 under its royalty-free Desktop/Mobile/Web application license path with required About attribution.'
    )
}

$jsonFull = Join-Path $repoRoot $JsonOut
$jsonDir = Split-Path -Parent $jsonFull
if (-not (Test-Path $jsonDir)) {
    New-Item -ItemType Directory -Force -Path $jsonDir | Out-Null
}
($inventory | ConvertTo-Json -Depth 8) | Set-Content -Path $jsonFull -Encoding utf8
Write-Host "Wrote $jsonFull ($($entries.Count) packages)"

$mdFull = Join-Path $repoRoot $MarkdownOut
$mdDir = Split-Path -Parent $mdFull
if (-not (Test-Path $mdDir)) {
    New-Item -ItemType Directory -Force -Path $mdDir | Out-Null
}

$sb = New-Object System.Text.StringBuilder
[void]$sb.AppendLine('# NOTICE / Third-Party Inventory (Spec 032)')
[void]$sb.AppendLine('')
[void]$sb.AppendLine('**Status:** attribution inventory with founder-selected MedScale SPDX; not a RELEASE_READY claim.')
[void]$sb.AppendLine('')
[void]$sb.AppendLine("| Field | Value |")
[void]$sb.AppendLine("|---|---|")
[void]$sb.AppendLine("| Generated (UTC) | $($inventory.generated_utc) |")
[void]$sb.AppendLine("| Git SHA | $gitSha |")
[void]$sb.AppendLine("| Git tree | $gitTree |")
[void]$sb.AppendLine("| ``rights_license_decision`` | **true** |")
[void]$sb.AppendLine("| ``medscale_spdx_chosen`` | **true** |")
[void]$sb.AppendLine("| ``medscale_spdx_identifier`` | **Apache-2.0** |")
[void]$sb.AppendLine("| ``release_ready`` | **false** |")
[void]$sb.AppendLine("| Package count | $($entries.Count) |")
[void]$sb.AppendLine('')
[void]$sb.AppendLine('## cargo-deny license allowlist')
[void]$sb.AppendLine('')
foreach ($lic in $allowlist) {
    [void]$sb.AppendLine("- ``$lic``")
}
[void]$sb.AppendLine('')
[void]$sb.AppendLine('## Honesty')
[void]$sb.AppendLine('')
foreach ($h in $inventory.honesty) {
    [void]$sb.AppendLine("- $h")
}
[void]$sb.AppendLine('')
[void]$sb.AppendLine('## Packages (cargo metadata)')
[void]$sb.AppendLine('')
[void]$sb.AppendLine('| Name | Version | License | Workspace | Source |')
[void]$sb.AppendLine('|---|---|---|---|---|')
foreach ($e in ($entries | Sort-Object name, version)) {
    $ws = if ($e.workspace_crate) { 'yes' } else { 'no' }
    $lic = if ($e.license) { $e.license } else { '(none declared)' }
    $src = ($e.source -replace '\|', '/')
    [void]$sb.AppendLine("| $($e.name) | $($e.version) | $lic | $ws | $src |")
}
[void]$sb.AppendLine('')
[void]$sb.AppendLine('Machine-readable copy: `evidence/032-privacy-probes-notice-perf/notice-inventory.json`.')
[void]$sb.AppendLine('Regenerate: `pwsh ./scripts/generate-notice-inventory.ps1`.')

Set-Content -Path $mdFull -Value $sb.ToString() -Encoding utf8
Write-Host "Wrote $mdFull"
Write-Host "LICENSE DECISION: Apache-2.0; PUBLIC_SOURCE_LICENSE_CHOICE=DECIDED. RELEASE_READY remains false."
