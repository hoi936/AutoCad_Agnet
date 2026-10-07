[CmdletBinding()]
param([switch]$Install)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'windows-native.ps1')
$vswhere=Find-WindowsExecutable 'vswhere'
$query=Invoke-WindowsProcess $vswhere @('-latest','-products','Microsoft.VisualStudio.Product.BuildTools','-property','installationPath')
$installPath=$query.Output.Trim()
if (!$installPath) { throw 'Build Tools instance not found.' }
$ready=Invoke-WindowsProcess $vswhere @('-latest','-products','*','-requires','Microsoft.VisualStudio.Component.VC.Tools.x86.x64','-property','installationPath')
if ($ready.Output.Trim()) { Write-Output 'C++ tools already present; no modification.'; exit 0 }
Write-Output ('PLAN: add Microsoft.VisualStudio.Workload.VCTools and recommended SDK to '+$installPath)
if (!$Install) { exit 0 }
$setup=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/setup.exe'
$result=Invoke-WindowsProcess $setup @('modify','--channelId','VisualStudio.17.Release','--installPath',$installPath,'--add','Microsoft.VisualStudio.Workload.VCTools','--includeRecommended','--passive','--norestart')
Write-Output $result.Output; Write-Output $result.Error; Write-Output ('EXIT '+$result.ExitCode)
exit $result.ExitCode
