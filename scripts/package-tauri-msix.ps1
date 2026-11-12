# Builds an UNSIGNED MSIX for Microsoft Store engineering qualification.
# Never signs, submits, uploads, publishes, or alters the user's computer.
# Restricted to disposable GitHub-hosted Windows runners by design.
[CmdletBinding()]
param(
    [ValidateSet('CI', 'PartnerCenter')][string]$Profile = 'CI',
    [string]$PackageName = '',
    [string]$Publisher = '',
    [string]$PublisherDisplayName = '',
    [string]$PackageVersion = '0.1.0.0'
)
$ErrorActionPreference = 'Stop'
if (-not $IsWindows -or $env:GITHUB_ACTIONS -ne 'true' -or -not $env:RUNNER_TEMP) {
    throw 'MSIX qualification only runs on disposable Windows GitHub Actions runners'
}
if ($Profile -eq 'CI') {
    if ($PackageName -or $Publisher -or $PublisherDisplayName) {
        throw 'CI identity must not be overridden with a real Partner Center identity'
    }
    $PackageName = 'MedScale.CI.Preview'
    $Publisher = 'CN=MedScale CI Preview'
    $PublisherDisplayName = 'MedScale CI Preview'
} else {
    if (-not $PackageName -or -not $Publisher -or -not $PublisherDisplayName) {
        throw 'PartnerCenter requires exact reserved Name, Publisher, and PublisherDisplayName'
    }
}
if ($PackageName -cnotmatch '^[A-Za-z0-9][A-Za-z0-9._-]{2,80}$') {
    throw 'Invalid MSIX package name'
}
if ($Publisher -notmatch '^CN=.{1,200}$' -or $Publisher.Contains("&#")) {
    throw 'Publisher must be the actual X.500 Publisher from Partner Center'
}
if ($PublisherDisplayName.Length -lt 2 -or $PublisherDisplayName.Length -gt 120) {
    throw 'Invalid publisher display name'
}
if ($PackageVersion -notmatch '^(?:0|[1-9][0-9]{0,4})(?:\.(?:0|[1-9][0-9]{0,4})){3}$') {
    throw 'MSIX version must have four numeric components'
}
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$appRoot = Join-Path $repoRoot 'apps/desktop-tauri'
$exe = Join-Path $appRoot 'src-tauri/target/release/medscale-desktop-tauri.exe'
$logo = Join-Path $appRoot 'src-tauri/icons/128x128@2x.png'
$template = Join-Path $appRoot 'msix/Package.appxmanifest.template'
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw "Missing release executable: $exe" }
if (-not (Test-Path -LiteralPath $logo -PathType Leaf)) { throw "Missing app logo: $logo" }
if (-not (Test-Path -LiteralPath $template -PathType Leaf)) { throw 'Missing MSIX manifest template' }
if (-not (Get-Command winapp -ErrorAction SilentlyContinue)) { throw 'Microsoft winapp CLI is not installed' }

$outRoot = Join-Path $repoRoot 'target/msix-qualification'
if (Test-Path -LiteralPath $outRoot) { throw 'Refusing to reuse an existing MSIX qualification directory' }
$stage = Join-Path $outRoot 'stage'
$assets = Join-Path $stage 'Assets'
New-Item -ItemType Directory -Path $assets -Force | Out-Null
Copy-Item -LiteralPath $exe -Destination (Join-Path $stage 'medscale-desktop-tauri.exe')

