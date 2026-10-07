# Read-only prerequisite check. This script never installs tools or edits policy.
[CmdletBinding()]
param([ValidateSet('desktop', 'bridge')][string]$Profile = 'desktop')

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'windows-native.ps1')
$missing = New-Object System.Collections.Generic.List[string]
function Report([string]$Name, [bool]$Available, [string]$Detail) {
    if ($Available) { Write-Output "[PASS] $Name - $Detail" }
    else { Write-Output "[MISSING] $Name - $Detail"; $missing.Add($Name) }
}

if ($env:OS -ne 'Windows_NT') {
    Write-Output '[MISSING] Windows host - Run doctor.ps1 in Windows PowerShell, not WSL/Linux.'
    exit 1
}

$projectRoot = Split-Path -Parent $PSScriptRoot
$manifest = Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json
$expectedPnpm = $manifest.packageManager -replace '^pnpm@', ''
$lockText = Get-Content -LiteralPath (Join-Path $projectRoot 'pnpm-lock.yaml') -Raw
$nodeEngine = [regex]::Match($lockText, '(?ms)^  vite@[^:]+:\r?\n    resolution:.*?\r?\n    engines: \{node: ([^}]+)\}')
Report 'Reviewed Vite Node engine' ($nodeEngine.Success -and $nodeEngine.Groups[1].Value.Trim() -eq '^20.19.0 || >=22.12.0') 'Expected engine from locked Vite; changed requirements need review before installation.' 

Write-Output '[GROUP] Desktop development'
foreach ($tool in @('node','cargo','rustc','rustup')) {
    $found = Find-WindowsExecutable $tool
    Report $tool ([bool]$found) $(if ($found) { $found } else { 'Executable not found in Windows PATH or known installation directory.' })
}
$node = Find-WindowsExecutable 'node'
$pnpm = Find-WindowsPnpm
Report 'pnpm' ([bool]$pnpm) $(if ($pnpm) { $pnpm } else { 'Windows pnpm package not found.' })
if ($node) {
    $result = Invoke-WindowsProcess $node @('--version')
    if ($result.ExitCode -ne 0) { throw 'Node executable failed.' }
    $nodeVersion = $result.Output.Trim().TrimStart('v')
    $parsedNode = [version]$nodeVersion
    $supportedNode = ($parsedNode.Major -eq 20 -and $parsedNode -ge [version]'20.19.0') -or $parsedNode -ge [version]'22.12.0'
    Report 'Node/Vite compatibility' $supportedNode "Node $nodeVersion"
}
if ($pnpm -and $node) {
    $result = Invoke-WindowsProcess $node @($pnpm,'--version')
    Report 'pnpm version' ($result.ExitCode -eq 0 -and $result.Output.Trim() -eq $expectedPnpm) $result.Output.Trim()
}
$rustc = Find-WindowsExecutable 'rustc'
if ($rustc) {
    $result = Invoke-WindowsProcess $rustc @('-vV')
    $hostLine = $result.Output -split "`n" | Where-Object { $_ -match '^host: ' } | Select-Object -First 1
    $rustHost = if ($hostLine) { $hostLine.Substring(6).Trim() } else { '' }
    Report 'Rust MSVC host' ($result.ExitCode -eq 0 -and $rustHost -match '-pc-windows-msvc$') "Host: $rustHost"
}

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vsPath = ''
if (Test-Path -LiteralPath $vswhere) {
    $vsResult = Invoke-WindowsProcess $vswhere @('-latest','-products','*','-requires','Microsoft.VisualStudio.Component.VC.Tools.x86.x64','-property','installationPath')
    $vsPath = $vsResult.Output.Trim()
}
$linker = @()
if ($vsPath) {
    $linker = @(Get-ChildItem -Path (Join-Path $vsPath 'VC\Tools\MSVC\*\bin\Hostx64\x64\link.exe') -ErrorAction SilentlyContinue)
}
Report 'Visual Studio C++ MSVC x64' ($linker.Count -gt 0) $(if ($linker.Count) { $linker[0].FullName } else { 'VS Build Tools C++ x64 and linker were not located by vswhere.' })

