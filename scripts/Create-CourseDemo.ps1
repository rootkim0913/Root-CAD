param([switch]$Release)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$profile = if ($Release) { 'release' } else { 'debug' }
$cli = Join-Path $root "target\$profile\root-cad-cli.exe"
if (!(Test-Path $cli)) { throw 'Run scripts\Build-AiCad.ps1 first.' }
$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Force -Path $dist | Out-Null
# Original synthetic geometry for command practice; not a construction document.
$steps = @(
    @{command='layer.new';params=@{name='Walls';color='white';current=$true}},
    @{command='rectang';params=@{p1=@(0,0);p2=@(6000,4000)}},
    @{command='rectang';params=@{p1=@(150,150);p2=@(5850,3850)}},
    @{command='line';params=@{points=@(@(3500,150),@(3500,2500),@(5850,2500))}},
    @{command='layer.new';params=@{name='Annotations';color='cyan';current=$true}},
    @{command='dimlinear';params=@{p1=@(0,0);p2=@(6000,0);at=@(3000,-550)}},
    @{command='dimlinear';params=@{p1=@(6000,0);p2=@(6000,4000);at=@(6650,2000)}},
    @{command='text';params=@{at=@(800,1900);height=150;text='ROOT CAD / LIVING'}},
    @{command='text';params=@{at=@(4000,3000);height=125;text='ROOM'}},
    @{command='text';params=@{at=@(0,5200);height=200;text='Root CAD - course command demo (mm)'}}
)
$plan = @{steps=$steps} | ConvertTo-Json -Depth 8 -Compress
$style = @{name='Root-mm';textHeight=100;arrowSize=75;decimals=0;current=$true} | ConvertTo-Json -Compress
$leader = @{points=@(@(150,4000),@(-500,4650),@(2000,4650));text='Synthetic 2D command practice'} | ConvertTo-Json -Compress
& $cli run --metric --cmd "dimstyle $style" --cmd "draft $plan" --cmd "qleader $leader" --save (Join-Path $dist 'root-course-demo.dxf') --export (Join-Path $dist 'root-course-demo.png')
if ($LASTEXITCODE -ne 0) { throw 'Course demo generation failed.' }
& $cli convert (Join-Path $dist 'root-course-demo.dxf') (Join-Path $dist 'root-course-demo.pdf')
if ($LASTEXITCODE -ne 0) { throw 'PDF export failed.' }
Write-Host "Demo saved to $dist\root-course-demo.dxf (also PNG and PDF)."