# Generate exact-dimension MSIX assets only on the isolated Windows runner.
Add-Type -AssemblyName System.Drawing
function Write-SquarePng([string]$From, [string]$To, [int]$Size) {
    $src = [Drawing.Image]::FromFile($From)
    $bmp = [Drawing.Bitmap]::new($Size, $Size)
    $g = [Drawing.Graphics]::FromImage($bmp)
    try {
        $g.Clear([Drawing.Color]::Transparent)
        $g.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::HighQuality
        $g.DrawImage($src, 0, 0, $Size, $Size)
        $bmp.Save($To, [Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $g.Dispose()
        $bmp.Dispose()
        $src.Dispose()
    }
}
Write-SquarePng $logo (Join-Path $assets 'Square44x44Logo.png') 44
Write-SquarePng $logo (Join-Path $assets 'Square150x150Logo.png') 150
Write-SquarePng $logo (Join-Path $assets 'StoreLogo.png') 50

$esc = { param([string]$value) [Security.SecurityElement]::Escape($value) }
$manifest = Get-Content -LiteralPath $template -Raw
$manifest = $manifest.Replace('@@PACKAGE_NAME@@', (& $esc $PackageName))
$manifest = $manifest.Replace('@@PUBLISHER@@', (& $esc $Publisher))
$manifest = $manifest.Replace('@@PUBLISHER_DISPLAY_NAME@@', (& $esc $PublisherDisplayName))
$manifest = $manifest.Replace('@@PACKAGE_VERSION@@', (& $esc $PackageVersion))
if ($manifest.Contains('@@')) { throw 'Unresolved MSIX manifest placeholder' }
$manifestPath = Join-Path $outRoot 'Package.appxmanifest'
$manifest | Set-Content -LiteralPath $manifestPath -Encoding utf8
[xml]$parsedManifest = $manifest
$identity = $parsedManifest.SelectSingleNode("//*[local-name()='Identity']")
if (-not $identity -or $identity.GetAttribute('Name') -cne $PackageName) {
    throw 'MSIX identity validation failed'
}
$app = $parsedManifest.SelectSingleNode("//*[local-name()='Application']")
if (-not $app -or $app.GetAttribute('Executable') -cne 'medscale-desktop-tauri.exe') {
    throw 'MSIX entry executable does not match the shipped Tauri binary'
}

$msix = Join-Path $outRoot "MedScale-$Profile-$PackageVersion-x64-UNSIGNED.msix"
# WinApp CLI v0.3.0 produces unsigned packages by default without a certificate.
# --no-sign is not supported in this pinned version; the CLI prints usage and exits nonzero.
& winapp package $stage --manifest $manifestPath --output $msix
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $msix -PathType Leaf)) {
    throw 'winapp package did not produce the unsigned MSIX'
}

# Validate the actual archive payload, not only the manifest template.
$unpacked = Join-Path $outRoot 'unpacked'
& winapp tool makeappx unpack /p $msix /d $unpacked
if ($LASTEXITCODE -ne 0) { throw 'MakeAppx could not unpack the candidate' }
foreach ($path in @('medscale-desktop-tauri.exe', 'AppxManifest.xml', 'Assets/Square150x150Logo.png', 'Assets/Square44x44Logo.png', 'Assets/StoreLogo.png')) {
    if (-not (Test-Path -LiteralPath (Join-Path $unpacked $path) -PathType Leaf)) {
        throw "Candidate missing required payload file: $path"
    }
}
$sourceSha = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceSha -notmatch '^[0-9a-f]{40}$') {
    throw 'Cannot bind MSIX qualification to a commit'
}
$metadata = [ordered]@{
    schema = 'medscale.msix-qualification.v1'
    source_sha = $sourceSha
    profile = $Profile
    package_name = $PackageName
    publisher = $Publisher
    package_version = $PackageVersion
    msix_file = [IO.Path]::GetFileName($msix)
    msix_sha256 = (Get-FileHash -LiteralPath $msix -Algorithm SHA256).Hash.ToLowerInvariant()
    executable_sha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    signature = 'unsigned'
    public_release = $false
    store_certified = $false
    install_qualified = $false
    end_user_installable_without_warning = $false
}
$metadata | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $outRoot 'qualification.json') -Encoding utf8
Write-Output "MSIX_QUALIFIED_ARCHIVE source=$sourceSha sha256=$($metadata.msix_sha256) profile=$Profile"
