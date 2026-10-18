# Spec 101 T101-09: Linux deb qualification. CI RUNNERS ONLY.
# Records package metadata and contents, installs with dpkg on the disposable
# runner, launch-probes under xvfb without interaction, removes the package
# and reports residue.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ReleaseRoot,
    [string]$ReportPath = '',
    [int]$LaunchSeconds = 20
)
$ErrorActionPreference = 'Stop'
if (-not $IsLinux) { throw 'Linux only' }
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'refusing to run outside GitHub Actions (disposable runner required)' }
$manifest = Get-Content -LiteralPath (Join-Path $ReleaseRoot 'package-manifest.json') -Raw | ConvertFrom-Json
# Absolute path: apt reads a relative 'dir/file.deb' as 'package/release'.
$deb = [IO.Path]::GetFullPath((Join-Path $ReleaseRoot $manifest.installer.path))
$debs = @(Get-ChildItem -LiteralPath (Split-Path $deb) -Filter '*.deb' -File)
if ($debs.Count -ne 1 -or $debs[0].FullName -ne $deb) { throw "expected exactly the manifest .deb, found $($debs.Count) candidate(s)" }
if ((Get-FileHash -LiteralPath $deb -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.installer.sha256) { throw 'deb digest differs from release manifest' }
$result = [ordered]@{ source_sha = $manifest.source_sha; installer_sha256 = $manifest.installer.sha256; INSTALL = 'FAIL'; LAUNCH = 'NOT_RUN'; UNINSTALL = 'NOT_RUN'; SIGNING = 'NOT_GRANTED' }
$pkg = (& dpkg-deb --field $deb Package).Trim()
$result.package = [ordered]@{ name = $pkg; version = (& dpkg-deb --field $deb Version).Trim(); architecture = (& dpkg-deb --field $deb Architecture).Trim(); depends = (& dpkg-deb --field $deb Depends).Trim() }
$result.contents = @(& dpkg-deb --contents $deb | ForEach-Object { ($_ -split '\s+', 6)[5] })
if ((& dpkg-query -W -f '${Status}' $pkg 2>$null) -match 'installed') { throw "$pkg is already installed; refusing" }

& sudo apt-get install -y --no-install-recommends "$deb" | Out-Host
$installExit = $LASTEXITCODE
$installedFiles = @(& dpkg -L $pkg 2>$null | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf })
$exe = $installedFiles | Where-Object { $_ -like '/usr/bin/*' } | Select-Object -First 1
if ($installExit -eq 0 -and $exe) { $result.INSTALL = 'PASS'; $result.installed_executable = $exe }

if ($result.INSTALL -eq 'PASS') {
    $result.LAUNCH = 'FAIL'
    $log = Join-Path ([IO.Path]::GetTempPath()) 'medscale-101-launch.log'
    $proc = Start-Process -FilePath 'xvfb-run' -ArgumentList @('-a', $exe) -PassThru -RedirectStandardError $log
    Start-Sleep -Seconds $LaunchSeconds
    $app = @(& pgrep -f $exe)
    $result.launch_detail = [ordered]@{ alive_after_seconds = $LaunchSeconds; app_pids = $app; wrapper_exited = $proc.HasExited; stderr_tail = @(Get-Content -LiteralPath $log -Tail 5 -ErrorAction SilentlyContinue) }
    if ($app.Count -gt 0) { $result.LAUNCH = 'PASS' }
    & pkill -f $exe 2>$null; Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
}

# Uninstall is only meaningful evidence when the package was installed.
if ($result.INSTALL -eq 'PASS') {
$result.UNINSTALL = 'FAIL'
& sudo dpkg -r $pkg | Out-Host
$removeExit = $LASTEXITCODE
$residue = @($installedFiles | Where-Object { Test-Path -LiteralPath $_ })
$result.uninstall_detail = [ordered]@{ exit_code = $removeExit; residue = $residue
    app_data = @(foreach ($d in "$HOME/.local/share/org.medscale.desktop.preview", "$HOME/.config/org.medscale.desktop.preview") { if (Test-Path -LiteralPath $d) { $d } }) }
if ($removeExit -eq 0 -and $residue.Count -eq 0) { $result.UNINSTALL = 'PASS' }
}

$json = $result | ConvertTo-Json -Depth 6
if ($ReportPath) { New-Item -ItemType Directory -Path (Split-Path $ReportPath) -Force | Out-Null; $json | Set-Content -LiteralPath $ReportPath -Encoding utf8 }
$json
"INSTALL=$($result.INSTALL) LAUNCH=$($result.LAUNCH) UNINSTALL=$($result.UNINSTALL) SIGNING=NOT_GRANTED"
if ($result.INSTALL -ne 'PASS' -or $result.LAUNCH -ne 'PASS' -or $result.UNINSTALL -ne 'PASS') { exit 1 }
# All three axes passed; do not let a stale native exit code (pkill, hdiutil detach, ...) fail the step.
exit 0
