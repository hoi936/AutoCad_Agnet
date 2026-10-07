param([switch]$WhatIf)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$source = Join-Path $root 'bridges/autocad/CadAgent.AutoCAD.bundle'
$manifest = Join-Path $source 'PackageContents.xml'
$destination = Join-Path $env:APPDATA 'Autodesk/ApplicationPlugins/CadAgent.AutoCAD.bundle'
if (!(Test-Path -LiteralPath $manifest)) { throw 'PackageContents.xml missing.' }
[xml]$package = Get-Content -LiteralPath $manifest -Raw
if ($package.ApplicationPackage.ProductCode -ne '{D9837021-48E8-4BB3-AD70-44A010A08B0E}') { throw 'Unexpected bundle identity.' }
foreach ($name in @('CadAgent.Bridge.AutoCAD.dll','CadAgent.Bridge.Protocol.dll','CadAgent.Bridge.Transport.dll')) {
    if (!(Test-Path -LiteralPath (Join-Path $source "Contents/Windows/$name"))) { throw "Missing staged $name; run autocad-bridge-build.ps1 first." }
}
if ($WhatIf) { Write-Output "Would install user-scope bundle: $destination"; return }
if (Test-Path -LiteralPath $destination) {
    [xml]$installed = Get-Content -LiteralPath (Join-Path $destination 'PackageContents.xml') -Raw
    if ($installed.ApplicationPackage.ProductCode -ne $package.ApplicationPackage.ProductCode) { throw 'Destination belongs to another bundle.' }
    Remove-Item -LiteralPath $destination -Recurse -Force
}
Copy-Item -LiteralPath $source -Destination $destination -Recurse
Write-Output "Installed user-scope bundle: $destination"
Write-Output 'AutoCAD may need a restart to discover the bundle. Keep SECURELOAD enabled; approve this specific plugin if prompted.'
