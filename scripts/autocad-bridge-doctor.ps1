$ErrorActionPreference = 'Stop'
$acad = 'C:\Program Files\Autodesk\AutoCAD 2026'
$destination = Join-Path $env:APPDATA 'Autodesk/ApplicationPlugins/CadAgent.AutoCAD.bundle'
$process = Get-CimInstance Win32_Process -Filter "name='acad.exe'" | Select-Object ProcessId,ExecutablePath
$files = @('acad.exe','AcMgd.dll','AcDbMgd.dll','AcCoreMgd.dll') | ForEach-Object {
    $path = Join-Path $acad $_
    [pscustomobject]@{ name=$_; exists=Test-Path -LiteralPath $path; version=if (Test-Path -LiteralPath $path) {(Get-Item -LiteralPath $path).VersionInfo.ProductVersion} else {$null} }
}
$manifest = Join-Path $destination 'PackageContents.xml'
[pscustomobject]@{ scope='user'; installed=Test-Path -LiteralPath $manifest; bundle=$destination; processes=@($process); files=@($files) } | ConvertTo-Json -Depth 5
