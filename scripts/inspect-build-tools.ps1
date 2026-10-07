$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'windows-native.ps1')
$result=Invoke-WindowsProcess (Find-WindowsExecutable 'vswhere') @('-all','-products','*','-format','json')
Write-Output $result.Output
Get-Process -Name setup,vs_installer,vs_installershell -ErrorAction SilentlyContinue | Select-Object ProcessName,Id
Get-ChildItem -LiteralPath $env:TEMP -Filter 'dd_bootstrapper*' -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1 | ForEach-Object { Get-Content -LiteralPath $_.FullName -Tail 20 }
