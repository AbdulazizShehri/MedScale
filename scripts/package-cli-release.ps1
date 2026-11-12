# R8: assemble one unsigned engineering archive of the `medscale` CLI for one
# target, after `cargo build --release --locked -p medscale-cli --target <T>`.
# The archive needs no Rust toolchain or runtime: the binary links OpenSSL and
# SQLCipher statically. This signs nothing, publishes nothing and does not
# establish release readiness.
#
# Output: target/cli-release/medscale-<version>-<target>.(zip|tar.gz) plus
# target/cli-release/SHA256SUMS-<target>. The archive holds the binary, LICENSE,
# NOTICE.md, licenses/ (every bundled crate's license and notice files),
# SBOM.cdx.json, evidence/rust-dependencies.json and its own SHA256SUMS.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Target)
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$version = ((Select-String -LiteralPath (Join-Path $repoRoot 'Cargo.toml') -Pattern '^version = "(.+)"$' | Select-Object -First 1).Matches[0].Groups[1].Value)
$name = "medscale-$version-$Target"
$outRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'target/cli-release'))
$stage = Join-Path $outRoot $name
# A restored build cache can carry an earlier run's output; start clean.
if (Test-Path -LiteralPath $outRoot) { Remove-Item -LiteralPath $outRoot -Recurse -Force }
$exeName = if ($Target -like '*windows*') { 'medscale.exe' } else { 'medscale' }
$exe = Join-Path $repoRoot "target/$Target/release/$exeName"
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw "build first: $exe is missing" }

New-Item -ItemType Directory -Path (Join-Path $stage 'licenses'), (Join-Path $stage 'evidence') -Force | Out-Null
Copy-Item -LiteralPath $exe -Destination $stage
Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination $stage

# The CLI's resolved graph for this target: normal and build dependencies only.
$metadata = (& cargo metadata --format-version 1 --locked --filter-platform $Target --manifest-path (Join-Path $repoRoot 'Cargo.toml') | Out-String) | ConvertFrom-Json -Depth 64
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
$packages = @{}; foreach ($p in $metadata.packages) { $packages[$p.id] = $p }
$nodes = @{}; foreach ($n in $metadata.resolve.nodes) { $nodes[$n.id] = $n }
$root = ($metadata.packages | Where-Object { $_.name -eq 'medscale-cli' }).id
$seen = [Collections.Generic.HashSet[string]]::new(); $stack = [Collections.Generic.Stack[string]]::new(); $stack.Push($root)
while ($stack.Count) {
    $id = $stack.Pop(); if (-not $seen.Add($id)) { continue }
    foreach ($d in $nodes[$id].deps) {
        if (@($d.dep_kinds | Where-Object { $null -eq $_.kind -or $_.kind -eq 'build' }).Count) { $stack.Push($d.pkg) }
    }
}
$inventory = @()
foreach ($id in ($seen | Sort-Object)) {
    $p = $packages[$id]
    $inventory += [ordered]@{ name = $p.name; version = $p.version; license = $p.license; source = $p.source }
    if (-not $p.source) { continue }
    $dir = Split-Path -Parent $p.manifest_path
    $files = @(Get-ChildItem -LiteralPath $dir -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT|UNLICENSE)' })
    # Bundled C libraries keep their own license beside their sources.
    if ($p.name -in 'openssl-src', 'libsqlite3-sys') {
        $files += @(Get-ChildItem -LiteralPath $dir -Recurse -Depth 2 -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING)' -and $_.DirectoryName -ne $dir })
    }
    $dest = Join-Path $stage "licenses/$($p.name)-$($p.version)"
    New-Item -ItemType Directory -Path $dest -Force | Out-Null
    if (-not $files.Count) {
        # A few crates publish no license file. Record what the crate declares.
        if (-not $p.license) { throw "no license file or declared license for $($p.name) $($p.version)" }
        @"
$($p.name) $($p.version) ships no license file in its published crate.
Declared license (Cargo.toml): $($p.license)
Authors: $(@($p.authors) -join ', ')
Repository: $($p.repository)
"@ | Set-Content -LiteralPath (Join-Path $dest 'DECLARED-LICENSE.txt') -Encoding utf8
    }
    foreach ($f in $files) {
        $rel = $f.FullName.Substring($dir.Length + 1) -replace '[\\/]', '__'
        Copy-Item -LiteralPath $f.FullName -Destination (Join-Path $dest $rel)
    }
}
$inventory | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $stage 'evidence/rust-dependencies.json') -Encoding utf8

@"
# MedScale CLI $version ($Target)

MedScale is licensed under Apache-2.0 (LICENSE). This is an unsigned
engineering build: it is not clinically validated, it is not authorized for
real patient data (REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED), and it is not a
qualified release.

The binary includes $($inventory.Count - 1) third-party Rust crates, listed with
their licenses in evidence/rust-dependencies.json. Each crate's own license
and notice files are under licenses/<crate>-<version>/. It statically links
OpenSSL (Apache-2.0, via openssl-src) and SQLCipher/SQLite (BSD-style and
public domain, via libsqlite3-sys); their license files are in those folders.

SBOM.cdx.json is the CycloneDX software bill of materials for the source
revision this was built from.
"@ | Set-Content -LiteralPath (Join-Path $stage 'NOTICE.md') -Encoding utf8

& pwsh -NoProfile -File (Join-Path $repoRoot 'scripts/generate-release-sbom.ps1') -OutPath (Join-Path $stage 'SBOM.cdx.json') | Out-Host
if ($LASTEXITCODE -ne 0) { throw 'SBOM generation failed' }

# Checksums of every file inside the archive.
Push-Location $stage
try {
    $sums = Get-ChildItem -Recurse -File | Sort-Object FullName | ForEach-Object {
        '{0}  {1}' -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), ($_.FullName.Substring($stage.Length + 1) -replace '\\', '/')
    }
    $sums | Set-Content -LiteralPath (Join-Path $stage 'SHA256SUMS') -Encoding ascii
} finally { Pop-Location }

Push-Location $outRoot
try {
    if ($Target -like '*windows*') {
        $archive = "$name.zip"
        Compress-Archive -LiteralPath $name -DestinationPath $archive -CompressionLevel Optimal
    } else {
        $archive = "$name.tar.gz"
        & tar -czf $archive $name
        if ($LASTEXITCODE -ne 0) { throw 'tar failed' }
    }
    '{0}  {1}' -f (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant(), $archive |
        Set-Content -LiteralPath "SHA256SUMS-$Target" -Encoding ascii
    Get-Content -LiteralPath "SHA256SUMS-$Target"
} finally { Pop-Location }
