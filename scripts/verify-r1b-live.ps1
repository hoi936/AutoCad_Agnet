[CmdletBinding()]
param([switch]$Live)
$ErrorActionPreference='Stop'
if (!$Live) { throw 'Pass -Live only after browser login and consent. This runs real inference using the saved ChatGPT profile; close the desktop first.' }
$root=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $root
. (Join-Path $PSScriptRoot 'windows-native.ps1')
$evidence=Join-Path $root 'docs/evidence/continue03-r1b'
New-Item -ItemType Directory -Force -Path $evidence | Out-Null
$result=Invoke-WindowsProcess (Find-WindowsExecutable 'cargo') @('run','--locked','-p','cad-app-core','--example','verify_live','--','--live',(Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'vn.cadagent.studio'),(Join-Path $evidence 'windows-live-runtime.json'))
[IO.File]::WriteAllText((Join-Path $evidence 'windows-live-runtime.txt'),('Exit: '+$result.ExitCode+"`n"+$result.Output+$result.Error))
Write-Output $result.Output;Write-Output $result.Error
exit $result.ExitCode
