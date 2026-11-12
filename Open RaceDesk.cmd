@echo off
if exist "%LOCALAPPDATA%\Programs\RaceDesk\RaceDesk.exe" (
  start "" "%LOCALAPPDATA%\Programs\RaceDesk\RaceDesk.exe"
) else (
  start "" "%~dp0app\RaceDesk.exe"
)
