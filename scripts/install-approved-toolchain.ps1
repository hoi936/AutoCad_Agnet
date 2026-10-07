[CmdletBinding()]
param([switch]$Install)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'windows-native.ps1')
if (!$Install) { Write-Output 'PLAN ONLY: rustup 1.29.1; pnpm 10.18.3; VS BuildTools 17.14.41 C++/SDK. Node/.NET existing retained.'; exit 0 }
$winget = (Get-Command winget.exe -ErrorAction Stop).Source
$items = @(
 @{ Id='Rustlang.Rustup'; Version='1.29.1'; Override='--quiet -y --default-host x86_64-pc-windows-msvc --default-toolchain stable --profile minimal --no-modify-path' },
 @{ Id='Microsoft.VisualStudio.2022.BuildTools'; Version='17.14.41'; Override='--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended' }
)
foreach ($item in $items) {
 Write-Output ('INSTALL ' + $item.Id + ' ' + $item.Version)
 $argsList=@('install','--exact','--id',$item.Id,'--version',$item.Version,'--source','winget','--accept-source-agreements','--accept-package-agreements','--disable-interactivity','--no-upgrade','--override',$item.Override)
 $result=Invoke-WindowsProcess $winget $argsList
 Write-Output $result.Output; Write-Output $result.Error; Write-Output ('EXIT '+$result.ExitCode)
 if ($result.ExitCode -ne 0 -and $result.ExitCode -ne 3010) { throw ('Installation failed: '+$item.Id+' '+$result.ExitCode) }
}
$node=Find-WindowsExecutable 'node'
$npm=Join-Path (Split-Path -Parent $node) 'node_modules/npm/bin/npm-cli.js'
$result=Invoke-WindowsProcess $node @($npm,'install','--global','pnpm@10.18.3','--registry=https://registry.npmjs.org')
Write-Output $result.Output; Write-Output $result.Error
if ($result.ExitCode -ne 0) { throw 'pnpm install failed' }
Write-Output 'Installers finished. Verify actual tools and build before claiming native readiness.'