$sdkRoot = (Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots' -ErrorAction SilentlyContinue).KitsRoot10
if (!$sdkRoot) { $sdkRoot = (Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows Kits\Installed Roots' -ErrorAction SilentlyContinue).KitsRoot10 }
if (!$sdkRoot) { $sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10' }
$sdkVersions = @()
if ($sdkRoot) {
    $sdkVersions = @(Get-ChildItem -LiteralPath (Join-Path $sdkRoot 'Include') -Directory -ErrorAction SilentlyContinue | Where-Object {
        (Test-Path -LiteralPath (Join-Path $_.FullName 'um\Windows.h')) -and
        (Test-Path -LiteralPath (Join-Path $_.FullName 'ucrt\stdio.h'))
    })
}
Report 'Windows SDK headers' ($sdkVersions.Count -gt 0) $(if ($sdkVersions.Count) { ($sdkVersions.Name -join ', ') } else { 'Windows SDK (UM and UCRT) headers not located.' })

$webviewVersions = @()
foreach ($registryPath in @('HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\*', 'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\*', 'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\*')) {
    $webviewVersions += @(Get-ItemProperty -Path $registryPath -ErrorAction SilentlyContinue | Where-Object {
        $_.name -match 'WebView2' -and $_.pv -and $_.pv -ne '0.0.0.0'
    } | ForEach-Object { $_.pv })
}
Report 'WebView2 Runtime' ($webviewVersions.Count -gt 0) $(if ($webviewVersions.Count) { ($webviewVersions | Select-Object -Unique) -join ', ' } else { 'WebView2 Evergreen Runtime not located in current-user or machine registry.' })

Write-Output '[GROUP] Repository dependencies and packaged artifacts'
$lockText = Get-Content -LiteralPath (Join-Path $projectRoot 'pnpm-lock.yaml') -Raw
$cliMatch = [regex]::Match($lockText, "(?m)^  '@tauri-apps/cli@([^']+)':")
$viteMatch = [regex]::Match($lockText, '(?m)^  vite@([^:]+):')
Write-Output ("Locked Tauri CLI: " + $cliMatch.Groups[1].Value + "; Vite: " + $viteMatch.Groups[1].Value)
$cliPackage = Join-Path $projectRoot 'apps/desktop/node_modules/@tauri-apps/cli/package.json'
$cliInstalled = Test-Path -LiteralPath $cliPackage
$cliVersionMatches = $false
if ($cliInstalled -and $cliMatch.Success) {
    $cliVersionMatches = ((Get-Content -LiteralPath $cliPackage -Raw | ConvertFrom-Json).version -eq $cliMatch.Groups[1].Value)
}
Report 'Locked Tauri CLI package' $cliVersionMatches 'Run pnpm install --frozen-lockfile in Windows; Linux node_modules is not a Windows install.'
$winPackages = @(Get-ChildItem -Path (Join-Path $projectRoot 'node_modules/.pnpm/@tauri-apps+cli-win32-*-msvc@*') -Directory -ErrorAction SilentlyContinue)
Report 'Windows Tauri CLI binary package' ($winPackages.Count -gt 0) 'Windows CLI platform dependency must be installed; presence alone does not prove launch.'
foreach ($relative in @('target/debug/cad-tool-host.exe', 'target/release/cad-tool-host.exe', 'apps/desktop/src-tauri/tauri.conf.json')) {
    $artifact = Join-Path $projectRoot $relative
    Write-Output ("[INFO] " + $relative + ': ' + (Test-Path -LiteralPath $artifact))
}
Write-Output '[GROUP] Bridge prerequisites (not required for mock desktop)'
$dotnet = Find-WindowsExecutable 'dotnet'
$sdks = @()
if ($dotnet) { $dotnetResult = Invoke-WindowsProcess $dotnet @('--list-sdks'); $sdks = @($dotnetResult.Output -split "`n" | Where-Object { $_.Trim() }) }
Write-Output ("[INFO] .NET SDKs: " + $(if ($sdks.Count) { $sdks -join '; ' } else { 'none observed' }))
$bridgeProfile = Join-Path $projectRoot 'bridge/profile.json'
Write-Output ("[INFO] Verified bridge profile: " + (Test-Path -LiteralPath $bridgeProfile))
if ($Profile -eq 'bridge') {
    Report '.NET SDK' ($sdks.Count -gt 0) 'Select SDK only after verifying the target AutoCAD runtime.'
    Report 'Verified bridge profile' (Test-Path -LiteralPath $bridgeProfile) 'No runtime inferred from an AutoCAD folder name.'
}
$registrations = @()
foreach ($registry in @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*', 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*', 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*')) {
    $registrations += @(Get-ItemProperty -Path $registry -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -match 'AutoCAD' })
}
if (!$registrations.Count) { Write-Output '[INFO] AutoCAD installation: none observed in uninstall registry.' }
foreach ($registration in $registrations) {
    Write-Output ("[OBSERVED] " + $registration.DisplayName + '; registered version=' + $registration.DisplayVersion)
    if ($registration.InstallLocation) {
        $acad = Join-Path $registration.InstallLocation 'acad.exe'
        if (Test-Path -LiteralPath $acad) {
            $installDir = Split-Path -Parent $acad
            $runtimeConfig = Join-Path $installDir 'acad.runtimeconfig.json'
            if (Test-Path -LiteralPath $runtimeConfig) {
                $runtime = Get-Content -LiteralPath $runtimeConfig -Raw | ConvertFrom-Json
                Write-Output ('[OBSERVED] acad runtimeconfig: ' + ($runtime.runtimeOptions | ConvertTo-Json -Compress -Depth 5))
            }
            foreach ($assembly in @('AcMgd.dll','AcDbMgd.dll','AcCoreMgd.dll')) {
                $reference = Join-Path $installDir $assembly
                if (Test-Path -LiteralPath $reference) {
                    try { $assemblyName = [System.Reflection.AssemblyName]::GetAssemblyName($reference)
                        Write-Output ('[OBSERVED] reference ' + $assembly + '; assembly version=' + $assemblyName.Version)
                    } catch { Write-Output ('[INFO] Cannot read managed assembly metadata: ' + $assembly) }
                }
            }
            $version = (Get-Item -LiteralPath $acad).VersionInfo
            Write-Output ("[OBSERVED] acad.exe product=" + $version.ProductVersion + '; file=' + $version.FileVersion)
        }
    }
}
$processes = @(Get-Process -Name acad -ErrorAction SilentlyContinue)
Write-Output ("[INFO] acad processes observed: " + $processes.Count + '; installed/running does not mean paired or connected.')
$bundle = Join-Path $projectRoot 'bridge/CadAgent.bundle/PackageContents.xml'
Write-Output ("[INFO] Bridge bundle manifest: " + (Test-Path -LiteralPath $bundle))
Write-Output '[INFO] Plugin handshake/readback not performed. AutoCAD write readiness is unverified.'

Write-Output 'Doctor only checks prerequisites. It does not prove native build, app startup, credential access, or installer operation.'
if ($missing.Count -gt 0) {
    Write-Output ("Missing prerequisites: " + ($missing -join ', '))
    exit 1
}
Write-Output 'Prerequisite checks passed. Continue with pnpm dev and native smoke tests.'
exit 0
