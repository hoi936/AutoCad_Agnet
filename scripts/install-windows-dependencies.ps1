$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'windows-native.ps1')
$result=Invoke-WindowsProcess (Find-WindowsExecutable 'node') @((Find-WindowsPnpm),'install','--frozen-lockfile')
Write-Output $result.Output; Write-Output $result.Error
exit $result.ExitCode
