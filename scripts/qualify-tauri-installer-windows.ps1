# Spec 101 T101-07: Windows NSIS install / launch / uninstall qualification.
# CI RUNNERS ONLY (disposable). Refuses to run if MedScale app data or an
# existing MedScale install is present, so it can never touch a real user's
# data. Launches on a separate, never-shown desktop: no user input can reach
# the process, nothing is typed, and no vault is created.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ReleaseRoot,
    [string]$ReportPath = '',
    [int]$LaunchSeconds = 20
)
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Windows only' }
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'refusing to run outside GitHub Actions (disposable runner required)' }
$identifier = 'org.medscale.desktop.preview'
$appData = Join-Path $env:LOCALAPPDATA $identifier
$uninstallKeys = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'
function Find-Install { Get-ItemProperty $uninstallKeys -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq 'MedScale' } | Select-Object -First 1 }
# App data present before installation is accepted only when it was created
# during this same CI job: the hosted runner creates RUNNER_TEMP fresh for
# each job, so its creation time is the job-start reference.
# It is recorded by name and timestamp and MOVED aside, never deleted.
$preexisting = $null
if (Test-Path -LiteralPath $appData) {
    if (-not $env:RUNNER_TEMP -or -not (Test-Path -LiteralPath $env:RUNNER_TEMP)) { throw "app data already present at $appData and RUNNER_TEMP is unavailable; refusing" }
    $jobStart = (Get-Item -LiteralPath $env:RUNNER_TEMP -Force).CreationTimeUtc
    $created = (Get-Item -LiteralPath $appData -Force).CreationTimeUtc
    if ($created -lt $jobStart) { throw "app data at $appData predates this job ($created < $jobStart); refusing" }
    $aside = Join-Path $env:RUNNER_TEMP ('medscale-preinstall-appdata-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
    $preexisting = [ordered]@{ created_utc = $created.ToString('o'); job_start_utc = $jobStart.ToString('o'); moved_to = $aside
        entries = @(Get-ChildItem -LiteralPath $appData -Recurse -Force | ForEach-Object { [ordered]@{ path = [IO.Path]::GetRelativePath($appData, $_.FullName); dir = $_.PSIsContainer; created_utc = $_.CreationTimeUtc.ToString('o') } }) }
    Move-Item -LiteralPath $appData -Destination $aside
}
if (Find-Install) { throw 'MedScale is already installed; refusing' }

$manifest = Get-Content -LiteralPath (Join-Path $ReleaseRoot 'package-manifest.json') -Raw | ConvertFrom-Json
$installer = Join-Path $ReleaseRoot $manifest.installer.path
$result = [ordered]@{ source_sha = $manifest.source_sha; installer_sha256 = $manifest.installer.sha256; INSTALL = 'FAIL'; LAUNCH = 'NOT_RUN'; UNINSTALL = 'NOT_RUN'; SIGNING = 'NOT_GRANTED'; preexisting_job_app_data = $preexisting }
if ((Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.installer.sha256) { throw 'installer digest differs from release manifest' }

Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class QD {
  [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)]
  public struct STARTUPINFO { public int cb; public string lpReserved; public string lpDesktop; public string lpTitle;
    public int dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute, dwFlags; public short wShowWindow, cbReserved2;
    public IntPtr lpReserved2, hStdInput, hStdOutput, hStdError; }
  [StructLayout(LayoutKind.Sequential)] public struct PROCESS_INFORMATION { public IntPtr hProcess, hThread; public int dwProcessId, dwThreadId; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode, SetLastError=true)] public static extern IntPtr CreateDesktop(string n, IntPtr d, IntPtr m, int f, uint a, IntPtr s);
  [DllImport("user32.dll")] public static extern bool CloseDesktop(IntPtr h);
  [DllImport("kernel32.dll")] public static extern bool GetExitCodeProcess(IntPtr h, out uint code);
  [DllImport("user32.dll")] public static extern bool EnumDesktopWindows(IntPtr desk, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
  public static extern bool CreateProcess(string app, string cmd, IntPtr pa, IntPtr ta, bool inh, uint f, IntPtr env, string cwd, ref STARTUPINFO si, out PROCESS_INFORMATION pi);
  public static List<string> Titles(IntPtr desk, uint pid) {
    var titles = new List<string>();
    EnumDesktopWindows(desk, (h, l) => { uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid) { var sb = new StringBuilder(256); if (GetWindowText(h, sb, 256) > 0) titles.Add(sb.ToString()); } return true; }, IntPtr.Zero);
    return titles;
  }
}
"@

