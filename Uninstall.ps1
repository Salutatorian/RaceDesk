param([switch]$RemovePreferences)
$ErrorActionPreference = 'Stop'
$installPath = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Programs\RaceDesk'))
$scriptFolder = [IO.Path]::GetFullPath($PSScriptRoot)
if ($scriptFolder.TrimEnd('\') -ne $installPath.TrimEnd('\')) { throw "Run the installed uninstaller at $installPath\Uninstall.ps1. No files were removed." }
$exePath = Join-Path $installPath 'RaceDesk.exe'
if (Test-Path -LiteralPath (Join-Path $installPath 'unins000.exe')) {
    Start-Process -FilePath (Join-Path $installPath 'unins000.exe') -WindowStyle Normal
    return
}
Get-Process -Name RaceDesk -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exePath } | Stop-Process
Start-Sleep -Milliseconds 400
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$startupValue = (Get-ItemProperty -Path $runKey -Name RaceDesk -ErrorAction SilentlyContinue).RaceDesk
if ($startupValue -and $startupValue.Contains($exePath)) { Remove-ItemProperty -Path $runKey -Name RaceDesk -ErrorAction SilentlyContinue }
$shortcutPath = Join-Path ([Environment]::GetFolderPath('Programs')) 'RaceDesk.lnk'
if (Test-Path -LiteralPath $shortcutPath) { Remove-Item -LiteralPath $shortcutPath -Force }
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\RaceDesk'
if (Test-Path -LiteralPath $uninstallKey) { Remove-Item -LiteralPath $uninstallKey -Recurse -Force }
# Explicitly checked absolute, app-specific target; do not follow a computed external path.
if ((Split-Path -Leaf $installPath) -ne 'RaceDesk' -or (Split-Path -Parent $installPath) -ne (Join-Path $env:LOCALAPPDATA 'Programs')) { throw 'Refusing to remove an unexpected directory.' }
if (Test-Path -LiteralPath $installPath) { Remove-Item -LiteralPath $installPath -Recurse -Force }
if ($RemovePreferences) {
    $preferencesPath = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'RaceDesk\RaceDesk\data'))
    $expectedPreferencesPath = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'RaceDesk\RaceDesk\data'))
    if ($preferencesPath -ne $expectedPreferencesPath) { throw 'Unexpected preferences directory.' }
    if (Test-Path -LiteralPath $preferencesPath) { Remove-Item -LiteralPath $preferencesPath -Recurse -Force }
}
Write-Host 'RaceDesk removed. Saved preferences were kept unless RemovePreferences was specified.'
