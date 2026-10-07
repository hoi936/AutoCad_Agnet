[CmdletBinding()]
param([string]$EvidenceDirectory='docs/evidence/continue03', [string]$BuildLabel='Chat R1')
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$source=Join-Path $root 'target/debug'
$hash=(Get-FileHash -LiteralPath (Join-Path $source 'cad-agent-studio.exe') -Algorithm SHA256).Hash
$destination=Join-Path $root ('target/native-smoke/'+$BuildLabel+' '+$hash.Substring(0,12))
New-Item -ItemType Directory -Force -Path $destination | Out-Null
$binary=Join-Path $destination 'cad-agent-studio.exe'
if (@(Get-Process -Name cad-agent-studio -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $binary }).Count) {
 Write-Output ('Already running: '+$binary);exit 0
}
Copy-Item -LiteralPath (Join-Path $source 'cad-agent-studio.exe') -Destination $binary
Copy-Item -LiteralPath (Join-Path $source 'cad-tool-host.exe') -Destination (Join-Path $destination 'cad-tool-host.exe')
$start=New-Object System.Diagnostics.ProcessStartInfo
$start.FileName=$binary;$start.WorkingDirectory=$destination;$start.UseShellExecute=$false
$process=[System.Diagnostics.Process]::Start($start)
for ($i=0;$i -lt 30;$i++) {
 Start-Sleep -Milliseconds 500;$process.Refresh()
 if ($process.HasExited) { throw ('App exited: '+$process.ExitCode) }
 if ($process.MainWindowHandle -ne 0) { break }
}
Start-Sleep -Seconds 2
$process.Refresh()
$children=@(Get-CimInstance Win32_Process -Filter ("ParentProcessId = "+$process.Id) | Where-Object { $_.Name -eq 'cad-tool-host.exe' })
$report=[pscustomobject]@{BinaryPath=$binary;SHA256=$hash;Bytes=(Get-Item -LiteralPath $binary).Length;ProcessId=$process.Id;WindowTitle=$process.MainWindowTitle;HasWindow=($process.MainWindowHandle -ne 0);Responding=$process.Responding;MockSidecarChildren=$children.Count;Scope='Native unconnected startup; no login, no DWG, no UI E2E'}
$json=$report|ConvertTo-Json
[IO.File]::WriteAllText((Join-Path (Join-Path $root $EvidenceDirectory) 'windows-startup.json'),$json)
Write-Output $json
if (!$report.HasWindow -or $children.Count -ne 0) { throw 'Unconnected native startup gate failed.' }
