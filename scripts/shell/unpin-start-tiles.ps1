### title: Unpin Start menu tiles
### category: shell
### risk: reversible
### requires-admin: no
### dry-run: yes
###
### Clears the current user's Start tile cache so Windows rebuilds the menu with no
### pinned tiles on the next sign-in. No application is uninstalled and no shortcut
### is deleted; tiles can be re-pinned by the user at any time.

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun

$targets = @(
    (Join-Path -Path $env:LOCALAPPDATA -ChildPath 'TileDataLayer')
    (Join-Path -Path $env:LOCALAPPDATA -ChildPath 'Microsoft\Windows\Shell\DefaultLayouts.xml')
)

$present = $targets | Where-Object { Test-Path -Path $_ }
if (-not $present) {
    Write-DebrisLine '  no tile cache to clear: the Start menu is already at its default layout'
}

foreach ($target in $present) {
    Invoke-DebrisStep "clear $target" -Action {
        Remove-Item -Path $target -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Write-DebrisLine ''
Write-DebrisLine 'Takes effect on the next sign-in. Pinned tiles are gone; apps are untouched.'

Complete-DebrisRun
