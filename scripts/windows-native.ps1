[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
function Invoke-WindowsProcess {
    param([string]$Executable, [string[]]$ArgumentList = @())
    $start = New-Object System.Diagnostics.ProcessStartInfo
    $start.FileName = $Executable
    $quoted = foreach ($argument in $ArgumentList) {
        '"' + ([regex]::Replace([regex]::Replace($argument, '(\\*)"', '$1$1\"'), '(\\+)$', '$1$1')) + '"'
    }
    $start.Arguments = $quoted -join ' '
    $start.WorkingDirectory = Split-Path -Parent $PSScriptRoot
    $toolPaths = @((Join-Path $env:ProgramFiles 'nodejs'), (Join-Path $env:USERPROFILE '.cargo/bin'), (Join-Path $env:APPDATA 'npm'))
    # WSL-launched PowerShell may inherit PATHEXT=.CPL; restore the Windows
    # machine value only for this child, without changing registry/user settings.
    $start.EnvironmentVariables['PATHEXT'] = [Environment]::GetEnvironmentVariable('PATHEXT','Machine')
    $start.EnvironmentVariables['PATH'] = ($toolPaths -join ';') + ';' + $env:PATH
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $start.StandardErrorEncoding = [System.Text.Encoding]::UTF8
    $process = [System.Diagnostics.Process]::Start($start)
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    $result = [pscustomobject]@{ ExitCode = $process.ExitCode; Output = $stdout.Result; Error = $stderr.Result }
    $process.Dispose()
    return $result
}
function Find-WindowsExecutable([string]$Name) {
    $command = Get-Command ($Name + '.exe') -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    $candidates = @()
    if ($Name -eq 'node') { $candidates += (Join-Path $env:ProgramFiles 'nodejs/node.exe') }
    if ($Name -eq 'dotnet') { $candidates += (Join-Path $env:ProgramFiles 'dotnet/dotnet.exe') }
    if ($Name -in @('cargo','rustc','rustup')) { $candidates += (Join-Path $env:USERPROFILE ('.cargo/bin/' + $Name + '.exe')) }
    if ($Name -eq 'vswhere') { $candidates += (Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe') }
    foreach ($candidate in $candidates) { if (Test-Path -LiteralPath $candidate) { return $candidate } }
    return $null
}

function Find-WindowsPnpm {
    foreach ($path in @((Join-Path $env:APPDATA 'npm/node_modules/pnpm/bin/pnpm.cjs'), (Join-Path $env:ProgramFiles 'nodejs/node_modules/pnpm/bin/pnpm.cjs'))) {
        if (Test-Path -LiteralPath $path) { return $path }
    }
    return $null
}
