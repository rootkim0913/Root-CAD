param([string]$Version = '0.7.0-root.1')
$ErrorActionPreference = 'Stop'
if ($Version -notmatch '^[0-9A-Za-z][0-9A-Za-z.-]*$') { throw 'Invalid version.' }
$root = Split-Path -Parent $PSScriptRoot
$output = Join-Path $root 'dist\releases'
New-Item -ItemType Directory -Force -Path $output | Out-Null
$stage = Join-Path $output "Root-CAD-$Version-source"
if (Test-Path -LiteralPath $stage) { throw 'Source staging folder already exists.' }
New-Item -ItemType Directory -Path $stage | Out-Null
# Only project sources. Build caches, drawings, credentials and machine-specific settings are excluded.
$directories = @('.cargo','apps','assets','contributors','crates','docs','examples','packaging','scripts','xtask')
$blocked = @('.git','.tools','target','dist','node_modules','.codex','.agents','.claude','plan','plans')
function Copy-SourceDirectory([string]$from,[string]$to) {
    New-Item -ItemType Directory -Force -Path $to | Out-Null
    foreach ($entry in Get-ChildItem -LiteralPath $from -Force) {
        if (($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { continue }
        if ($entry.PSIsContainer) {
            if ($entry.Name -notin $blocked) { Copy-SourceDirectory $entry.FullName (Join-Path $to $entry.Name) }
        } elseif ($entry.Name -ne '.DS_Store' -and $entry.Name -notmatch '^\.env($|\.)|\.(pfx|p12|pem|key)$') {
            Copy-Item -LiteralPath $entry.FullName -Destination $to
        }
    }
}
foreach ($directory in $directories) {
    $from = Join-Path $root $directory
    if (Test-Path -LiteralPath $from) { Copy-SourceDirectory $from (Join-Path $stage $directory) }
}
foreach ($file in @('Cargo.toml','Cargo.lock','README.md','AGENTS.md','CLAUDE.md','ATTRIBUTION.md','NOTICE','LICENSE-MIT','LICENSE-APACHE','ROADMAP.md','rustfmt.toml','clippy.toml','.gitignore','Start-RootCAD.cmd')) {
    $from = Join-Path $root $file
    if (Test-Path -LiteralPath $from) { Copy-Item -LiteralPath $from -Destination $stage }
}
# Do not inherit upstream workflows that refer to another organisation's release services.
$zip = Join-Path $output "Root-CAD-$Version-source.zip"
if (Test-Path -LiteralPath $zip) { throw 'Source archive already exists.' }
Add-Type -AssemblyName System.IO.Compression.FileSystem
foreach ($file in Get-ChildItem -LiteralPath $stage -Recurse -File -Force) {
    if ($file.LastWriteTime.Year -lt 1980) { $file.LastWriteTime = [datetime]'1980-01-01' }
}
[IO.Compression.ZipFile]::CreateFromDirectory($stage,$zip,[IO.Compression.CompressionLevel]::Optimal,$true)
"$((Get-FileHash -Algorithm SHA256 -LiteralPath $zip).Hash.ToLowerInvariant())  $([IO.Path]::GetFileName($zip))" |
    Add-Content -Encoding ascii -LiteralPath (Join-Path $output 'SHA256SUMS.txt')
Get-Item -LiteralPath $zip | Select-Object Name,Length
