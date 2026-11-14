# Spec 050 — run host-bound perf harness measurement path (does NOT claim budgets).
#
# Usage:
#   pwsh ./scripts/run-host-perf-measurement.ps1
#   pwsh ./scripts/run-host-perf-measurement.ps1 -TimelineEvents 10000 -LexicalScale
#
# Defaults match CI-feasible delivery-plan scale (1000 events) unless overridden.

[CmdletBinding()]
param(
    [int]$TimelineEvents = 1000,
    [switch]$LexicalScale,
    [int]$TimedRuns = 8,
    [string]$EvidenceDir = "evidence/050-host-perf-measurement"
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$env:MEDSCALE_027_DELIVERY_PLAN_SCALE = '1'
$env:MEDSCALE_027_TIMELINE_EVENTS = "$TimelineEvents"
$env:MEDSCALE_027_TIMED_RUNS = "$TimedRuns"
if ($LexicalScale) {
    $env:MEDSCALE_027_LEXICAL_SCALE = '10000'
} else {
    Remove-Item Env:MEDSCALE_027_LEXICAL_SCALE -ErrorAction SilentlyContinue
}

$sourceSha = (git -C $repoRoot rev-parse HEAD).Trim()
$treeSha = (git -C $repoRoot log -1 --format=%T).Trim()
$lockHash = (Get-FileHash -Algorithm SHA256 -Path (Join-Path $repoRoot 'Cargo.lock')).Hash.ToLowerInvariant()
$rustc = (rustc --version).Trim()
$os = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
$cpu = $env:PROCESSOR_IDENTIFIER
if (-not $cpu) { $cpu = $env:PROCESSOR_ARCHITECTURE }
$ram = ''
try {
    $cs = Get-CimInstance Win32_ComputerSystem -ErrorAction SilentlyContinue
    if ($cs) { $ram = [string]([math]::Round($cs.TotalPhysicalMemory / 1MB)) + ' MiB' }
} catch { $ram = '' }

$evidenceFull = Join-Path $repoRoot $EvidenceDir
if (-not (Test-Path $evidenceFull)) {
    New-Item -ItemType Directory -Force -Path $evidenceFull | Out-Null
}

$sidecar = [ordered]@{
    schema_version = 1
    spec_id = '050-host-perf-measurement'
    generated_at = (Get-Date).ToUniversalTime().ToString('o')
    budgets_claimed_met = $false
    release_ready = $false
    scale = [ordered]@{
        timeline_events = $TimelineEvents
        lexical_scale = [bool]$LexicalScale
        timed_runs = $TimedRuns
        delivery_plan_scale = $true
    }
    binding = [ordered]@{
        source_sha = $sourceSha
        tree_sha = $treeSha
        cargo_lock_sha256 = $lockHash
        rustc = $rustc
        os = "$os"
        arch = "$arch"
        cpu = "$cpu"
        ram = "$ram"
    }
    note = 'Host measurement path only. Do not claim budgets_claimed_met or RELEASE_READY from this sidecar alone. Harness JSON is written under evidence/027 or evidence/042/045 paths by the test.'
}

$sidecarPath = Join-Path $evidenceFull 'host_perf_binding_sidecar.json'
($sidecar | ConvertTo-Json -Depth 6) | Set-Content -Path $sidecarPath -Encoding utf8
Write-Host "Wrote $sidecarPath"

Write-Host "Running perf harness (delivery-plan scale; timeline=$TimelineEvents lexical=$LexicalScale)..."
if (Test-Path 'C:\Strawberry\perl\bin') {
    $env:Path = "C:\Strawberry\perl\bin;$env:Path"
}
cargo test -p medscale-core --test perf_harness_027 --locked -- --nocapture
if ($LASTEXITCODE -ne 0) {
    Write-Host "HOST PERF MEASUREMENT FAILED"
    exit $LASTEXITCODE
}

Write-Host "Running Spec 057 desktop launch + idle RSS coverage harness..."
cargo test -p medscale-desktop --test runtime_perf_057 --locked -- --nocapture
if ($LASTEXITCODE -ne 0) {
    Write-Host "HOST RUNTIME PERF COVERAGE FAILED"
    exit $LASTEXITCODE
}

Write-Host "HOST PERF MEASUREMENT PATH OK"
Write-Host "LIMITATION: budgets_claimed_met remains false; RELEASE_READY remains false."
exit 0
