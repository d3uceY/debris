### title: Clear temporary files
### category: maintenance
### risk: reversible
### requires-admin: yes
### dry-run: yes
###
### Empties the per-user temp folder, the Windows temp folder, and the Prefetch
### folder. The folders themselves are kept; only their contents are deleted, and a
### file that a running program still has open is left where it is and reported.
###
### Prefetch is a boot-speed cache rather than junk: the first start of each
### application afterwards is slower while Windows rebuilds what it needs. That cost
### is the reason this is something you choose to run.

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun
Assert-DebrisAdmin -Reason 'empty the Windows temp and Prefetch folders'

$roots = @(
    @{ Label = 'user temp'; Path = $env:Temp }
    @{ Label = 'Windows temp'; Path = (Join-Path -Path $env:SystemRoot -ChildPath 'Temp') }
    @{ Label = 'Prefetch'; Path = (Join-Path -Path $env:SystemRoot -ChildPath 'Prefetch') }
)

$counted = 0
foreach ($root in $roots) {
    if (-not (Test-Path -LiteralPath $root.Path)) {
        Write-DebrisLine "  $($root.Label): not present on this machine, skipping"
        continue
    }

    $items = @(Get-ChildItem -LiteralPath $root.Path -Force -ErrorAction SilentlyContinue)
    if ($items.Count -eq 0) {
        Write-DebrisLine "  $($root.Label): already empty"
        continue
    }

    $counted += $items.Count
    Invoke-DebrisStep "delete $($items.Count) item(s) from the $($root.Label) folder" -Action {
        foreach ($item in $items) {
            Remove-Item -LiteralPath $item.FullName -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

$left = 0
foreach ($root in $roots) {
    if (Test-Path -LiteralPath $root.Path) {
        $left += @(Get-ChildItem -LiteralPath $root.Path -Force -ErrorAction SilentlyContinue).Count
    }
}

Write-DebrisLine ''
if (Test-DebrisDryRun) {
    Write-DebrisLine "$counted item(s) would be deleted. The folders stay, and Windows recreates anything it needs."
}
else {
    Write-DebrisLine "$counted item(s) deleted, $left left behind, which is normally a file still open in a running program."
}
Write-DebrisLine 'Nothing else changed: no application was touched and no setting was written.'

Complete-DebrisRun
