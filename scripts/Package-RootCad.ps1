param(
    [string]$Version = '0.7.0-root.1',
    [string]$BinaryDirectory,
    [string]$StripTool,
    [string]$RuntimeLicenseDirectory,
    [Parameter(Mandatory=$true)][string]$MetadataFile
)
$ErrorActionPreference = 'Stop'
if ($Version -notmatch '^[0-9A-Za-z][0-9A-Za-z.-]*$') { throw 'Invalid version.' }
$root = Split-Path -Parent $PSScriptRoot
if (!$BinaryDirectory) { $BinaryDirectory = Join-Path $root 'target\debug' }
$output = Join-Path $root 'dist\releases'
New-Item -ItemType Directory -Force -Path $output | Out-Null
$stage = Join-Path $output "Root-CAD-$Version-windows-x64"
if (Test-Path -LiteralPath $stage) { throw "Staging folder already exists: $stage" }
New-Item -ItemType Directory -Path $stage | Out-Null
function Check-Pe($path, $subsystem) {
    $bytes = [IO.File]::ReadAllBytes($path)
    $pe = [BitConverter]::ToInt32($bytes,0x3c)
    if ([BitConverter]::ToUInt16($bytes,$pe+4) -ne 0x8664) { throw 'Only Windows x64 binaries can be packaged here.' }
    if ([BitConverter]::ToUInt16($bytes,$pe+0x5c) -ne $subsystem) { throw "Unexpected subsystem: $path" }
}
foreach ($name in @('root-cad.exe','root-cad-cli.exe')) {
    $source = Join-Path $BinaryDirectory $name
    $destination = Join-Path $stage $name
    if ($StripTool) {
        $flags = @('--strip-debug')
        if ($name -eq 'root-cad.exe') { $flags += @('--subsystem','windows') }
        & $StripTool @flags $source $destination
        if ($LASTEXITCODE -ne 0) { throw 'Stripping failed.' }
    } else { Copy-Item -LiteralPath $source -Destination $destination }
}
Check-Pe (Join-Path $stage 'root-cad.exe') 2
Check-Pe (Join-Path $stage 'root-cad-cli.exe') 3
foreach ($file in @('LICENSE-MIT','LICENSE-APACHE','NOTICE','ATTRIBUTION.md')) { Copy-Item -LiteralPath (Join-Path $root $file) -Destination $stage }
foreach ($file in @('Start-RootCAD.cmd','Configure-MCP.ps1')) { Copy-Item -LiteralPath (Join-Path $root "packaging\windows\$file") -Destination $stage }
Copy-Item -LiteralPath (Join-Path $root 'packaging\windows\PORTABLE-README.ko.md') -Destination (Join-Path $stage 'README.ko.md')
New-Item -ItemType Directory -Path (Join-Path $stage 'docs'),(Join-Path $stage 'examples'),(Join-Path $stage 'licenses') | Out-Null
Copy-Item -LiteralPath (Join-Path $root 'docs\course-compatibility.ko.md') -Destination (Join-Path $stage 'docs')
foreach ($extension in @('dxf','pdf','png')) {
    $example = Join-Path $root "dist\root-course-demo.$extension"
    if (Test-Path -LiteralPath $example) { Copy-Item -LiteralPath $example -Destination (Join-Path $stage 'examples') }
}
$metadata = Get-Content -Raw -LiteralPath $MetadataFile | ConvertFrom-Json
$notices = @('Dependencies resolved for this build; includes transitive and build-time packages.', 'Original upstream licences remain in the licenses directory.', '')
foreach ($package in $metadata.packages | Sort-Object name,version) {
    $notices += "$($package.name) $($package.version) | $($package.license) | $($package.repository)"
    if (!$package.source) { continue }
    $directory = Split-Path -Parent $package.manifest_path
    $files = @()
    for ($level=0; $level -lt 4 -and $directory; $level++) {
        $files = @(Get-ChildItem -LiteralPath $directory -File | Where-Object {$_.Name -match '^(LICENSE|LICENCE|COPYING|COPYRIGHT|NOTICE|OFL)([.-]|$)'})
        if ($files.Count) { break }
        $directory = Split-Path -Parent $directory
    }
    if ($files.Count) {
        $destination = Join-Path $stage "licenses\$($package.name)-$($package.version)"
        New-Item -ItemType Directory -Path $destination | Out-Null
        foreach ($file in $files) { Copy-Item -LiteralPath $file.FullName -Destination $destination }
    }
}
$notices | Set-Content -Encoding utf8 -LiteralPath (Join-Path $stage 'THIRD-PARTY-NOTICES.txt')
if (!$RuntimeLicenseDirectory -and (Get-Command rustc -ErrorAction SilentlyContinue)) {
    $sysroot = & rustc --print sysroot
    if ($LASTEXITCODE -eq 0) { $RuntimeLicenseDirectory = Join-Path $sysroot 'share\doc\rust' }
}
if ($RuntimeLicenseDirectory -and (Test-Path -LiteralPath (Join-Path $RuntimeLicenseDirectory 'COPYRIGHT-library.html'))) {
    $runtime = Join-Path $stage 'licenses\rust-runtime'
    New-Item -ItemType Directory -Path $runtime | Out-Null
    Copy-Item -LiteralPath (Join-Path $RuntimeLicenseDirectory 'COPYRIGHT-library.html') -Destination $runtime
    if (Test-Path -LiteralPath (Join-Path $RuntimeLicenseDirectory 'licenses')) {
        Copy-Item -LiteralPath (Join-Path $RuntimeLicenseDirectory 'licenses') -Destination $runtime -Recurse
    }
}
@{product='Root CAD';version=$Version;platform='windows-x64';source='https://github.com/rootkim0913/Root-CAD';build='See release notes for build profile and verification scope.'} |
    ConvertTo-Json | Set-Content -Encoding utf8 -LiteralPath (Join-Path $stage 'BUILD-INFO.json')
$zip = Join-Path $output "Root-CAD-$Version-windows-x64.zip"
if (Test-Path -LiteralPath $zip) { throw "Archive already exists: $zip" }
foreach ($file in Get-ChildItem -LiteralPath $stage -Recurse -File) {
    if ($file.LastWriteTime.Year -lt 1980) { $file.LastWriteTime = [datetime]'1980-01-01' }
}
Compress-Archive -LiteralPath $stage -DestinationPath $zip -CompressionLevel Optimal
$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $zip).Hash.ToLowerInvariant()
"$hash  $([IO.Path]::GetFileName($zip))" | Set-Content -Encoding ascii -LiteralPath (Join-Path $output 'SHA256SUMS.txt')
Get-Item -LiteralPath $zip | Select-Object Name,Length
