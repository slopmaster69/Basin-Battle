@echo off
rem Starts Basin Battle. The first time it asks for your WoW 1.12.1 folder and the host's address.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0launcher.ps1"
if errorlevel 1 pause
