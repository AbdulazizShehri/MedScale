# Spec 101 T101-05 negative tests for scripts/verify-tauri-release.ps1.
# Builds a small synthetic release set (no real installer, no product data),
# checks it verifies, then checks every tampering case fails closed.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$verifier = Join-Path $PSScriptRoot 'verify-tauri-release.ps1'
$base = Join-Path ([IO.Path]::GetTempPath()) ("medscale-101-verify-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
function Sha([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }

function New-Fixture([string]$Dir) {
    $src = 'a' * 40
    New-Item -ItemType Directory -Path (Join-Path $Dir 'installer') -Force | Out-Null
    [IO.File]::WriteAllBytes((Join-Path $Dir 'installer/MedScale_0.1.0_x64-setup.exe'), [byte[]](1..64))
    Set-Content -LiteralPath (Join-Path $Dir 'LICENSE') -Value 'Apache-2.0 fixture' -Encoding utf8
    Set-Content -LiteralPath (Join-Path $Dir 'NOTICE.md') -Value 'notice fixture' -Encoding utf8
    @{ metadata = @{ properties = @(@{ name = 'medscale:source_sha'; value = $src }) } } | ConvertTo-Json -Depth 5 |
        Set-Content -LiteralPath (Join-Path $Dir 'SBOM.cdx.json') -Encoding utf8
    $payload = @(Get-ChildItem -LiteralPath $Dir -Recurse -File | ForEach-Object {
        @{ path = [IO.Path]::GetRelativePath($Dir, $_.FullName).Replace('\', '/'); bytes = $_.Length; sha256 = Sha $_.FullName } })
    [ordered]@{
        schema = 'medscale.tauri-release-manifest.v1'; source_sha = $src; tree_sha = 'b' * 40
        installer = @{ path = 'installer/MedScale_0.1.0_x64-setup.exe'; bytes = 64; sha256 = Sha (Join-Path $Dir 'installer/MedScale_0.1.0_x64-setup.exe') }
        sbom_sha256 = Sha (Join-Path $Dir 'SBOM.cdx.json'); notice_sha256 = Sha (Join-Path $Dir 'NOTICE.md')
        payload = $payload; signed = $false; notarized = $false; release_ready = $false; reproducible_binary_build = $false; signing = 'NOT_GRANTED'
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $Dir 'package-manifest.json') -Encoding utf8
    Update-Sums $Dir
}
function Update-Sums([string]$Dir) {
    $lines = @(Get-ChildItem -LiteralPath $Dir -Recurse -File | Where-Object Name -ne 'SHA256SUMS' | Sort-Object FullName | ForEach-Object {
        '{0}  {1}' -f (Sha $_.FullName), [IO.Path]::GetRelativePath($Dir, $_.FullName).Replace('\', '/') })
    $lines -join "`n" | Set-Content -LiteralPath (Join-Path $Dir 'SHA256SUMS') -Encoding utf8 -NoNewline
}
function Invoke-Case([string]$Name, [scriptblock]$Mutate, [bool]$ExpectPass, [string]$Expected = '') {
    $dir = Join-Path $base $Name
    New-Fixture $dir
    & $Mutate $dir
    $out = & pwsh -NoProfile -File $verifier -Root $dir -ExpectedSourceSha $Expected 2>&1
    $passed = $LASTEXITCODE -eq 0
    if ($passed -ne $ExpectPass) { throw "case '$Name' expected $(if ($ExpectPass) {'PASS'} else {'FAIL'}), got: $($out -join ' | ')" }
    Write-Output ("ok   {0,-28} {1}" -f $Name, ($out | Select-Object -Last 1))
}

try {
    Invoke-Case 'clean' { param($d) } $true
    Invoke-Case 'clean-expected-sha' { param($d) } $true ('a' * 40)
    Invoke-Case 'tampered-installer' { param($d) $f = Join-Path $d 'installer/MedScale_0.1.0_x64-setup.exe'; $b = [IO.File]::ReadAllBytes($f); $b[0] = 0xFF; [IO.File]::WriteAllBytes($f, $b) } $false
    Invoke-Case 'tampered-and-resummed' { param($d) $f = Join-Path $d 'installer/MedScale_0.1.0_x64-setup.exe'; $b = [IO.File]::ReadAllBytes($f); $b[0] = 0xFF; [IO.File]::WriteAllBytes($f, $b); Update-Sums $d } $false
    Invoke-Case 'extra-file' { param($d) Set-Content -LiteralPath (Join-Path $d 'extra.txt') -Value 'x' } $false
    Invoke-Case 'missing-notice' { param($d) Remove-Item -LiteralPath (Join-Path $d 'NOTICE.md') } $false
    Invoke-Case 'release-ready-flag' { param($d) $m = Get-Content (Join-Path $d 'package-manifest.json') -Raw | ConvertFrom-Json; $m.release_ready = $true; $m | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $d 'package-manifest.json'); Update-Sums $d } $false
    Invoke-Case 'wrong-expected-sha' { param($d) } $false ('c' * 40)
    Write-Output 'TAURI_RELEASE_VERIFY_NEGATIVE_TESTS=PASS'
}
finally { Remove-Item -LiteralPath $base -Recurse -Force -ErrorAction SilentlyContinue }
# Every case above passed (a mismatch throws). The expected-failure child runs
# leave $LASTEXITCODE=1, and the GitHub pwsh wrapper exits with it, so reset it
# explicitly. Reached only when all assertions held.
exit 0
