$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'windows-native.ps1')
$root=Split-Path -Parent $PSScriptRoot
$cases=@(
 @{Name='core';Args=@('test:core')},
 @{Name='mcp';Args=@('test:mcp')}
)
foreach ($case in $cases) {
 Write-Output ('RUN '+$case.Name)
 $result=Invoke-WindowsProcess (Find-WindowsExecutable 'node') (@((Find-WindowsPnpm)) + $case.Args)
 [IO.File]::WriteAllText((Join-Path $root ('docs/evidence/continue02/windows-'+$case.Name+'.txt')),('Exit: '+$result.ExitCode+"`n"+$result.Output+$result.Error))
 Write-Output $result.Output;Write-Output $result.Error
 if ($result.ExitCode -ne 0) { exit $result.ExitCode }
}
Write-Output 'RUN Windows credential fixture in unique namespace with cleanup'
$result=Invoke-WindowsProcess (Find-WindowsExecutable 'cargo') @('test','--locked','-p','cad-model-provider','windows_credential_roundtrip_and_cleanup','--','--ignored')
[IO.File]::WriteAllText((Join-Path $root 'docs/evidence/continue02/windows-vault.txt'),('Exit: '+$result.ExitCode+"`n"+$result.Output+$result.Error))
Write-Output $result.Output;Write-Output $result.Error
if ($result.ExitCode -ne 0) { exit $result.ExitCode }
# Restore a normal executable after fault-injection tests, before users launch debug desktop.
$result=Invoke-WindowsProcess (Find-WindowsExecutable 'cargo') @('build','--locked','-p','cad-tool-host')
Write-Output $result.Output;Write-Output $result.Error
exit $result.ExitCode