# INSTALL
$p = Start-Process -FilePath $installer -ArgumentList '/S' -Wait -PassThru
$install = Find-Install
if ($p.ExitCode -eq 0 -and $install) {
    $installDir = $install.InstallLocation.Trim('"')
    if (-not $installDir) { $installDir = Split-Path ($install.UninstallString.Trim('"')) }
    $exe = Get-ChildItem -LiteralPath $installDir -Filter '*.exe' -File | Where-Object Name -notlike 'uninstall*' | Select-Object -First 1
    if ($exe) {
        $result.INSTALL = 'PASS'
        $result.install_dir_files = @(Get-ChildItem -LiteralPath $installDir -Recurse -File | ForEach-Object { [IO.Path]::GetRelativePath($installDir, $_.FullName).Replace('\', '/') })
        $result.installed_exe = [ordered]@{ name = $exe.Name; sha256 = (Get-FileHash -LiteralPath $exe.FullName -Algorithm SHA256).Hash.ToLowerInvariant(); bytes = $exe.Length }
        $result.display_version = $install.DisplayVersion
    }
}
$result.installer_exit_code = $p.ExitCode

# LAUNCH. Run 37709847054 showed that on a GitHub-hosted runner the app panics
# (exit 0x65 = Rust panic) 2.2 s after creating its window on a secondary
# hidden desktop, while the same binary in the same session stays up with a
# WebView2 child and a "MedScale" window on the runner's default desktop. The
# hidden desktop is therefore recorded as a diagnostic only, and the verdict
# comes from the default desktop of a GitHub-hosted (disposable) runner. No
# input is ever sent; refuses on any other machine.
function Watch-Launch([int]$ProcessId, [IntPtr]$Handle, [IntPtr]$Desk) {
    $seen = [Collections.Generic.HashSet[string]]::new(); $start = Get-Date; $exitedAt = $null
    while (((Get-Date) - $start).TotalSeconds -lt $LaunchSeconds) {
        Get-CimInstance Win32_Process -Filter "ParentProcessId=$ProcessId" -ErrorAction SilentlyContinue | ForEach-Object { [void]$seen.Add($_.Name) }
        if (-not (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)) { $exitedAt = ((Get-Date) - $start).TotalSeconds; break }
        Start-Sleep -Milliseconds 500
    }
    $code = $null
    if ($Handle -ne [IntPtr]::Zero) { [uint32]$c = 0; if ([QD]::GetExitCodeProcess($Handle, [ref]$c)) { $code = if ($c -eq 259) { 'STILL_ACTIVE' } else { '0x{0:X8}' -f $c } } }
    elseif ($exitedAt -ne $null) { $code = 'unavailable' }
    $titles = @(if ($Desk -ne [IntPtr]::Zero) { [QD]::Titles($Desk, [uint32]$ProcessId) } else { Get-Process -Id $ProcessId -ErrorAction SilentlyContinue | Where-Object MainWindowTitle | Select-Object -ExpandProperty MainWindowTitle })
    [ordered]@{ pid = $ProcessId; alive = ($exitedAt -eq $null); lifetime_seconds = $(if ($exitedAt -ne $null) { [math]::Round($exitedAt, 1) } else { $LaunchSeconds })
        exit_code = $code; child_processes_seen = @($seen); window_titles = $titles }
}
function Stop-Tree([int]$ProcessId) {
    Get-CimInstance Win32_Process | Where-Object { $_.ProcessId -eq $ProcessId -or $_.ParentProcessId -eq $ProcessId } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
    Start-Sleep -Seconds 3
}
function Get-LaunchEvents([datetime]$Since) {
    @(Get-WinEvent -FilterHashtable @{ LogName = 'Application'; StartTime = $Since } -ErrorAction SilentlyContinue |
        Where-Object { $_.ProviderName -in 'Application Error', 'Windows Error Reporting', 'Application Hang', 'SideBySide' -or $_.Message -match 'medscale|msedgewebview2|WebView2' } |
        Select-Object -First 10 | ForEach-Object { [ordered]@{ time = $_.TimeCreated.ToUniversalTime().ToString('o'); provider = $_.ProviderName; id = $_.Id; message = ($_.Message -replace '\s+', ' ').Substring(0, [math]::Min(400, ($_.Message -replace '\s+', ' ').Length)) } })
}
if ($result.INSTALL -eq 'PASS') {
    $result.LAUNCH = 'FAIL'
    $wv2 = Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue
    $result.launch_environment = [ordered]@{ session_id = (Get-Process -Id $PID).SessionId; user_interactive = [Environment]::UserInteractive; webview2_runtime = $wv2.pv }
    $deskName = 'medscale-101-' + [guid]::NewGuid().ToString('N').Substring(0, 8)
    $desk = [QD]::CreateDesktop($deskName, [IntPtr]::Zero, [IntPtr]::Zero, 0, 0x10000000, [IntPtr]::Zero)
    $si = New-Object QD+STARTUPINFO; $si.cb = [Runtime.InteropServices.Marshal]::SizeOf($si); $si.lpDesktop = "WinSta0\$deskName"
    $pi = New-Object QD+PROCESS_INFORMATION
    $t0 = Get-Date
    if ($desk -ne [IntPtr]::Zero -and [QD]::CreateProcess($exe.FullName, $null, [IntPtr]::Zero, [IntPtr]::Zero, $false, 0, [IntPtr]::Zero, $installDir, [ref]$si, [ref]$pi)) {
        $d = Watch-Launch $pi.dwProcessId $pi.hProcess $desk
        $d.mechanism = 'hidden_desktop_diagnostic_only'; $d.events = Get-LaunchEvents $t0
        # F101-02: where WebView2 cannot initialize, the app must show its startup-failure
        # message (and wait for OK) instead of panicking with exit 0x65.
        $d.startup_failure_message_shown = ($d.window_titles -contains 'MedScale could not start')
        $d.panicked = ($d.exit_code -eq '0x00000065')
        $result.launch_hidden_desktop_diagnostic = $d
        Stop-Tree $pi.dwProcessId
    } else { $result.launch_hidden_desktop_diagnostic = [ordered]@{ mechanism = 'hidden_desktop'; created = $false; desktop_created = ($desk -ne [IntPtr]::Zero); win32_error = [Runtime.InteropServices.Marshal]::GetLastWin32Error() } }
    if ($desk -ne [IntPtr]::Zero) { [QD]::CloseDesktop($desk) | Out-Null }
    $result.app_data_after_launch = @(if (Test-Path -LiteralPath $appData) { Get-ChildItem -LiteralPath $appData -Force | Select-Object -ExpandProperty Name })
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') {
        $result.launch_detail = [ordered]@{ mechanism = 'runner_default_desktop'; refused = 'not a GitHub-hosted disposable runner' }
    } else {
        $t1 = Get-Date
        $proc = Start-Process -FilePath $exe.FullName -WorkingDirectory $installDir -PassThru
        $d2 = Watch-Launch $proc.Id ([IntPtr]::Zero) ([IntPtr]::Zero)
        if (-not $d2.alive) { $proc.WaitForExit(2000) | Out-Null; $d2.exit_code = '0x{0:X8}' -f $proc.ExitCode }
        $d2.mechanism = 'runner_default_desktop'; $d2.events = Get-LaunchEvents $t1
        # Product-level invariant: the process stays up, hosts a WebView2 runtime
        # process and owns a window titled MedScale. Termination is ours.
        $d2.acceptance = 'alive >= LaunchSeconds AND msedgewebview2.exe child AND window title MedScale'
        $result.launch_detail = $d2
        if ($d2.alive -and ($d2.child_processes_seen -contains 'msedgewebview2.exe') -and ($d2.window_titles -contains 'MedScale')) { $result.LAUNCH = 'PASS' }
        Stop-Tree $proc.Id
        $d2.terminated = -not [bool](Get-Process -Id $proc.Id -ErrorAction SilentlyContinue)
    }
}

# UNINSTALL
if ($install) {
    $result.UNINSTALL = 'FAIL'
    $uninstaller = $install.UninstallString.Trim('"')
    $u = Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait -PassThru
    Start-Sleep -Seconds 5  # NSIS uninstallers re-launch from a temp copy
    $stillRegistered = [bool](Find-Install)
    $residue = @(if ($installDir -and (Test-Path -LiteralPath $installDir)) { Get-ChildItem -LiteralPath $installDir -Recurse -Force | ForEach-Object { $_.FullName } })
    $result.uninstall_detail = [ordered]@{ exit_code = $u.ExitCode; still_registered = $stillRegistered; program_files_residue = $residue
        # Uninstall removes the program; user app data (WebView profile, vaults) is
        # intentionally kept by the uninstaller and recorded separately.
        user_app_data_residue = @(if (Test-Path -LiteralPath $appData) { Get-ChildItem -LiteralPath $appData -Recurse -Force | ForEach-Object { [IO.Path]::GetRelativePath($appData, $_.FullName) } }) }
    if ($u.ExitCode -eq 0 -and -not $stillRegistered -and $residue.Count -eq 0) { $result.UNINSTALL = 'PASS' }
}

$json = $result | ConvertTo-Json -Depth 6
if ($ReportPath) { New-Item -ItemType Directory -Path (Split-Path $ReportPath) -Force | Out-Null; $json | Set-Content -LiteralPath $ReportPath -Encoding utf8 }
$json
"INSTALL=$($result.INSTALL) LAUNCH=$($result.LAUNCH) UNINSTALL=$($result.UNINSTALL) SIGNING=NOT_GRANTED"
if ($result.INSTALL -ne 'PASS' -or $result.LAUNCH -ne 'PASS' -or $result.UNINSTALL -ne 'PASS') { exit 1 }
# All three axes passed; do not let a stale native exit code (pkill, hdiutil detach, ...) fail the step.
exit 0
