# Engineering preview packaging only; this does not establish release readiness.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$taskRepoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$taskAppRoot = Join-Path $taskRepoRoot 'apps/desktop-tauri'
$taskPackageRoot = [IO.Path]::GetFullPath((Join-Path $taskRepoRoot 'target/tauri-preview-package'))
$taskTargetRoot = [IO.Path]::GetFullPath((Join-Path $taskRepoRoot 'target')) + [IO.Path]::DirectorySeparatorChar
if (-not $taskPackageRoot.StartsWith($taskTargetRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'preview package escaped target' }
if (Test-Path -LiteralPath $taskPackageRoot) { throw 'preview package must be assembled in a fresh directory' }
$taskExecutableName = if ($env:OS -eq 'Windows_NT') { 'medscale-desktop-tauri.exe' } else { 'medscale-desktop-tauri' }
$taskExecutable = Join-Path $taskAppRoot "src-tauri/target/release/$taskExecutableName"
if (-not (Test-Path -LiteralPath $taskExecutable -PathType Leaf)) { throw 'native preview executable is missing' }

$taskMetadataJson = & cargo metadata --manifest-path (Join-Path $taskAppRoot 'src-tauri/Cargo.toml') --format-version 1 --locked | Out-String
if ($LASTEXITCODE -ne 0) { throw 'locked dependency metadata failed' }
$taskMetadata = $taskMetadataJson | ConvertFrom-Json
foreach ($taskDirectory in @('bin','licenses/rust','licenses/npm','licenses/fonts','source-archives','evidence')) {
    New-Item -ItemType Directory -Path (Join-Path $taskPackageRoot $taskDirectory) -Force | Out-Null
}
Copy-Item -LiteralPath $taskExecutable -Destination (Join-Path $taskPackageRoot 'bin')
Copy-Item -LiteralPath (Join-Path $taskRepoRoot 'LICENSE') -Destination $taskPackageRoot
Copy-Item -LiteralPath (Join-Path $taskAppRoot 'src-tauri/Cargo.lock'),(Join-Path $taskAppRoot 'package-lock.json') -Destination (Join-Path $taskPackageRoot 'evidence')
Copy-Item -LiteralPath (Join-Path $taskRepoRoot 'docs/engineering/admissions/096-tauri-presentation.md') -Destination (Join-Path $taskPackageRoot 'evidence')

$taskRustInventory = @()
foreach ($taskPackage in $taskMetadata.packages) {
    if (-not $taskPackage.source) { continue }
    $taskSourceDirectory = Split-Path -Parent $taskPackage.manifest_path
    $taskNoticeFiles = @(Get-ChildItem -LiteralPath $taskSourceDirectory -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|README)' })
    $taskNoticeDirectory = Join-Path $taskPackageRoot "licenses/rust/$($taskPackage.name)-$($taskPackage.version)"
    New-Item -ItemType Directory -Path $taskNoticeDirectory -Force | Out-Null
    foreach ($taskNoticeFile in $taskNoticeFiles) { Copy-Item -LiteralPath $taskNoticeFile.FullName -Destination $taskNoticeDirectory }
    $taskRustInventory += [ordered]@{ name=$taskPackage.name; version=$taskPackage.version; license=$taskPackage.license; source=$taskPackage.source }
}
$taskRustInventory | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $taskPackageRoot 'evidence/rust-dependencies.json') -Encoding utf8

