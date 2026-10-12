<#
    Shared helpers for every debris script.

    This module exists to enforce two rules so individual scripts cannot forget them:

    1. A script never acts when -DryRun is set. Not "checks a flag and mostly skips" —
       every mutating call goes through Invoke-DebrisStep or a helper built on it, and
       all of them return early in dry-run mode.
    2. A script never elevates itself. When a script needs administrator rights it
       calls Assert-DebrisAdmin, which throws a legible error rather than opening a
       UAC prompt behind the user's back.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:DryRunActive = $false
$script:StepCount = 0

function Use-DebrisMode {
    [CmdletBinding()]
    param([switch]$DryRun)

    $script:DryRunActive = $DryRun.IsPresent
    $mode = if ($script:DryRunActive) {
        'DRY RUN - nothing will change'
    }
    else {
        'LIVE - changes will be applied'
    }
    Write-DebrisLine ''
    Write-DebrisLine "debris: $mode"
    Write-DebrisLine ''
}

function Test-DebrisDryRun {
    [CmdletBinding()]
    param()
    return $script:DryRunActive
}

function Write-DebrisLine {
    [CmdletBinding()]
    param([string]$Message = '')
    Write-Host $Message
}

function Write-DebrisStep {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$Message)

    $script:StepCount++
    $prefix = if (Test-DebrisDryRun) { 'would ' } else { '' }
    Write-DebrisLine ("  {0}{1}" -f $prefix, $Message)
}

function Invoke-DebrisStep {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$Description,
        [Parameter(Mandatory)][scriptblock]$Action
    )

    Write-DebrisStep $Description
    if (Test-DebrisDryRun) { return }
    & $Action
}

function Assert-DebrisAdmin {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$Reason)

    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    $isAdmin = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    if (-not $isAdmin) {
        throw "Needs administrator rights to $Reason. Nothing was changed. Re-run debris as administrator."
    }
}

function Assert-DebrisProcessElevated {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$Reason)

    # Alias kept separate so the intent reads clearly at call sites that care about
    # elevation specifically as a precondition, not as a permission check.
    Assert-DebrisAdmin -Reason $Reason
}

function Test-DebrisAppxInstalled {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$Name)
    return [bool](Get-AppxPackage -Name $Name -ErrorAction SilentlyContinue)
}

function Remove-DebrisAppx {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$Name)

    Invoke-DebrisStep "uninstall $Name" -Action {
        $packages = Get-AppxPackage -Name $Name -ErrorAction SilentlyContinue
        if (-not $packages) {
            Write-DebrisLine '    not installed, skipping'
            return
        }
        foreach ($package in $packages) {
            Remove-AppxPackage -Package $package.PackageFullName -ErrorAction Continue
        }
    }
}

function Disable-DebrisService {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$Name)

    Invoke-DebrisStep "disable service $Name" -Action {
        $service = Get-Service -Name $Name -ErrorAction SilentlyContinue
        if (-not $service) {
            Write-DebrisLine '    no such service, skipping'
            return
        }
        Set-Service -Name $Name -StartupType Disabled
        Stop-Service -Name $Name -Force -ErrorAction SilentlyContinue
    }
}

function Set-DebrisRegistryValue {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Name,
        $Value
    )

    Invoke-DebrisStep "set $Name under $Path" -Action {
        if (-not (Test-Path $Path)) { New-Item -Path $Path -Force | Out-Null }
        Set-ItemProperty -Path $Path -Name $Name -Value $Value
    }
}

function Complete-DebrisRun {
    [CmdletBinding()]
    param()

    Write-DebrisLine ''
    if (Test-DebrisDryRun) {
        Write-DebrisLine "Done. $script:StepCount step(s) listed. Nothing was changed."
        Write-DebrisLine 'Run again with -DryRun:$false to apply them.'
    }
    else {
        Write-DebrisLine "Done. $script:StepCount step(s) applied."
    }
}

Export-ModuleMember -Function @(
    'Use-DebrisMode'
    'Test-DebrisDryRun'
    'Write-DebrisLine'
    'Write-DebrisStep'
    'Invoke-DebrisStep'
    'Assert-DebrisAdmin'
    'Assert-DebrisProcessElevated'
    'Test-DebrisAppxInstalled'
    'Remove-DebrisAppx'
    'Disable-DebrisService'
    'Set-DebrisRegistryValue'
    'Complete-DebrisRun'
)
