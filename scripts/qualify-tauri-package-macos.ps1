# Spec 101 T101-08: macOS dmg/app qualification. CI RUNNERS ONLY.
# Mounts the dmg read-only, copies the app to a temporary directory, records
# bundle identity and signature state, launch-probes the executable without
# any interaction, then removes the copy and detaches the image.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ReleaseRoot,
    [string]$ReportPath = '',
    [int]$LaunchSeconds = 20
)
$ErrorActionPreference = 'Stop'
if (-not $IsMacOS) { throw 'macOS only' }
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'refusing to run outside GitHub Actions (disposable runner required)' }
$manifest = Get-Content -LiteralPath (Join-Path $ReleaseRoot 'package-manifest.json') -Raw | ConvertFrom-Json
$dmg = Join-Path $ReleaseRoot $manifest.installer.path
$result = [ordered]@{ source_sha = $manifest.source_sha; installer_sha256 = $manifest.installer.sha256; DMG_VERIFY = 'FAIL'; DMG_MOUNT = 'FAIL'; INSTALL = 'FAIL'; LAUNCH = 'NOT_RUN'; UNINSTALL = 'NOT_RUN'; SIGNING = 'NOT_GRANTED' }
if ((Get-FileHash -LiteralPath $dmg -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.installer.sha256) { throw 'dmg digest differs from release manifest' }
$mount = Join-Path ([IO.Path]::GetTempPath()) ('medscale-101-mnt-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
$work = Join-Path ([IO.Path]::GetTempPath()) ('medscale-101-app-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $mount, $work -Force | Out-Null
try {
    & hdiutil verify $dmg | Out-Host
    if ($LASTEXITCODE -eq 0) { $result.DMG_VERIFY = 'PASS' }
    # bundle.licenseFile embeds the license as a click-through agreement (SLA);
    # hdiutil cancels it without a terminal, so accept it non-interactively.
    $result.dmg_license_agreement = [bool]((& hdiutil imageinfo $dmg 2>$null | Out-String) -match 'Software License Agreement: true')
    & bash -c "yes | PAGER=cat hdiutil attach '$dmg' -nobrowse -readonly -mountpoint '$mount' >/dev/null"
    if ($LASTEXITCODE -ne 0) { throw 'hdiutil attach failed' }
    $result.DMG_MOUNT = 'PASS'
    $app = Get-ChildItem -LiteralPath $mount -Filter '*.app' -Directory | Select-Object -First 1
    if ($app) {
        & ditto $app.FullName (Join-Path $work $app.Name)
        $copy = Join-Path $work $app.Name
        $plist = Join-Path $copy 'Contents/Info.plist'
        $read = { param($k) (& /usr/libexec/PlistBuddy -c "Print :$k" $plist 2>$null) }
        $result.bundle = [ordered]@{ name = & $read 'CFBundleName'; identifier = & $read 'CFBundleIdentifier'; version = & $read 'CFBundleShortVersionString'; executable = & $read 'CFBundleExecutable' }
        $result.signature = (& codesign -dv $copy 2>&1 | Out-String).Trim()
        $exe = Join-Path $copy ("Contents/MacOS/" + $result.bundle.executable)
        if ((Test-Path -LiteralPath $exe) -and $result.bundle.identifier -eq 'org.medscale.desktop.preview') { $result.INSTALL = 'PASS' }
        if ($result.INSTALL -eq 'PASS') {
            $result.LAUNCH = 'FAIL'
            $proc = Start-Process -FilePath $exe -PassThru
            Start-Sleep -Seconds $LaunchSeconds
            $alive = -not $proc.HasExited
            $result.launch_detail = [ordered]@{ alive_after_seconds = $LaunchSeconds; alive = $alive; exit_code = $(if ($proc.HasExited) { $proc.ExitCode } else { $null }) }
            if ($alive) { $result.LAUNCH = 'PASS'; Stop-Process -Id $proc.Id -Force }
            Start-Sleep -Seconds 2
        }
        if ($result.INSTALL -eq 'PASS') {
            Remove-Item -LiteralPath $copy -Recurse -Force
            $result.UNINSTALL = if (Test-Path -LiteralPath $copy) { 'FAIL' } else { 'PASS' }
        }
        $support = Join-Path $HOME 'Library/Application Support/org.medscale.desktop.preview'
        $result.app_data_after_removal = @(if (Test-Path -LiteralPath $support) { Get-ChildItem -LiteralPath $support -Force | Select-Object -ExpandProperty Name })
    }
}
finally {
    & hdiutil detach $mount -quiet 2>$null | Out-Null
    Remove-Item -LiteralPath $mount, $work -Recurse -Force -ErrorAction SilentlyContinue
}
$json = $result | ConvertTo-Json -Depth 6
if ($ReportPath) { New-Item -ItemType Directory -Path (Split-Path $ReportPath) -Force | Out-Null; $json | Set-Content -LiteralPath $ReportPath -Encoding utf8 }
$json
"DMG_VERIFY=$($result.DMG_VERIFY) DMG_MOUNT=$($result.DMG_MOUNT) INSTALL=$($result.INSTALL) LAUNCH=$($result.LAUNCH) UNINSTALL=$($result.UNINSTALL) SIGNING=NOT_GRANTED"
if ($result.INSTALL -ne 'PASS' -or $result.LAUNCH -ne 'PASS' -or $result.UNINSTALL -ne 'PASS') { exit 1 }
# All three axes passed; do not let a stale native exit code (pkill, hdiutil detach, ...) fail the step.
exit 0
