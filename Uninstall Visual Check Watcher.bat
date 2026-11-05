@echo off
rem Double-click to stop the visual check watcher and keep it from starting with Windows.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\visual-check-windows.ps1" -UninstallWatcher
pause
