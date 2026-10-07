param([switch]$WhatIf)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$bridge = Join-Path $root 'bridges/autocad'
$project = Join-Path $bridge 'CadAgent.Bridge.AutoCAD/CadAgent.Bridge.AutoCAD.csproj'
$bundle = Join-Path $bridge 'CadAgent.AutoCAD.bundle'
$out = Join-Path $bundle 'Contents/Windows'
$acad = 'C:\Program Files\Autodesk\AutoCAD 2026'
if (!(Test-Path -LiteralPath (Join-Path $acad 'acad.exe'))) { throw 'AutoCAD 2026 not installed at expected path.' }
if (!(Test-Path -LiteralPath (Join-Path $acad 'AcMgd.dll'))) { throw 'AutoCAD managed reference missing.' }
if ($WhatIf) { Write-Output "Would build $project and stage only project DLLs in $out"; return }
. (Join-Path $PSScriptRoot 'windows-native.ps1')
$dotnet = Find-WindowsExecutable 'dotnet'
if (!$dotnet) { throw 'dotnet SDK not found.' }
$build = Invoke-WindowsProcess $dotnet @('build', $project, '-c', 'Release')
Write-Output $build.Output
if ($build.ExitCode -ne 0) { Write-Output $build.Error; throw 'Plugin build failed.' }
$source = Join-Path $bridge 'CadAgent.Bridge.AutoCAD/bin/Release/net10.0-windows'
New-Item -ItemType Directory -Path $out -Force | Out-Null
foreach ($name in @('CadAgent.Bridge.AutoCAD.dll','CadAgent.Bridge.Protocol.dll','CadAgent.Bridge.Transport.dll')) {
    Copy-Item -LiteralPath (Join-Path $source $name) -Destination (Join-Path $out $name) -Force
}
Write-Output "Staged bundle: $bundle"
