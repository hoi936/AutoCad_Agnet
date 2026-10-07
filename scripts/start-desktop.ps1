[CmdletBinding()]
param([switch]$CheckOnly)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'windows-native.ps1')
if ($env:OS -ne 'Windows_NT') { throw 'Run from Windows PowerShell.' }
$root = Split-Path -Parent $PSScriptRoot
& (Join-Path $PSScriptRoot 'doctor.ps1') -Profile desktop
if ($LASTEXITCODE -ne 0) { Write-Error 'Prerequisites missing. Read doctor output and setup-windows.ps1 plan. Nothing started.'; exit 1 }
if ($CheckOnly) { Write-Output 'Checks passed; no app/build started.'; exit 0 }
Push-Location -LiteralPath $root
try {
    $run = Invoke-WindowsProcess (Find-WindowsExecutable 'node') @((Find-WindowsPnpm),'dev')
    Write-Output $run.Output; Write-Output $run.Error
    $result = $run.ExitCode
} finally { Pop-Location }
exit $result
