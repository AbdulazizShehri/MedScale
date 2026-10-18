# Spec 101 T101-04: assemble the unsigned engineering release set for this OS.
# Runs after scripts/package-tauri-preview.ps1 and the Tauri bundle step in the
# same CI job. This does not establish release readiness and signs nothing.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$appRoot = Join-Path $repoRoot 'apps/desktop-tauri'
$bundleRoot = Join-Path $appRoot 'src-tauri/target/release/bundle'
$previewRoot = Join-Path $repoRoot 'target/tauri-preview-package'
$platform = if ($IsWindows) { 'windows' } elseif ($IsMacOS) { 'macos' } elseif ($IsLinux) { 'linux' } else { throw 'unsupported platform' }
$outRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot "target/tauri-release/$platform"))
$targetRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'target')) + [IO.Path]::DirectorySeparatorChar
if (-not $outRoot.StartsWith($targetRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'release set escaped target' }
if (Test-Path -LiteralPath $outRoot) { throw 'release set must be assembled in a fresh directory' }
if (-not (Test-Path -LiteralPath $previewRoot)) { throw 'run scripts/package-tauri-preview.ps1 first (notices, MPL sources, inventory)' }

$installers = switch ($platform) {
    'windows' { @(Get-ChildItem -LiteralPath (Join-Path $bundleRoot 'nsis') -Filter '*.exe' -File) }
    'macos' { @(Get-ChildItem -LiteralPath (Join-Path $bundleRoot 'dmg') -Filter '*.dmg' -File) }
    'linux' { @(Get-ChildItem -LiteralPath (Join-Path $bundleRoot 'deb') -Filter '*.deb' -File) }
}
if ($installers.Count -ne 1) { throw "expected exactly one installer for $platform, found $($installers.Count)" }

New-Item -ItemType Directory -Path (Join-Path $outRoot 'installer') -Force | Out-Null
Copy-Item -LiteralPath $installers[0].FullName -Destination (Join-Path $outRoot 'installer')
Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination $outRoot
foreach ($item in 'NOTICE.md', 'MPL-SOURCE-NOTICE.txt', 'licenses', 'source-archives') {
    Copy-Item -LiteralPath (Join-Path $previewRoot $item) -Destination $outRoot -Recurse
}
New-Item -ItemType Directory -Path (Join-Path $outRoot 'evidence') -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $previewRoot 'evidence/rust-dependencies.json') -Destination (Join-Path $outRoot 'evidence')

# Root-workspace SBOM bound to this exact source (Spec 054 generator, unchanged).
& pwsh -NoProfile -File (Join-Path $repoRoot 'scripts/generate-release-sbom.ps1') -OutPath (Join-Path $outRoot 'SBOM.cdx.json') | Out-Host
if ($LASTEXITCODE -ne 0) { throw 'SBOM generation failed' }

# macOS: also inventory the .app payload inside the bundle directory.
$appPayload = @()
if ($platform -eq 'macos') {
    $app = @(Get-ChildItem -LiteralPath (Join-Path $bundleRoot 'macos') -Filter '*.app' -Directory)
    if ($app.Count -ne 1) { throw 'expected exactly one .app' }
    $appPayload = @(Get-ChildItem -LiteralPath $app[0].FullName -Recurse -File | Sort-Object FullName | ForEach-Object {
        [ordered]@{ path = [IO.Path]::GetRelativePath($app[0].FullName, $_.FullName).Replace('\', '/'); bytes = $_.Length; sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
    })
}

function Sha([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
$cliVersion = (Get-Content -LiteralPath (Join-Path $appRoot 'package-lock.json') -Raw | ConvertFrom-Json -AsHashtable)['packages']['node_modules/@tauri-apps/cli']['version']
$payload = @(Get-ChildItem -LiteralPath $outRoot -Recurse -File | Sort-Object FullName | ForEach-Object {
    [ordered]@{ path = [IO.Path]::GetRelativePath($outRoot, $_.FullName).Replace('\', '/'); bytes = $_.Length; sha256 = Sha $_.FullName }
})
$installerRel = 'installer/' + $installers[0].Name
$manifest = [ordered]@{
    schema = 'medscale.tauri-release-manifest.v1'
    spec = '101'
    product = 'MedScale'
    version = (Get-Content -LiteralPath (Join-Path $appRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
    platform = $platform
    architecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
    source_sha = (& git -C $repoRoot rev-parse HEAD).Trim()
    tree_sha = (& git -C $repoRoot rev-parse 'HEAD^{tree}').Trim()
    cargo_lock_sha256 = Sha (Join-Path $repoRoot 'Cargo.lock')
    tauri_cargo_lock_sha256 = Sha (Join-Path $appRoot 'src-tauri/Cargo.lock')
    npm_lock_sha256 = Sha (Join-Path $appRoot 'package-lock.json')
    toolchain = [ordered]@{ rustc = (& rustc --version | Out-String).Trim(); node = (& node --version | Out-String).Trim(); tauri_cli = $cliVersion }
    installer = [ordered]@{ path = $installerRel; bytes = $installers[0].Length; sha256 = Sha (Join-Path $outRoot $installerRel) }
    sbom_sha256 = Sha (Join-Path $outRoot 'SBOM.cdx.json')
    notice_sha256 = Sha (Join-Path $outRoot 'NOTICE.md')
    app_payload = $appPayload
    payload = $payload
    signed = $false
    notarized = $false
    release_ready = $false
    reproducible_binary_build = $false
    signing = 'NOT_GRANTED'
}
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $outRoot 'package-manifest.json') -Encoding utf8
$sums = @(Get-ChildItem -LiteralPath $outRoot -Recurse -File | Where-Object Name -ne 'SHA256SUMS' | Sort-Object FullName | ForEach-Object {
    '{0}  {1}' -f (Sha $_.FullName), [IO.Path]::GetRelativePath($outRoot, $_.FullName).Replace('\', '/')
})
$sums -join "`n" | Set-Content -LiteralPath (Join-Path $outRoot 'SHA256SUMS') -Encoding utf8 -NoNewline
Write-Output "TAURI_RELEASE_SET platform=$platform installer=$installerRel sha256=$($manifest.installer.sha256) files=$($payload.Count)"
