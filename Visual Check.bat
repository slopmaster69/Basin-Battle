@echo off
rem Double-click to see abilities working in the game client: it plays a scenario from
rem tools\visual-check against the server on this PC and keeps stills and a video of it in
rem visual-check-out\ (docs\visual-check.md). Start Host Basin Battle.bat first.
rem Arguments are scenario names (or all, or setup); with none it lists them and asks.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\visual-check-windows.ps1" %*
pause
