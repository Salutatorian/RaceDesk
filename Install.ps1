param([switch]$NoLaunch)
$ErrorActionPreference = 'Stop'
$installPath = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Programs\RaceDesk'))
$expectedPath = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Programs\RaceDesk'))
if ($installPath -ne $expectedPath) { throw 'Unexpected install destination.' }
$sourcePath = Join-Path $PSScriptRoot 'app'
if (-not (Test-Path -LiteralPath (Join-Path $sourcePath 'RaceDesk.exe'))) { throw 'app\RaceDesk.exe is missing. Extract the complete RaceDesk download first.' }
Get-Process -Name RaceDesk -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq (Join-Path $installPath 'RaceDesk.exe') } | ForEach-Object { $_.Kill(); $_.WaitForExit() }
New-Item -ItemType Directory -Force -Path $installPath | Out-Null
Copy-Item -LiteralPath (Join-Path $sourcePath 'RaceDesk.exe') -Destination $installPath -Force
Copy-Item -LiteralPath (Join-Path $sourcePath 'collector') -Destination $installPath -Recurse -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Uninstall.ps1') -Destination $installPath -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'README.md') -Destination $installPath -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'THIRD-PARTY-NOTICES.txt') -Destination $installPath -Force
$shell = New-Object -ComObject WScript.Shell
$shortcutPath = Join-Path ([Environment]::GetFolderPath('Programs')) 'RaceDesk.lnk'
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = Join-Path $installPath 'RaceDesk.exe'
$shortcut.WorkingDirectory = $installPath
$shortcut.Description = 'Your iRacing desktop schedule and race reminders'
$shortcut.Save()
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\RaceDesk'
New-Item -Path $uninstallKey -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name DisplayName -Value 'RaceDesk' -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name DisplayVersion -Value '0.1.0' -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name Publisher -Value 'RaceDesk local app' -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name InstallLocation -Value $installPath -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name DisplayIcon -Value (Join-Path $installPath 'RaceDesk.exe') -PropertyType String -Force | Out-Null
$uninstallCommand = 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "' + (Join-Path $installPath 'Uninstall.ps1') + '"'
New-ItemProperty -Path $uninstallKey -Name UninstallString -Value $uninstallCommand -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name NoModify -Value 1 -PropertyType DWord -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name NoRepair -Value 1 -PropertyType DWord -Force | Out-Null
Write-Host "Installed RaceDesk to $installPath. Windows startup is controlled in Settings."
if (-not $NoLaunch) { Start-Process -FilePath (Join-Path $installPath 'RaceDesk.exe') -WorkingDirectory $installPath -WindowStyle Hidden }
