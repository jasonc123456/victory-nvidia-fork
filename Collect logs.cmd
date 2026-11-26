@echo off
rem Collects the last run logs, the crash log and the GPU/driver details into victory-logs.zip on the desktop.
rem Nothing is sent anywhere; you choose whether to share the zip.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\nvidia\collect-logs.ps1"
pause
