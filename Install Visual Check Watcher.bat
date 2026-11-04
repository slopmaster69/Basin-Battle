@echo off
rem Double-click once: the visual check watcher starts now, minimized, and again whenever you sign
rem in to Windows. It runs the scenarios that request files in Documents\visual-check\requests ask
rem for (docs\visual-check.md). Uninstall Visual Check Watcher.bat undoes it.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\visual-check-windows.ps1" -InstallWatcher
pause
