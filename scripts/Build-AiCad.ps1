param([switch]$Release)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $localCargo = Join-Path $root '.tools\cargo\bin'
    if (Test-Path (Join-Path $localCargo 'cargo.exe')) {
        $env:CARGO_HOME = Join-Path $root '.tools\cargo'
        $env:RUSTUP_HOME = Join-Path $root '.tools\rustup'
        $env:PATH = "$localCargo;$env:PATH"
        $gnu = Join-Path $env:RUSTUP_HOME 'toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained'
        if (Test-Path (Join-Path $gnu 'dlltool.exe')) {
            $env:PATH = "$gnu;$env:PATH"
            $binutils = Join-Path $root '.tools\binutils\mingw64\bin'
            if (Test-Path $binutils) { $env:PATH = "$binutils;C:\Program Files\Git\mingw64\bin;$env:PATH" }
            $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $gnu 'x86_64-w64-mingw32-gcc.exe'
        }
    }
    $buildArgs = @('build', '-p', 'cadcraft', '--bin', 'root-cad', '-p', 'cadcraft-cli', '--bin', 'root-cad-cli')
    if ($Release) { $buildArgs += '--release' }
    & cargo @buildArgs
    if ($LASTEXITCODE -ne 0) { throw 'Root CAD build failed.' }
    $profile = if ($Release) { 'release' } else { 'debug' }
    $cli = Join-Path $root "target\$profile\root-cad-cli.exe"
    $dist = Join-Path $root 'dist'
    New-Item -ItemType Directory -Force -Path $dist | Out-Null
    @{mcpServers=@{cadcraft=@{command=$cli;args=@('mcp','--connect','127.0.0.1:7980','--launch')}}} |
        ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 (Join-Path $dist 'mcp-claude.json')
    $toml = "[mcp_servers.cadcraft]`ncommand = " + (ConvertTo-Json $cli -Compress) + "`nargs = [`"mcp`", `"--connect`", `"127.0.0.1:7980`", `"--launch`"]`n"
    Set-Content -Encoding utf8 -Path (Join-Path $dist 'mcp-codex.toml') -Value $toml
    Write-Host "Built Root CAD. MCP configurations: $dist"
} finally { Pop-Location }
