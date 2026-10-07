param([switch]$WhatIf)
$ErrorActionPreference = 'Stop'
$destination = Join-Path $env:APPDATA 'Autodesk/ApplicationPlugins/CadAgent.AutoCAD.bundle'
if (!(Test-Path -LiteralPath $destination)) { Write-Output 'CAD Agent bundle not installed.'; return }
$manifest = Join-Path $destination 'PackageContents.xml'
if (!(Test-Path -LiteralPath $manifest)) { throw 'Cannot verify bundle identity.' }
[xml]$installed = Get-Content -LiteralPath $manifest -Raw
if ($installed.ApplicationPackage.ProductCode -ne '{D9837021-48E8-4BB3-AD70-44A010A08B0E}') { throw 'Refusing to remove another bundle.' }
if ($WhatIf) { Write-Output "Would remove user-scope bundle: $destination"; return }
Remove-Item -LiteralPath $destination -Recurse -Force
Write-Output "Removed user-scope bundle: $destination"
