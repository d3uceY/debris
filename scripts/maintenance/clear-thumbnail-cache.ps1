### title: Clear the thumbnail and icon cache
### category: maintenance
### risk: reversible
### requires-admin: no
### dry-run: yes
###
### Deletes Explorer's thumbnail and icon databases so Windows rebuilds them. Windows
### has no size limit worth worrying about on these, so a machine that has browsed a
### lot of images can hold several hundred megabytes here.
###
### The rebuild is the cost: the next time you open a folder of pictures, Explorer
### reads the originals again to draw the thumbnails, and folders open more slowly
### until it has. Explorer also keeps some of these databases open at all times, and
### what is open cannot be deleted; the script reports what it had to leave behind.

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun

$cacheFolder = Join-Path -Path $env:LOCALAPPDATA -ChildPath 'Microsoft\Windows\Explorer'

if (-not (Test-Path -LiteralPath $cacheFolder)) {
    Write-DebrisLine '  this machine has no Explorer cache folder, so there is nothing to clear'
    Complete-DebrisRun
    return
}

$caches = @(
    Get-ChildItem -LiteralPath $cacheFolder -Filter 'thumbcache_*.db' -Force -ErrorAction SilentlyContinue
    Get-ChildItem -LiteralPath $cacheFolder -Filter 'iconcache_*.db' -Force -ErrorAction SilentlyContinue
)

if ($caches.Count -eq 0) {
    Write-DebrisLine '  no thumbnail or icon database found, so there is nothing to clear'
    Complete-DebrisRun
    return
}

$bytes = 0
foreach ($cache in $caches) { $bytes += $cache.Length }

Invoke-DebrisStep ('delete {0} database(s) holding {1:N1} MB' -f $caches.Count, ($bytes / 1MB)) -Action {
    foreach ($cache in $caches) {
        Remove-Item -LiteralPath $cache.FullName -Force -ErrorAction SilentlyContinue
    }
}

$left = @(
    Get-ChildItem -LiteralPath $cacheFolder -Filter 'thumbcache_*.db' -Force -ErrorAction SilentlyContinue
    Get-ChildItem -LiteralPath $cacheFolder -Filter 'iconcache_*.db' -Force -ErrorAction SilentlyContinue
)

Write-DebrisLine ''
if (Test-DebrisDryRun) {
    Write-DebrisLine 'Explorer rebuilds these on its own; nothing else in the folder is touched.'
}
else {
    Write-DebrisLine "$($caches.Count - $left.Count) deleted, $($left.Count) still open in Explorer."
    if ($left.Count -gt 0) {
        Write-DebrisLine 'Restart Explorer or sign out and back in to release those, then run this again for the rest.'
    }
}

Complete-DebrisRun
