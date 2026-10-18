@echo off
setlocal
set "CADCRAFT_CONFIG_DIR=%~dp0data"
start "" "%~dp0root-cad.exe" --control 7980 %*
