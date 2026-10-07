[CmdletBinding()]
param([switch]$SkipBuild, [string]$EvidenceDirectory='docs/evidence/continue02')
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'windows-native.ps1')
$root=Split-Path -Parent $PSScriptRoot
$evidence=Join-Path $root $EvidenceDirectory
New-Item -ItemType Directory -Force -Path $evidence | Out-Null
$node=Find-WindowsExecutable 'node';$pnpm=Find-WindowsPnpm
$result=Invoke-WindowsProcess (Find-WindowsExecutable 'rustc') @('-vV')
Write-Output $result.Output
foreach ($task in @('typecheck','lint','test','build')) {
 Write-Output ('RUN pnpm '+$task)
 $result=Invoke-WindowsProcess $node @($pnpm,$task)
 [System.IO.File]::WriteAllText((Join-Path $evidence ('windows-'+$task+'.txt')),('Exit: '+$result.ExitCode+"`n"+$result.Output+$result.Error))
 Write-Output $result.Output;Write-Output $result.Error
 if ($result.ExitCode -ne 0) { exit $result.ExitCode }
}
if (!$SkipBuild) {
 Write-Output 'RUN Tauri debug native build (includes packaged sidecar preparation)'
 $result=Invoke-WindowsProcess $node @($pnpm,'--filter','@cad/desktop','tauri','build','--debug','--no-bundle')
 [System.IO.File]::WriteAllText((Join-Path $evidence 'windows-native-build.txt'),('Exit: '+$result.ExitCode+"`n"+$result.Output+$result.Error))
 Write-Output $result.Output;Write-Output $result.Error
 exit $result.ExitCode
}
