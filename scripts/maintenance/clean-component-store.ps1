### title: Clean up the component store
### category: maintenance
### risk: reversible
### requires-admin: yes
### dry-run: yes
###
### Runs DISM's StartComponentCleanup, which deletes superseded versions of components
### from WinSxS. A machine that has taken a few years of updates can have several
### gigabytes of them sitting there, and Windows indexes them on every scan.
###
### It deliberately does not pass /ResetBase. That flag frees more space but makes
### every already-installed update impossible to uninstall, so a patch that breaks
### something could no longer be rolled back. Everything installed stays uninstallable.
###
### Expect this to take several minutes and to print nothing while it works: DISM does
### not report progress for it, and the output arrives when it finishes.

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun
Assert-DebrisAdmin -Reason 'run DISM cleanup on the component store'

$dism = Join-Path -Path $env:SystemRoot -ChildPath 'System32\Dism.exe'
if (-not (Test-Path -LiteralPath $dism)) {
    Write-DebrisLine '  DISM is not present on this machine, so there is nothing to run'
    Complete-DebrisRun
    return
}

Write-DebrisLine '  Only the component store is touched. No file you can see is removed.'
Write-DebrisLine ''

Invoke-DebrisStep 'run DISM /Online /Cleanup-Image /StartComponentCleanup' -Action {
    & $dism /Online /Cleanup-Image /StartComponentCleanup /Quiet | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-DebrisLine "    DISM exited with code $LASTEXITCODE. It reports its own reason; nothing else was changed."
    }
}

Write-DebrisLine ''
Write-DebrisLine 'Superseded component versions are gone. Already-installed updates can still be'
Write-DebrisLine 'uninstalled, because /ResetBase was not used.'

Complete-DebrisRun
