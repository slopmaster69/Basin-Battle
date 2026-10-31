@echo off
rem Double-click to host Basin Battle and Elwynn Survival on this PC. Leave this window open while
rem you play; close it to stop the server. Arguments go to the server (e.g. --password secret).
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\host-windows.ps1" %*
pause
