# No installers, native app or compilation are started by this suite.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$passed = 0
foreach ($file in (Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.ps1' | ForEach-Object { $_.Name })) {
    $tokens = $null; $parseErrors = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot $file), [ref]$tokens, [ref]$parseErrors)
    if ($parseErrors.Count) { throw ("Parse failed: " + $file + ': ' + ($parseErrors.Message -join '; ')) }
    $passed++; Write-Output ("PASS parse " + $file)
}
function CheckSetup([string[]]$Arguments, [int]$ExpectedExit, [string]$ExpectedText) {
    $start = New-Object System.Diagnostics.ProcessStartInfo
    $start.FileName = Join-Path $PSHOME 'powershell.exe'
    $start.Arguments = '-NoProfile -File "' + (Join-Path $PSScriptRoot 'setup-windows.ps1') + '" ' + ($Arguments -join ' ')
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::Start($start)
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    if (!$process.WaitForExit(15000)) { $process.Kill(); throw 'Setup fixture timed out.' }
    $output = @($stdout.Result, $stderr.Result)
    $code = $process.ExitCode
    $process.Dispose()
    if ($code -ne $ExpectedExit -or ($output -join "`n") -notmatch $ExpectedText) {
        throw ("Unexpected setup result: exit=" + $code + '; output=' + ($output -join "`n"))
    }
    $script:passed++; Write-Output ("PASS setup " + ($Arguments -join ' ') + ' -> ' + $ExpectedExit)
}
CheckSetup -Arguments @() -ExpectedExit 0 -ExpectedText 'PLAN ONLY'
CheckSetup -Arguments @('-Install') -ExpectedExit 1 -ExpectedText 'specific -Component'
CheckSetup -Arguments @('-Install','-Component','node') -ExpectedExit 1 -ExpectedText 'exact numeric -Version'
CheckSetup -Arguments @('-Install','-Component','node','-Version','18.20.0') -ExpectedExit 1 -ExpectedText 'does not satisfy'
CheckSetup -Arguments @('-Install','-Component','node','-Version','22.12.0','-WhatIf') -ExpectedExit 0 -ExpectedText 'WHATIF: winget install'
CheckSetup -Arguments @('-Install','-Component','pnpm','-Version','1.0.0') -ExpectedExit 1 -ExpectedText 'must match packageManager'
Write-Output ("PASS: " + $passed + ' checks; no installation performed.')

. (Join-Path $PSScriptRoot 'windows-native.ps1')
$node = Find-WindowsExecutable 'node'
if ($node) {
    $result = Invoke-WindowsProcess $node @('-e','process.stdout.write(JSON.stringify(process.argv.slice(1))); process.exit(7);','path with spaces','Unicode tấm đế','quote"value','trailing\')
    if ($result.ExitCode -ne 7) { throw 'Native exit code was not preserved.' }
    $actual = $result.Output | ConvertFrom-Json
    if ($actual[0] -ne 'path with spaces' -or $actual[1] -ne 'Unicode tấm đế' -or $actual[2] -ne 'quote"value' -or $actual[3] -ne 'trailing\') { throw 'Native argument quoting corrupted input.' }
    Write-Output 'PASS Windows native exit code and argument quoting.'
}
