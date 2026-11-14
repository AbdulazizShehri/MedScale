# F096-T05 measurement aid. Launches the MedScale exe on a separate, never-shown Windows desktop so it
# cannot receive the user's keyboard/mouse input, observes outbound network
# activity of its process tree, then terminates it. No UI interaction occurs.
param(
  [Parameter(Mandatory = $true)][string]$Exe,
  [string]$ExtraBrowserArgs = "",
  [int]$Seconds = 25,
  [string]$Label = "baseline"
)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class HD {
  [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)]
  public struct STARTUPINFO { public int cb; public string lpReserved; public string lpDesktop; public string lpTitle;
    public int dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute, dwFlags; public short wShowWindow, cbReserved2;
    public IntPtr lpReserved2, hStdInput, hStdOutput, hStdError; }
  [StructLayout(LayoutKind.Sequential)]
  public struct PROCESS_INFORMATION { public IntPtr hProcess, hThread; public int dwProcessId, dwThreadId; }
  [DllImport("user32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
  public static extern IntPtr CreateDesktop(string name, IntPtr dev, IntPtr mode, int flags, uint access, IntPtr sa);
  [DllImport("user32.dll", SetLastError=true)] public static extern bool CloseDesktop(IntPtr h);
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
  public static extern bool CreateProcess(string app, string cmd, IntPtr pa, IntPtr ta, bool inherit, uint flags, IntPtr env, string cwd, ref STARTUPINFO si, out PROCESS_INFORMATION pi);
}
"@
$deskName = "medscale-qual-" + [guid]::NewGuid().ToString("N").Substring(0, 8)
$GENERIC_ALL = 0x10000000
$desk = [HD]::CreateDesktop($deskName, [IntPtr]::Zero, [IntPtr]::Zero, 0, $GENERIC_ALL, [IntPtr]::Zero)
if ($desk -eq [IntPtr]::Zero) { throw "CreateDesktop failed" }
if ($ExtraBrowserArgs) { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $ExtraBrowserArgs } else { Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue }
$si = New-Object HD+STARTUPINFO
$si.cb = [Runtime.InteropServices.Marshal]::SizeOf($si)
$si.lpDesktop = "WinSta0\$deskName"
$pi = New-Object HD+PROCESS_INFORMATION
if (-not [HD]::CreateProcess($Exe, $null, [IntPtr]::Zero, [IntPtr]::Zero, $false, 0, [IntPtr]::Zero, (Split-Path $Exe), [ref]$si, [ref]$pi)) { throw "CreateProcess failed" }
$root = $pi.dwProcessId
$seen = @{}
$deadline = (Get-Date).AddSeconds($Seconds)
while ((Get-Date) -lt $deadline) {
  $tree = @($root)
  for ($i = 0; $i -lt 3; $i++) { $tree += @(Get-CimInstance Win32_Process | Where-Object { $tree -contains $_.ParentProcessId } | Select-Object -ExpandProperty ProcessId) ; $tree = $tree | Select-Object -Unique }
  foreach ($c in @(Get-NetTCPConnection -ErrorAction SilentlyContinue | Where-Object { $tree -contains $_.OwningProcess -and $_.RemoteAddress -notmatch '^(127\.|::1$|0\.0\.0\.0$|::$)' })) {
    $cmd = (Get-CimInstance Win32_Process -Filter "ProcessId=$($c.OwningProcess)").CommandLine
    $role = if ($c.OwningProcess -eq $root) { "medscale-host" } elseif ($cmd -match '--type=([a-z-]+)') { "webview2-" + $Matches[1] } else { "webview2-browser" }
    $k = "$role|$($c.RemoteAddress):$($c.RemotePort)"
    if (-not $seen.ContainsKey($k)) { $seen[$k] = $c.State }
  }
  Start-Sleep -Milliseconds 700
}
$dns = Get-DnsClientCache -ErrorAction SilentlyContinue | Where-Object { $_.Type -in 1, 28 }
"[$Label] extra browser args: '$ExtraBrowserArgs'"
if ($seen.Count -eq 0) { "[$Label] no non-loopback TCP connections from the process tree" }
foreach ($k in $seen.Keys | Sort-Object) {
  $role, $ep = $k -split '\|'
  $ip = ($ep -replace ':\d+$', '').Trim('[', ']')
  $names = ($dns | Where-Object { $_.Data -eq $ip } | Select-Object -ExpandProperty Entry -Unique) -join ','
  "[$Label] $role -> $ep  dns=$names"
}
"[$Label] DNS names resolved during run: " + (($dns | Select-Object -ExpandProperty Entry -Unique | Sort-Object) -join ', ')
Get-CimInstance Win32_Process | Where-Object { $_.ProcessId -eq $root -or $_.ParentProcessId -eq $root } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
Start-Sleep -Seconds 2
Get-Process msedgewebview2 -ErrorAction SilentlyContinue | Where-Object { (Get-CimInstance Win32_Process -Filter "ProcessId=$($_.Id)").CommandLine -match 'org.medscale.desktop.preview' } | Stop-Process -Force -ErrorAction SilentlyContinue
[HD]::CloseDesktop($desk) | Out-Null
"[$Label] stopped; medscale running: " + [bool](Get-Process medscale-desktop-tauri -ErrorAction SilentlyContinue)
