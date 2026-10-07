[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$source=Join-Path $root 'target/debug'
$binary=Join-Path $source 'cad-agent-studio.exe'
$sidecar=Join-Path $source 'cad-tool-host.exe'
if (!(Test-Path -LiteralPath $binary) -or !(Test-Path -LiteralPath $sidecar)) { throw 'Native app and sidecar must be built before launch.' }
$existing=@(Get-Process -Name cad-agent-studio -ErrorAction SilentlyContinue)
if ($existing.Count) { Write-Output 'Native app already running; no second instance started.'; exit 0 }
$installed=Join-Path $root 'target/native-smoke/Bản chạy thử'
New-Item -ItemType Directory -Force -Path $installed | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $installed 'cad-agent-studio.exe')
Copy-Item -LiteralPath $sidecar -Destination (Join-Path $installed 'cad-tool-host.exe')
$binary=Join-Path $installed 'cad-agent-studio.exe'; $sidecar=Join-Path $installed 'cad-tool-host.exe'
$start=New-Object System.Diagnostics.ProcessStartInfo
$start.FileName=$binary; $start.WorkingDirectory=Split-Path -Parent $binary
$start.UseShellExecute=$false
$p=[System.Diagnostics.Process]::Start($start)
for ($i=0; $i -lt 30; $i++) {
 Start-Sleep -Milliseconds 500
 $p.Refresh()
 if ($p.HasExited) { throw ('Native app exited early: '+$p.ExitCode) }
 if ($p.MainWindowHandle -ne 0) { break }
}
$p.Refresh()
$report=[pscustomobject]@{ BinaryPath=$binary; ProcessId=$p.Id; MainWindowTitle=$p.MainWindowTitle; HasWindow=($p.MainWindowHandle -ne 0); Responding=$p.Responding; AppSHA256=(Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash; AppBytes=(Get-Item -LiteralPath $binary).Length; SidecarSHA256=(Get-FileHash -LiteralPath $sidecar -Algorithm SHA256).Hash; SidecarBytes=(Get-Item -LiteralPath $sidecar).Length; Scope='Native startup only; not full IPC E2E or AutoCAD live' }
$json = $report | ConvertTo-Json
[IO.File]::WriteAllText((Join-Path $root 'docs/evidence/continue02/windows-native-startup.json'), $json)
Write-Output $json
if (!$report.HasWindow) { throw 'Process remains running but no application window was observed.' }
