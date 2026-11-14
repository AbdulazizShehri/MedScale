# Spec 058 — versioned-slot portable install/upgrade/rollback state machine.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('install','upgrade','rollback')][string]$Action,
    [string]$PackagePath = '',
    [Parameter(Mandatory = $true)][string]$InstallRoot
)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot
function Write-State([string]$Path, $State) {
    $enc = [Text.UTF8Encoding]::new($false)
    [IO.File]::WriteAllText($Path, (($State | ConvertTo-Json -Depth 5).TrimEnd() + "`n"), $enc)
}
function Invoke-ActiveSmoke([string]$Root, [string]$Slot) {
    $manifest = Get-Content -Raw (Join-Path $Root "slots/$Slot/package-manifest.json") | ConvertFrom-Json
    $exe = if ($manifest.platform -eq 'windows') { '.exe' } else { '' }
    foreach ($item in @(@("bin/medscale$exe",'--help'), @("bin/medscale-desktop$exe",'--smoke'))) {
        $path = Join-Path $Root "slots/$Slot/$($item[0])"
        if (-not $IsWindows) { & chmod +x $path | Out-Null }
        & $path $item[1] | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "active slot smoke failed: $($item[0])" }
    }
}
New-Item -ItemType Directory -Force -Path (Join-Path $InstallRoot 'slots') | Out-Null
$statePath = Join-Path $InstallRoot 'state.json'
$state = if (Test-Path $statePath) { Get-Content -Raw $statePath | ConvertFrom-Json } else { [pscustomobject]@{ current_slot = $null; previous_slot = $null } }
if ($Action -eq 'rollback') {
    if ([string]::IsNullOrWhiteSpace([string]$state.previous_slot)) { throw 'rollback unavailable: previous_slot absent' }
    $oldCurrent = [string]$state.current_slot
    $state.current_slot = [string]$state.previous_slot
    $state.previous_slot = $oldCurrent
    Write-State $statePath $state
    Invoke-ActiveSmoke $InstallRoot ([string]$state.current_slot)
    Write-Host "ROLLBACK_OK=$($state.current_slot)"
    exit 0
}
if ([string]::IsNullOrWhiteSpace($PackagePath)) { throw "$Action requires PackagePath" }
$verifyRoot = Join-Path ([IO.Path]::GetTempPath()) ('medscale-install-verify-' + [guid]::NewGuid().ToString('N'))
try {
    & (Join-Path $PSScriptRoot 'verify-portable-release-package.ps1') -PackagePath $PackagePath -ExtractRoot $verifyRoot | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'package verification failed before install' }
    $manifest = Get-Content -Raw (Join-Path $verifyRoot 'package-manifest.json') | ConvertFrom-Json
    $packageHash = (Get-FileHash -Algorithm SHA256 -Path $PackagePath).Hash.ToLowerInvariant()
    $safeLabel = ([string]$manifest.package_label) -replace '[^A-Za-z0-9._-]','_'
    $slot = "$safeLabel-$($packageHash.Substring(0,16))"
    $slotPath = Join-Path $InstallRoot "slots/$slot"
    if (Test-Path $slotPath) { Remove-Item -Recurse -Force $slotPath }
    New-Item -ItemType Directory -Force -Path $slotPath | Out-Null
    Get-ChildItem -Force $verifyRoot | Copy-Item -Destination $slotPath -Recurse -Force
    $oldCurrent = [string]$state.current_slot
    if ($Action -eq 'install' -and -not [string]::IsNullOrWhiteSpace($oldCurrent)) { throw 'install refused: current slot already exists' }
    if ($Action -eq 'upgrade' -and [string]::IsNullOrWhiteSpace($oldCurrent)) { throw 'upgrade refused: current slot absent' }
    $state.previous_slot = if ($Action -eq 'upgrade') { $oldCurrent } else { $null }
    $state.current_slot = $slot
    Write-State $statePath $state
    Invoke-ActiveSmoke $InstallRoot $slot
    Write-Host "${Action}_OK=$slot"
} finally {
    if (Test-Path $verifyRoot) { Remove-Item -Recurse -Force $verifyRoot }
}
