### title: Clear the Windows Update cache
### category: maintenance
### risk: reversible
### requires-admin: yes
### dry-run: yes
###
### Stops the Windows Update and BITS services, empties the update download cache and
### the update datastore, then restarts whichever of the two was already running. This
### is the fix for an update that has wedged itself, and it frees whatever the download
### cache had been holding.
###
### Reversible in the sense that nothing breaks: Windows rebuilds both folders on the
### next update check. The visible cost is that the installed-update history in
### Settings disappears, which is why it is not worth running casually.

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun
Assert-DebrisAdmin -Reason 'stop the Windows Update services and empty their folders'

$services = @('wuauserv', 'bits')

$wereRunning = New-Object System.Collections.Generic.List[string]
foreach ($name in $services) {
    $service = Get-Service -Name $name -ErrorAction SilentlyContinue
    if ($service -and $service.Status -eq 'Running') { $wereRunning.Add($name) }
}

if ($wereRunning.Count -eq 0) {
    Write-DebrisLine '  neither service was running, so neither will be stopped or restarted'
    Write-DebrisLine ''
}

$folders = @(
    @{ Label = 'update download cache'; Path = (Join-Path -Path $env:SystemRoot -ChildPath 'SoftwareDistribution\Download') }
    @{ Label = 'update datastore'; Path = (Join-Path -Path $env:SystemRoot -ChildPath 'SoftwareDistribution\DataStore') }
)

# The services are restarted in finally so that a failure while deleting cannot leave
# Windows Update switched off.
try {
    if ($wereRunning.Count -gt 0) {
        Invoke-DebrisStep "stop $($wereRunning -join ' and ')" -Action {
            foreach ($name in $wereRunning) {
                Stop-Service -Name $name -Force -ErrorAction SilentlyContinue
            }
        }
    }

    foreach ($folder in $folders) {
        if (-not (Test-Path -LiteralPath $folder.Path)) {
            Write-DebrisLine "  $($folder.Label): not present on this machine, skipping"
            continue
        }

        $items = @(Get-ChildItem -LiteralPath $folder.Path -Force -ErrorAction SilentlyContinue)
        if ($items.Count -eq 0) {
            Write-DebrisLine "  $($folder.Label): already empty"
            continue
        }

        Invoke-DebrisStep "delete $($items.Count) item(s) from the $($folder.Label)" -Action {
            foreach ($item in $items) {
                Remove-Item -LiteralPath $item.FullName -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }
}
finally {
    if ($wereRunning.Count -gt 0) {
        Invoke-DebrisStep "restart $($wereRunning -join ' and ')" -Action {
            foreach ($name in $wereRunning) {
                Start-Service -Name $name -ErrorAction SilentlyContinue
            }
        }
    }
}

$left = 0
foreach ($folder in $folders) {
    if (Test-Path -LiteralPath $folder.Path) {
        $left += @(Get-ChildItem -LiteralPath $folder.Path -Force -ErrorAction SilentlyContinue).Count
    }
}

Write-DebrisLine ''
if (Test-DebrisDryRun) {
    Write-DebrisLine 'Nothing was stopped and nothing was deleted.'
}
else {
    if ($left -gt 0) {
        Write-DebrisLine "$left item(s) could not be removed, which normally means Windows Update restarted itself"
        Write-DebrisLine 'and opened them again. Running this a second time catches the rest.'
    }
    Write-DebrisLine 'The next update check downloads what it needs again.'
    Write-DebrisLine 'Update history in Settings is now empty; installed updates are unaffected.'
}

Complete-DebrisRun
