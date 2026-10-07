# Default is a read-only plan. -Install is explicit consent for one selected installer.
[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = 'High')]
param(
    [switch]$Install,
    [ValidateSet('node','rustup','build-tools','webview2','pnpm')][string]$Component,
    [string]$Version
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'windows-native.ps1')
if ($env:OS -ne 'Windows_NT') { throw 'Use Windows PowerShell, not a Linux toolchain.' }
$root = Split-Path -Parent $PSScriptRoot
$manifest = Get-Content -LiteralPath (Join-Path $root 'package.json') -Raw | ConvertFrom-Json
$pnpmVersion = $manifest.packageManager -replace '^pnpm@', ''
$lockText = Get-Content -LiteralPath (Join-Path $root 'pnpm-lock.yaml') -Raw
$nodeEngine = [regex]::Match($lockText, '(?ms)^  vite@[^:]+:\r?\n    resolution:.*?\r?\n    engines: \{node: ([^}]+)\}')
if (!$nodeEngine.Success -or $nodeEngine.Groups[1].Value.Trim() -ne '^20.19.0 || >=22.12.0') {
    throw 'Locked Vite engine changed; review setup compatibility before installing.'
}
$packages = @{
    node = 'OpenJS.NodeJS.LTS'
    rustup = 'Rustlang.Rustup'
    'build-tools' = 'Microsoft.VisualStudio.2022.BuildTools'
    webview2 = 'Microsoft.EdgeWebView2Runtime'
}
Write-Output 'PLAN: no AutoCAD upgrade, security-policy changes, broad trusted paths, or application elevation.'
Write-Output "Node: choose an explicit version compatible with locked Vite (20.19+ or 22.12+); pnpm: $pnpmVersion."
Write-Output 'Rust: MSVC host; install rustup then verify rustc/cargo and host target.'
Write-Output 'Build Tools: C++ workload including MSVC and Windows SDK. WebView2: reuse if installed.'
Write-Output '.NET/AutoCAD: choose a verified bridge profile first; this script does not install or alter AutoCAD.'
Write-Output 'Official sources: https://nodejs.org/ https://rustup.rs/ https://visualstudio.microsoft.com/downloads/ https://developer.microsoft.com/microsoft-edge/webview2/'
if (!$Install) {
    Write-Output 'PLAN ONLY. Run doctor.ps1. No installer was started.'
    foreach ($key in ($packages.Keys | Sort-Object)) { Write-Output ("Package candidate: " + $packages[$key]) }
    Write-Output 'After explicit approval: use -Install -Component <name> -Version <reviewed exact version>; inspect winget show first.'
    exit 0
}
if (!$Component) { throw '-Install requires a specific -Component; bulk installation is not implicit.' }
if ($Component -eq 'pnpm') {
    if ($Version -and $Version -ne $pnpmVersion) { throw 'pnpm must match packageManager.' }
    if (!(Find-WindowsExecutable 'node')) { throw 'Install Node for Windows and reopen terminal first.' }
    if ($PSCmdlet.ShouldProcess("pnpm@$pnpmVersion", 'Install using npm official registry')) {
        $node = Find-WindowsExecutable 'node'
        $npm = Join-Path (Split-Path -Parent $node) 'node_modules/npm/bin/npm-cli.js'
        $result = Invoke-WindowsProcess $node @($npm,'install','--global',"pnpm@$pnpmVersion",'--registry=https://registry.npmjs.org')
        Write-Output $result.Output; Write-Output $result.Error
        if ($result.ExitCode -ne 0) { throw "npm install failed: $($result.ExitCode)" }
        $check = Invoke-WindowsProcess $node @((Find-WindowsPnpm),'--version')
        $actual = $check.Output.Trim()
        if ($actual -ne $pnpmVersion) { throw 'Installed pnpm is not available or does not match packageManager; reopen terminal and run doctor.' }
    }
    exit 0
}
if (!$Version -or $Version -notmatch '^\d+(\.\d+){1,3}$') { throw 'An exact numeric -Version reviewed with winget show is required. No latest fallback.' }
if ($Component -eq 'node') {
    $v = [version]$Version
    if (!(($v.Major -eq 20 -and $v -ge [version]'20.19.0') -or $v -ge [version]'22.12.0')) { throw 'Node version does not satisfy locked Vite engine.' }
}
if ($WhatIfPreference) { Write-Output ("WHATIF: winget install --exact --id " + $packages[$Component] + ' --version ' + $Version); exit 0 }
if (!(Get-Command winget.exe -ErrorAction SilentlyContinue)) { throw 'WinGet unavailable. Use the official installer page; no alternate download is attempted.' }
$id = $packages[$Component]
$winget = (Get-Command winget.exe).Source
$show = Invoke-WindowsProcess $winget @('show','--exact','--id',$id,'--version',$Version,'--source','winget','--disable-interactivity')
Write-Output $show.Output; Write-Output $show.Error
if ($show.ExitCode -ne 0) { throw 'Specified package/version cannot be verified by WinGet; nothing installed.' }
if ($PSCmdlet.ShouldProcess("$id version $Version", 'Install Windows prerequisite from reviewed WinGet manifest')) {
    $installArgs = @('install','--exact','--id',$id,'--version',$Version,'--source','winget','--no-upgrade')
    if ($Component -eq 'build-tools') { $installArgs += @('--override','--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended') }
    $installed = Invoke-WindowsProcess $winget $installArgs
    Write-Output $installed.Output; Write-Output $installed.Error
    if ($installed.ExitCode -eq 3010) { Write-Output 'Installer requires a restart; do not assume the toolchain is ready.'; exit 3010 }
    if ($installed.ExitCode -ne 0) { throw "Installer failed: $($installed.ExitCode)" }
    $listed = Invoke-WindowsProcess $winget @('list','--exact','--id',$id,'--source','winget','--disable-interactivity')
    Write-Output $listed.Output; Write-Output $listed.Error
    if ($listed.ExitCode -ne 0) { throw 'Installer returned success but installed package was not confirmed.' }
    Write-Output 'Reopen Windows terminal and run doctor.ps1; installer success is not native build success.'
}
