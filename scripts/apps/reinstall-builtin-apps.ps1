### title: Reinstall all built-in apps
### category: apps
### risk: reversible
### requires-admin: yes
### dry-run: yes
###
### Re-registers every Appx package for the current user, which restores built-in
### apps that were removed or that stopped launching. This is the undo for an
### over-eager cleanup, so keep it available.

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun
Assert-DebrisAdmin -Reason 're-register packages installed for all users'

$packages = @(Get-AppxPackage -AllUsers -ErrorAction SilentlyContinue)
Write-DebrisLine "  found $($packages.Count) package(s)"
Write-DebrisLine ''

foreach ($package in $packages) {
    $manifest = Join-Path -Path $package.InstallLocation -ChildPath 'AppXManifest.xml'
    if ([string]::IsNullOrWhiteSpace($package.InstallLocation) -or -not (Test-Path -Path $manifest)) {
        continue
    }
    Invoke-DebrisStep "re-register $($package.Name)" -Action {
        Add-AppxPackage -DisableDevelopmentMode -Register $manifest -ErrorAction SilentlyContinue
    }
}

Complete-DebrisRun
