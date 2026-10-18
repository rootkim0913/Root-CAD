param([int]$Port = 7980, [switch]$InstallCodex)
$ErrorActionPreference = 'Stop'
if ($Port -lt 1 -or $Port -gt 65535) { throw 'Port must be in 1..65535.' }
$cli = Join-Path $PSScriptRoot 'root-cad-cli.exe'
if (!(Test-Path -LiteralPath $cli)) { throw 'Extract the entire ZIP first.' }
$config = Join-Path $PSScriptRoot 'mcp-config'
New-Item -ItemType Directory -Force -Path $config | Out-Null
$server = @{command=$cli;args=@('mcp','--connect',"127.0.0.1:$Port",'--launch');env=@{CADCRAFT_CONFIG_DIR=(Join-Path $PSScriptRoot 'data')}}
$utf8 = [Text.UTF8Encoding]::new($false)
$json = @{mcpServers=@{'root-cad'=$server}} | ConvertTo-Json -Depth 6
[IO.File]::WriteAllText((Join-Path $config 'claude.json'), $json, $utf8)
$quoted = ConvertTo-Json $cli -Compress
$state = ConvertTo-Json (Join-Path $PSScriptRoot 'data') -Compress
$toml = @"
[mcp_servers.root-cad]
command = $quoted
args = ["mcp", "--connect", "127.0.0.1:$Port", "--launch"]
[mcp_servers.root-cad.env]
CADCRAFT_CONFIG_DIR = $state
"@
[IO.File]::WriteAllText((Join-Path $config 'codex.toml'), $toml, $utf8)
if ($InstallCodex) {
    & codex mcp add root-cad --env "CADCRAFT_CONFIG_DIR=$(Join-Path $PSScriptRoot 'data')" -- $cli mcp --connect "127.0.0.1:$Port" --launch
    if ($LASTEXITCODE -ne 0) { throw 'Codex MCP registration failed.' }
}
Write-Host "MCP configurations created in $config. Merge only the root-cad entry into your client settings."
