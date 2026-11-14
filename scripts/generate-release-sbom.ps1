# Generate the deterministic Spec 054 CycloneDX release SBOM from live source
# state (Spec 054). Binds source/tree/Cargo.lock SHAs + workspace + Rust +
# native inventories + artifacts + Pack assets. This is READY_BASE evidence:
# signing/provenance/installer qualification remain external. Apache-2.0 is the
# founder-selected public source license. Prefer re-running from repo root.
#
# Usage:
#   pwsh ./scripts/generate-release-sbom.ps1
#   pwsh ./scripts/generate-release-sbom.ps1 -OutPath evidence/054-release-sbom/SBOM.cdx.json

[CmdletBinding()]
param(
    [string]$OutPath = "evidence/054-release-sbom/SBOM.cdx.json"
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

function Resolve-RepoPath([string]$Path) {
    if ([IO.Path]::IsPathRooted($Path)) { return [IO.Path]::GetFullPath($Path) }
    return [IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
}

$sourceSha = (git -C $repoRoot rev-parse HEAD).Trim()
$treeSha = (git -C $repoRoot rev-parse 'HEAD^{tree}').Trim()
$lockHash = (Get-FileHash -Algorithm SHA256 -Path (Join-Path $repoRoot 'Cargo.lock')).Hash.ToLowerInvariant()
$rustc = (rustc --version).Trim()
$cargo = (cargo --version).Trim()
$buildOs = if ($IsWindows) { 'windows' } elseif ($IsMacOS) { 'macos' } elseif ($IsLinux) { 'linux' } else { 'unknown' }
$osVersion = [System.Environment]::OSVersion.VersionString
if ($IsWindows) {
    try {
        $caption = Get-CimInstance Win32_OperatingSystem -ErrorAction Stop | Select-Object -ExpandProperty Caption
        if ($caption) { $osVersion = [string]$caption }
    } catch { }
}
$hostLine = rustc -vV | Where-Object { $_ -match '^host:' } | Select-Object -First 1
$targetTriple = ([string]$hostLine -replace '^host:\s*', '').Trim()
if ([string]::IsNullOrWhiteSpace($targetTriple)) { throw 'rustc host target triple unavailable' }

$metadata = (cargo metadata --format-version 1 --locked | Out-String) | ConvertFrom-Json
$workspaceIds = @($metadata.workspace_members)

$workspacePackages = @()
$rustDeps = @()
foreach ($pkg in ($metadata.packages | Sort-Object { "$($_.name)@$($_.version)" })) {
    $isWorkspace = $workspaceIds -contains $pkg.id
    if ($isWorkspace) {
        $workspacePackages += $pkg
    } else {
        $rustDeps += $pkg
    }
}

function Convert-License($raw) {
    if ([string]::IsNullOrWhiteSpace($raw)) { return @{ license = @{ name = 'NOASSERTION' } } }
    $t = $raw.Trim()
    if ($t -eq 'UNLICENSED') { return @{ license = @{ name = 'UNLICENSED-workspace-no-public-license' } } }
    if ($t -eq 'NOASSERTION') { return @{ license = @{ name = 'NOASSERTION' } } }
    return @{ license = @{ id = $t } }
}

$components = [System.Collections.ArrayList]::new()
foreach ($pkg in $workspacePackages) {
    $lic = $pkg.license
    if (-not $lic) { $lic = 'UNLICENSED' }
    [void]$components.Add([ordered]@{
        type = 'application'
        'bom-ref' = "pkg:cargo/$($pkg.name)@$($pkg.version)"
        name = $pkg.name
        version = $pkg.version
        purl = "pkg:cargo/$($pkg.name)@$($pkg.version)"
        licenses = @((Convert-License $lic))
        scope = 'required'
        properties = @(@{ name = 'medscale:package_scope'; value = 'workspace' })
    })
}
foreach ($pkg in $rustDeps) {
    $ids = @()
    if ($pkg.license) { $ids = @($pkg.license -split '\s+OR\s+|\s+/\s+' | ForEach-Object { $_.Trim() } | Where-Object { $_ }) }
    $licenses = @()
    if ($ids.Count -eq 0) { $licenses = @(@{ license = @{ name = 'NOASSERTION' } }) }
    else { foreach ($id in $ids) { $licenses += (Convert-License $id) } }
    [void]$components.Add([ordered]@{
        type = 'library'
        'bom-ref' = "pkg:cargo/$($pkg.name)@$($pkg.version)"
        name = $pkg.name
        version = $pkg.version
        purl = "pkg:cargo/$($pkg.name)@$($pkg.version)"
        licenses = $licenses
        scope = 'required'
    })
}

$nativeInventoryPath = 'evidence/047-release-dry-run-verifier/native-deps-inventory.json'
$nativeInventory = Get-Content -Raw -Path (Join-Path $repoRoot $nativeInventoryPath) | ConvertFrom-Json
$nativeList = @()
if ($nativeInventory.native_dependencies) { $nativeList = @($nativeInventory.native_dependencies) }
elseif ($nativeInventory.admitted_native_components) { $nativeList = @($nativeInventory.admitted_native_components) }
elseif ($nativeInventory -is [array]) { $nativeList = @($nativeInventory) }
foreach ($dep in ($nativeList | Sort-Object { "$($_.name)$($_.id)" })) {
    $depName = "$($dep.name)$($dep.id)"
    $classification = "$($dep.classification)"
    if ([string]::IsNullOrWhiteSpace($classification)) {
        switch ("$($dep.kind)") {
            'embedded_native' { $classification = 'bundled-source' }
            'os_ipc' { $classification = 'os-provided' }
            default { $classification = 'os-provided' }
        }
    }
    $detail = "$($dep.detail)$($dep.measured_note) $($dep.admission)".Trim()
    [void]$components.Add([ordered]@{
        type = 'library'
        'bom-ref' = "pkg:medscale/native/$depName"
        name = $depName
        licenses = @(@{ license = @{ name = 'NOASSERTION' } })
        scope = 'required'
        properties = @(
            @{ name = 'medscale:native_classification'; value = $classification }
            @{ name = 'medscale:native_detail'; value = $detail }
        )
    })
}
# Toolchain / host tools that are not cargo crates: explicitly classified.
foreach ($tool in @(@{ name = 'rustc'; detail = $rustc }, @{ name = 'cargo'; detail = $cargo })) {
    [void]$components.Add([ordered]@{
        type = 'library'
        'bom-ref' = "pkg:medscale/native/$($tool.name)"
        name = $tool.name
        licenses = @(@{ license = @{ name = 'NOASSERTION' } })
        scope = 'required'
        properties = @(
            @{ name = 'medscale:native_classification'; value = 'host-tool' }
            @{ name = 'medscale:native_detail'; value = $tool.detail }
        )
    })
}

# Artifact inventory: release-manifest scaffold + checksums evidence when present.
$artifactNames = @('scripts/release-dry-run.ps1', 'scripts/verify-release-manifest.ps1', 'scripts/generate-release-sbom.ps1')
foreach ($rel in $artifactNames) {
    $full = Join-Path $repoRoot $rel
    if (Test-Path $full) {
        $h = (Get-FileHash -Algorithm SHA256 -Path $full).Hash.ToLowerInvariant()
        [void]$components.Add([ordered]@{
            type = 'file'
            'bom-ref' = "pkg:medscale/artifact/$rel"
            name = $rel
            licenses = @(@{ license = @{ name = 'NOASSERTION' } })
            hashes = @(@{ alg = 'SHA-256'; content = $h })
        })
    }
}

# Pack / model assets: offline synthetic Pack v0 only; no third-party weights.
[void]$components.Add([ordered]@{
    type = 'data'
    'bom-ref' = 'pkg:medscale/pack-asset/offline-pack-v0'
    name = 'offline-pack-v0'
    licenses = @(@{ license = @{ name = 'NOASSERTION' } })
    properties = @(
        @{ name = 'medscale:pack_asset_kind'; value = 'offline-pack' }
        @{ name = 'medscale:rights_note'; value = 'synthetic fixture only; no third-party model weights; terminology/model rights external' }
    )
})

$sorted = @($components | Sort-Object { $_['bom-ref'] })
$shortTree = $treeSha.Substring(0, [Math]::Min(12, $treeSha.Length))
$bom = [ordered]@{
    bomFormat = 'CycloneDX'
    specVersion = '1.5'
    version = 1
    serialNumber = "urn:uuid:medscale-054-$shortTree"
    metadata = [ordered]@{
        timestamp = '1970-01-01T00:00:00Z'
        tools = @(@{ vendor = 'MedScale'; name = 'generate-release-sbom.ps1'; version = '054' })
        component = [ordered]@{ type = 'application'; name = 'MedScale'; version = 'trusted-v1' }
        properties = @(
            @{ name = 'medscale:sbom_kind'; value = 'release_qualified_cyclonedx15' }
            @{ name = 'medscale:sbom_document_format'; value = 'CycloneDX-1.5 (SBOM format; not the public project license)' }
            @{ name = 'medscale:public_project_license'; value = 'Apache-2.0' }
            @{ name = 'medscale:source_sha'; value = $sourceSha }
            @{ name = 'medscale:tree_sha'; value = $treeSha }
            @{ name = 'medscale:cargo_lock_sha256'; value = $lockHash }
            @{ name = 'medscale:build_os'; value = $buildOs }
            @{ name = 'medscale:build_os_version'; value = "$osVersion" }
            @{ name = 'medscale:target_triple'; value = $targetTriple }
            @{ name = 'medscale:rustc'; value = $rustc }
            @{ name = 'medscale:cargo'; value = $cargo }
            @{ name = 'medscale:reproducible_build'; value = 'unproven' }
            @{ name = 'medscale:release_ready'; value = 'false' }
        )
    }
    components = $sorted
}

$outFull = Resolve-RepoPath $OutPath
$outDir = Split-Path -Parent $outFull
if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Force -Path $outDir | Out-Null }
($bom | ConvertTo-Json -Depth 12) | Set-Content -Path $outFull -Encoding utf8
$docHash = (Get-FileHash -Algorithm SHA256 -Path $outFull).Hash.ToLowerInvariant()
Write-Host "Wrote $outFull ($($sorted.Count) components)"
Write-Host "source=$sourceSha tree=$treeSha lock=$lockHash doc=$docHash"
Write-Host 'READY_BASE only: reproducible_build=unproven; release_ready=false; signing/provenance external.'
