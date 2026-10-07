[CmdletBinding()]
param([switch]$SkipNativeBuild)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'windows-native.ps1')
$root=Split-Path -Parent $PSScriptRoot
$evidence=Join-Path $root 'docs/evidence/continue03-r1b'
New-Item -ItemType Directory -Force -Path $evidence | Out-Null
$cases=@(
 @{Name='format';Args=@('fmt','--all','--','--check')},
 @{Name='rust';Args=@('test','--locked','-p','cad-app-core','-p','cad-model-provider','-p','cad-native-vault','-p','cad-contracts')},
 @{Name='wincred';Args=@('test','--locked','-p','cad-model-provider','windows_credential_roundtrip_and_cleanup','--','--ignored')}
)
foreach($case in $cases){
 $result=Invoke-WindowsProcess (Find-WindowsExecutable 'cargo') $case.Args
 [IO.File]::WriteAllText((Join-Path $evidence ('windows-'+$case.Name+'.txt')),('Exit: '+$result.ExitCode+"`n"+$result.Output+$result.Error))
 Write-Output $result.Output;Write-Output $result.Error
 if($result.ExitCode -ne 0){exit $result.ExitCode}
}
& (Join-Path $PSScriptRoot 'verify-windows-build.ps1') -SkipBuild:$SkipNativeBuild -EvidenceDirectory 'docs/evidence/continue03-r1b'
exit $LASTEXITCODE
