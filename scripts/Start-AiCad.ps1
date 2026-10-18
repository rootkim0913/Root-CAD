param([int]$Port = 7980, [switch]$Release)
$ErrorActionPreference = 'Stop'
if ($Port -lt 1 -or $Port -gt 65535) { throw 'Port must be in 1..65535.' }
$root = Split-Path -Parent $PSScriptRoot
$profile = if ($Release) { 'release' } else { 'debug' }
$app = Join-Path $root "target\$profile\root-cad.exe"
if (!(Test-Path $app)) { throw 'Run scripts\Build-AiCad.ps1 first.' }
Start-Process -FilePath $app -ArgumentList @('--control', $Port) -WindowStyle Hidden
