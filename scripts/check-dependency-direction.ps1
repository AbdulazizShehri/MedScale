# Check MedScale crate dependency direction.
# Allowed:
#   cli -> core -> contracts
#   core -> storage -> contracts
#   core -> fhir -> contracts
$ErrorActionPreference = 'Stop'

$metadataJson = cargo metadata --format-version 1 --no-deps | Out-String
$metadata = $metadataJson | ConvertFrom-Json

function Get-Package {
    param([string]$Name)
    $pkg = $metadata.packages | Where-Object { $_.name -eq $Name } | Select-Object -First 1
    if (-not $pkg) {
        throw "Missing package $Name in workspace metadata"
    }
    return $pkg
}

function Test-DependsOn {
    param(
        [Parameter(Mandatory = $true)]$Package,
        [Parameter(Mandatory = $true)][string]$DepName
    )
    return [bool]($Package.dependencies | Where-Object { $_.name -eq $DepName })
}

$contracts = Get-Package 'medscale-contracts'
$core = Get-Package 'medscale-core'
$cli = Get-Package 'medscale-cli'
$desktop = Get-Package 'medscale-desktop'
$keys = Get-Package 'medscale-keys'
$storage = Get-Package 'medscale-storage'
$fhir = Get-Package 'medscale-fhir'
$network = Get-Package 'medscale-network'
$pack = Get-Package 'medscale-pack'

$failures = @()

foreach ($name in @('medscale-contracts')) {
    $pkg = Get-Package $name
    foreach ($forbidden in @('medscale-core', 'medscale-cli', 'medscale-storage', 'medscale-fhir', 'medscale-keys', 'medscale-desktop', 'medscale-network', 'medscale-pack')) {
        if (Test-DependsOn -Package $pkg -DepName $forbidden) {
            $failures += "$name must not depend on $forbidden"
        }
    }
}

if (Test-DependsOn -Package $core -DepName 'medscale-cli') {
    $failures += 'medscale-core must not depend on medscale-cli'
}
if (Test-DependsOn -Package $core -DepName 'medscale-desktop') {
    $failures += 'medscale-core must not depend on medscale-desktop'
}
if (-not (Test-DependsOn -Package $core -DepName 'medscale-contracts')) {
    $failures += 'medscale-core must depend on medscale-contracts'
}
if (-not (Test-DependsOn -Package $core -DepName 'medscale-storage')) {
    $failures += 'medscale-core must depend on medscale-storage'
}
if (-not (Test-DependsOn -Package $core -DepName 'medscale-fhir')) {
    $failures += 'medscale-core must depend on medscale-fhir'
}
if (-not (Test-DependsOn -Package $cli -DepName 'medscale-core')) {
    $failures += 'medscale-cli must depend on medscale-core'
}
if (Test-DependsOn -Package $cli -DepName 'medscale-storage') {
    $failures += 'medscale-cli must not depend on medscale-storage (facade-only)'
}
if (Test-DependsOn -Package $cli -DepName 'rusqlite') {
    $failures += 'medscale-cli must not depend on rusqlite'
}
if (-not (Test-DependsOn -Package $desktop -DepName 'medscale-core')) {
    $failures += 'medscale-desktop must depend on medscale-core'
}
if (Test-DependsOn -Package $desktop -DepName 'tauri') {
    $failures += 'medscale-desktop must not depend on tauri in Spec 006'
}
if (-not (Test-DependsOn -Package $storage -DepName 'medscale-contracts')) {
    $failures += 'medscale-storage must depend on medscale-contracts'
}
if (-not (Test-DependsOn -Package $storage -DepName 'medscale-keys')) {
    $failures += 'medscale-storage must depend on medscale-keys'
}
if (-not (Test-DependsOn -Package $fhir -DepName 'medscale-contracts')) {
    $failures += 'medscale-fhir must depend on medscale-contracts'
}
if (-not (Test-DependsOn -Package $keys -DepName 'medscale-contracts')) {
    $failures += 'medscale-keys must depend on medscale-contracts'
}
if (Test-DependsOn -Package $storage -DepName 'medscale-core') {
    $failures += 'medscale-storage must not depend on medscale-core'
}
if (Test-DependsOn -Package $fhir -DepName 'medscale-core') {
    $failures += 'medscale-fhir must not depend on medscale-core'
}
if (Test-DependsOn -Package $keys -DepName 'medscale-core') {
    $failures += 'medscale-keys must not depend on medscale-core'
}
if (Test-DependsOn -Package $keys -DepName 'medscale-storage') {
    $failures += 'medscale-keys must not depend on medscale-storage'
}

if (-not (Test-DependsOn -Package $core -DepName 'medscale-network')) {
    $failures += 'medscale-core must depend on medscale-network'
}
if (-not (Test-DependsOn -Package $network -DepName 'medscale-contracts')) {
    $failures += 'medscale-network must depend on medscale-contracts'
}
if (Test-DependsOn -Package $network -DepName 'medscale-core') {
    $failures += 'medscale-network must not depend on medscale-core'
}
if (Test-DependsOn -Package $cli -DepName 'ureq') {
    $failures += 'medscale-cli must not depend on ureq'
}
if (Test-DependsOn -Package $desktop -DepName 'ureq') {
    $failures += 'medscale-desktop must not depend on ureq'
}

if (-not (Test-DependsOn -Package $core -DepName 'medscale-pack')) {
    $failures += 'medscale-core must depend on medscale-pack'
}
if (-not (Test-DependsOn -Package $pack -DepName 'medscale-contracts')) {
    $failures += 'medscale-pack must depend on medscale-contracts'
}
if (Test-DependsOn -Package $pack -DepName 'medscale-core') {
    $failures += 'medscale-pack must not depend on medscale-core'
}
if (Test-DependsOn -Package $cli -DepName 'medscale-pack') {
    $failures += 'medscale-cli must not depend on medscale-pack (facade-only)'
}
if (Test-DependsOn -Package $desktop -DepName 'medscale-pack') {
    $failures += 'medscale-desktop must not depend on medscale-pack (facade-only)'
}

if ($failures.Count -gt 0) {
    Write-Error ("Dependency direction check failed:`n - " + ($failures -join "`n - "))
    exit 1
}

Write-Host 'Dependency direction check passed (cli/desktop -> core -> {storage,fhir,network,pack} -> {contracts,keys}).'
