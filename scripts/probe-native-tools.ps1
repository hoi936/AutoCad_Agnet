$ErrorActionPreference='Stop'
function Probe([string]$Executable,[string]$Arguments) {
 Write-Output ("PROBE " + $Executable + ' ' + $Arguments)
 $start=New-Object System.Diagnostics.ProcessStartInfo
 $start.FileName=$Executable; $start.Arguments=$Arguments
 $start.UseShellExecute=$false; $start.RedirectStandardOutput=$true; $start.RedirectStandardError=$true
 $p=[System.Diagnostics.Process]::Start($start)
 $out=$p.StandardOutput.ReadToEndAsync(); $err=$p.StandardError.ReadToEndAsync()
 if (!$p.WaitForExit(45000)) { $p.Kill(); throw 'Probe timeout' }
 Write-Output $out.Result; Write-Output $err.Result; Write-Output ("EXIT " + $p.ExitCode);$p.Dispose()
}
Probe 'C:\Program Files\nodejs\node.exe' '--version'
Probe 'C:\Program Files\dotnet\dotnet.exe' '--list-sdks'
Probe (Join-Path $env:LOCALAPPDATA 'Microsoft\WindowsApps\winget.exe') 'show --id Rustlang.Rustup --exact --source winget --accept-source-agreements --disable-interactivity'
Probe (Join-Path $env:LOCALAPPDATA 'Microsoft\WindowsApps\winget.exe') 'show --id Microsoft.VisualStudio.2022.BuildTools --exact --source winget --accept-source-agreements --disable-interactivity'
