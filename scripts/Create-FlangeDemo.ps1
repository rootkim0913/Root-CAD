param([switch]$Release)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$profile = if ($Release) { 'release' } else { 'debug' }
$cli = Join-Path $root "target\$profile\root-cad-cli.exe"
if (!(Test-Path $cli)) { throw 'Run scripts\Build-AiCad.ps1 first.' }
$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$steps = @(
    @{command='flange';params=@{outerDiameter=100;pipeDiameter=70;pipeLength=500;flangeThickness=12;boltCircle=85;holeDiameter=10;holes=12}},
    @{command='dimdiameter';params=@{center=@(0,0);point=@(50,0)}},
    @{command='dimlinear';params=@{p1=@(112,-35);p2=@(612,-35);at=@(362,-65)}},
    @{command='dimlinear';params=@{p1=@(100,-50);p2=@(112,-50);at=@(106,-85)}},
    @{command='dimlinear';params=@{p1=@(612,-35);p2=@(612,35);at=@(640,0)}},
    @{command='text';params=@{at=@(-50,70);height=4;text='12 x DIA 10 / PCD 85'}},
    @{command='text';params=@{at=@(130,70);height=4;text='SCHEMATIC - mm; flange thickness 12 specified for this example'}}
)
$plan = @{steps=$steps} | ConvertTo-Json -Depth 8 -Compress
& $cli run --metric --cmd "draft $plan" --save (Join-Path $dist 'flange-demo.dxf') --export (Join-Path $dist 'flange-demo.png')
if ($LASTEXITCODE -ne 0) { throw 'Demo generation failed.' }
Write-Host "Demo saved to $dist\flange-demo.dxf and flange-demo.png"
