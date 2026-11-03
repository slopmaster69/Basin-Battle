@echo off
rem Host: runs your own server for both games on this PC, for you alone or for friends to join.
rem Leave this window open while anyone plays; close it to stop the server.
rem Options go to the server, e.g. Host.bat --password secret
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0launcher\host.ps1" %*
