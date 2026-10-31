@echo off
rem Starts Elwynn Survival. The first time it asks for your WoW 1.12.1 folder and the host's address.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0launcher.ps1" -Game "Elwynn Survival"
if errorlevel 1 pause
