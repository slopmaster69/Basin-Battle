@echo off
rem Play: gets the newest version if there is one, then starts the game where you played last.
rem Press C in the first 3 seconds to choose the game, the server or your WoW folder again.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0launcher\play.ps1" %*
if errorlevel 1 pause
