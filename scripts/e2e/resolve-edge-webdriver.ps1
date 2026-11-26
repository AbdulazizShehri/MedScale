# Resolves a Microsoft Edge WebDriver executable whose major version matches the
# installed WebView2 Runtime on Windows. GitHub-hosted windows-latest runners
# occasionally ship a newer preinstalled msedgedriver (e.g. 158) alongside an
# older WebView2 Runtime (e.g. 157), causing WebDriver session creation to fail
# with a major-version mismatch. Prints the resolved msedgedriver.exe path on
# stdout; diagnostic lines go to stderr (Write-Host).
$ErrorActionPreference = 'Stop'

$driver = if ($env:EDGEWEBDRIVER) { Join-Path $env:EDGEWEBDRIVER 'msedgedriver.exe' } else { '' }
$driverMajor = $null
if ($driver -and (Test-Path $driver)) {
    $verOut = (& $driver --version) | Out-String
    [Console]::Error.WriteLine("Preinstalled Edge WebDriver: $($verOut.Trim())")
    if ($verOut -match '(\d+)\.\d+\.\d+\.\d+') {
        $driverMajor = $Matches[1]
    }
}

$wv2Ver = $null
foreach ($regPath in @(
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
    'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
    'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
)) {
    $pv = (Get-ItemProperty -Path $regPath -ErrorAction SilentlyContinue).pv
    if ($pv -and $pv -match '^(\d+)\.\d+\.\d+\.\d+$') {
        $wv2Ver = $pv
        break
    }
}

if (-not $wv2Ver) {
    [Console]::Error.WriteLine('Installed WebView2 Runtime version not found in registry; using preinstalled driver.')
    Write-Output $driver
    exit 0
}

[Console]::Error.WriteLine("Installed WebView2 Runtime: $wv2Ver")
$null = $wv2Ver -match '^(\d+)\.'
$wv2Major = $Matches[1]

if ($driverMajor -eq $wv2Major -and (Test-Path $driver)) {
    Write-Output $driver
    exit 0
}

[Console]::Error.WriteLine("Edge WebDriver major ($driverMajor) != WebView2 major ($wv2Major); downloading matching msedgedriver for $wv2Ver.")
$baseTemp = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [System.IO.Path]::GetTempPath() }
$matchDir = Join-Path $baseTemp "msedgedriver-$wv2Ver"
New-Item -ItemType Directory -Force -Path $matchDir | Out-Null
$zipPath = Join-Path $matchDir 'edgedriver.zip'

$downloaded = $false
try {
    Invoke-WebRequest -Uri "https://msedgedriver.microsoft.com/$wv2Ver/edgedriver_win64.zip" -OutFile $zipPath -UseBasicParsing
    $downloaded = $true
} catch {
    [Console]::Error.WriteLine("Exact version $wv2Ver unavailable ($($_.Exception.Message)); querying LATEST_RELEASE_${wv2Major}_WINDOWS.")
}

if (-not $downloaded) {
    $resp = Invoke-WebRequest -Uri "https://msedgedriver.microsoft.com/LATEST_RELEASE_${wv2Major}_WINDOWS" -UseBasicParsing
    $raw = if ($resp.Content -is [byte[]]) {
        [System.Text.Encoding]::Unicode.GetString($resp.Content)
    } else {
        [string]$resp.Content
    }
    $latest = ($raw -replace "[\uFEFF\0]", '').Trim()
    if ($latest -notmatch '^\d+\.\d+\.\d+\.\d+$') {
        throw "Unexpected LATEST_RELEASE_${wv2Major}_WINDOWS content: '$latest'"
    }
    [Console]::Error.WriteLine("Resolved LATEST_RELEASE_${wv2Major}_WINDOWS: $latest")
    Invoke-WebRequest -Uri "https://msedgedriver.microsoft.com/$latest/edgedriver_win64.zip" -OutFile $zipPath -UseBasicParsing
}

Expand-Archive -Path $zipPath -DestinationPath $matchDir -Force
$resolved = Join-Path $matchDir 'msedgedriver.exe'
if (-not (Test-Path $resolved)) {
    throw "msedgedriver.exe missing after extracting $zipPath"
}
$resolvedVer = (& $resolved --version) | Out-String
[Console]::Error.WriteLine("Resolved Edge WebDriver: $($resolvedVer.Trim())")
Write-Output $resolved