$taskMplPackages = @(
    @{ name='cssparser'; version='0.37.0'; checksum='8c9cdaae01d5ed7882b04d795e7f752f46ff52d2fa3b50a20d28c464510bba98' },
    @{ name='cssparser-macros'; version='0.7.1'; checksum='d045de693cb712d0b22c6a64be5b953f67b3ce00ab5ad3dd5d8b441886ab8e1a' },
    @{ name='dtoa-short'; version='0.3.5'; checksum='cd1511a7b6a56299bd043a9c167a6d2bfb37bf84a6dfceaba651168adfb43c87' },
    @{ name='option-ext'; version='0.2.0'; checksum='04744f49eae99ab78e0d5c0b603ab218f515ea8cfe5a456d7629ad883a3b6e7d' },
    @{ name='selectors'; version='0.38.0'; checksum='8adfa1c298912827b8a28b223b3b874357397ae706e6190acd9bf28cee99114d' }
)
$taskCargoRoot = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path ([Environment]::GetFolderPath('UserProfile')) '.cargo' }
$taskLockText = Get-Content -LiteralPath (Join-Path $taskAppRoot 'src-tauri/Cargo.lock') -Raw
foreach ($taskMpl in $taskMplPackages) {
    $taskLockBlocks = @($taskLockText -split '\[\[package\]\]' | Where-Object { $_ -match ('(?m)^name = "' + [regex]::Escape($taskMpl.name) + '"') -and $_ -match ('(?m)^version = "' + [regex]::Escape($taskMpl.version) + '"') })
    if ($taskLockBlocks.Count -ne 1 -or $taskLockBlocks[0] -notmatch ('checksum = "' + $taskMpl.checksum + '"')) { throw "MPL source admission differs from lock: $($taskMpl.name)" }
    $taskArchives = @(Get-ChildItem -LiteralPath (Join-Path $taskCargoRoot 'registry/cache') -Recurse -File -Filter "$($taskMpl.name)-$($taskMpl.version).crate")
    if ($taskArchives.Count -eq 0) { throw "MPL source archive missing: $($taskMpl.name)" }
    $taskArchive = $taskArchives[0]
    if ((Get-FileHash -LiteralPath $taskArchive.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskMpl.checksum) { throw "MPL source archive digest mismatch: $($taskMpl.name)" }
    Copy-Item -LiteralPath $taskArchive.FullName -Destination (Join-Path $taskPackageRoot 'source-archives')
}
$taskCssParser = $taskMetadata.packages | Where-Object { $_.name -eq 'cssparser' -and $_.version -eq '0.37.0' } | Select-Object -First 1
Copy-Item -LiteralPath (Join-Path (Split-Path -Parent $taskCssParser.manifest_path) 'LICENSE') -Destination (Join-Path $taskPackageRoot 'licenses/MPL-2.0.txt')
@'
This is an unqualified synthetic-only MedScale presentation preview.
Unmodified MPL-2.0 dependencies are identified in evidence/096-tauri-presentation.md.
Their exact original source archives and notices are included in source-archives/.
No modification to those covered sources is made by MedScale.
The original MPL license is included at licenses/MPL-2.0.txt.
'@ | Set-Content -LiteralPath (Join-Path $taskPackageRoot 'MPL-SOURCE-NOTICE.txt') -Encoding utf8

foreach ($taskFontLicense in @('Inter-OFL.txt','JetBrainsMono-OFL.txt')) {
    Copy-Item -LiteralPath (Join-Path $taskRepoRoot "assets/brand/fonts/$taskFontLicense") -Destination (Join-Path $taskPackageRoot 'licenses/fonts')
}
Copy-Item -LiteralPath (Join-Path $taskRepoRoot 'assets/brand/fonts/FONT_NOTICE.md') -Destination (Join-Path $taskPackageRoot 'licenses/fonts')
$taskFontNotice = Get-Content -LiteralPath (Join-Path $taskRepoRoot 'assets/brand/fonts/FONT_NOTICE.md') -Raw
@"
# MedScale Tauri presentation preview notices

MedScale's first-party source is licensed under Apache-2.0; see LICENSE.
This unqualified synthetic-only preview includes the Rust dependencies listed in
evidence/rust-dependencies.json and npm runtime packages identified in
evidence/package-lock.json. Original notices are under licenses/rust and licenses/npm.
Unmodified MPL sources and their notice are included under source-archives and
MPL-SOURCE-NOTICE.txt. Dependency admission is recorded in evidence/096-tauri-presentation.md.

$taskFontNotice
"@ | Set-Content -LiteralPath (Join-Path $taskPackageRoot 'NOTICE.md') -Encoding utf8
foreach ($taskNpmName in @('react','react-dom','scheduler','@tauri-apps/api')) {
    $taskNpmDirectory = Join-Path $taskAppRoot "node_modules/$taskNpmName"
    $taskNpmNotices = @(Get-ChildItem -LiteralPath $taskNpmDirectory -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE)' })
    if ($taskNpmNotices.Count -eq 0) { throw "runtime npm license missing: $taskNpmName" }
    $taskNpmNoticeDirectory = Join-Path $taskPackageRoot ("licenses/npm/" + ($taskNpmName -replace '[@/]','_'))
    New-Item -ItemType Directory -Path $taskNpmNoticeDirectory -Force | Out-Null
    foreach ($taskNpmNotice in $taskNpmNotices) { Copy-Item -LiteralPath $taskNpmNotice.FullName -Destination $taskNpmNoticeDirectory }
}

$taskReceipt = [ordered]@{
    status='UNQUALIFIED_PREVIEW'; synthetic_only=$true; core_connected=$true; core='rust-core-in-process-cli-session'
    head=(& git -C $taskRepoRoot rev-parse HEAD).Trim(); tree=(& git -C $taskRepoRoot rev-parse 'HEAD^{tree}').Trim()
    platform=[Runtime.InteropServices.RuntimeInformation]::OSDescription
    rustc=(& rustc --version | Out-String).Trim(); node=(& node --version | Out-String).Trim()
    executable_sha256=(Get-FileHash -LiteralPath $taskExecutable -Algorithm SHA256).Hash.ToLowerInvariant()
    executable_bytes=(Get-Item -LiteralPath $taskExecutable).Length
    native_visual_evidence='NOT_CAPTURED'; privacy_qualified=$false; release_ready=$false
}
$taskReceipt | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $taskPackageRoot 'evidence/build-receipt.json') -Encoding utf8
$taskInventory = @(Get-ChildItem -LiteralPath $taskPackageRoot -Recurse -File | Sort-Object FullName | ForEach-Object {
    [ordered]@{ path=[IO.Path]::GetRelativePath($taskPackageRoot,$_.FullName).Replace('\','/'); bytes=$_.Length; sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
})
$taskInventory | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $taskPackageRoot 'file-inventory.json') -Encoding utf8
Write-Output "UNQUALIFIED_TAURI_PREVIEW_PACKAGED files=$($taskInventory.Count)"
