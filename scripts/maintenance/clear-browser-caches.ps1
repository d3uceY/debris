### title: Clear browser caches
### category: maintenance
### risk: reversible
### requires-admin: no
### dry-run: yes
###
### Empties the on-disk caches of Edge, Chrome, Brave, Opera, and Firefox for every
### profile that exists on this machine, not just the default one. Each browser rebuilds
### what it needs on the next page load.
###
### Nothing else is touched: passwords, cookies, history, bookmarks, extensions, and
### open sessions all stay. A file a browser currently has open cannot be deleted, so
### close the browsers first if you want the whole cache gone; the script reports how
### many items it had to leave behind.

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun

$browsers = @(
    @{ Label = 'Edge'; Root = (Join-Path -Path $env:LOCALAPPDATA -ChildPath 'Microsoft\Edge\User Data') }
    @{ Label = 'Chrome'; Root = (Join-Path -Path $env:LOCALAPPDATA -ChildPath 'Google\Chrome\User Data') }
    @{ Label = 'Brave'; Root = (Join-Path -Path $env:LOCALAPPDATA -ChildPath 'BraveSoftware\Brave-Browser\User Data') }
    @{ Label = 'Opera'; Root = (Join-Path -Path $env:APPDATA -ChildPath 'Opera Software\Opera Stable') }
    @{ Label = 'Opera GX'; Root = (Join-Path -Path $env:APPDATA -ChildPath 'Opera Software\Opera GX Stable') }
)
$chromiumCaches = @('Cache', 'Code Cache', 'GPUCache')

$firefoxRoots = @(
    (Join-Path -Path $env:APPDATA -ChildPath 'Mozilla\Firefox\Profiles')
    (Join-Path -Path $env:LOCALAPPDATA -ChildPath 'Mozilla\Firefox\Profiles')
)
$firefoxCaches = @('cache2', 'startupCache')

# Collect first, delete second: the preview is worth more when it names every profile
# it found before anything is removed.
$targets = New-Object System.Collections.Generic.List[object]

foreach ($browser in $browsers) {
    if (-not (Test-Path -LiteralPath $browser.Root)) { continue }

    $profiles = @(Get-ChildItem -LiteralPath $browser.Root -Directory -Force -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -eq 'Default' -or $_.Name -like 'Profile *' })

    foreach ($profile in $profiles) {
        $paths = New-Object System.Collections.Generic.List[string]
        foreach ($cache in $chromiumCaches) {
            $candidate = Join-Path -Path $profile.FullName -ChildPath $cache
            if (Test-Path -LiteralPath $candidate) { $paths.Add($candidate) }
        }
        if ($paths.Count -gt 0) {
            $targets.Add(@{ Label = ('{0}, profile {1}' -f $browser.Label, $profile.Name); Paths = $paths })
        }
    }
}

foreach ($root in $firefoxRoots) {
    if (-not (Test-Path -LiteralPath $root)) { continue }

    foreach ($profile in @(Get-ChildItem -LiteralPath $root -Directory -Force -ErrorAction SilentlyContinue)) {
        $paths = New-Object System.Collections.Generic.List[string]
        foreach ($cache in $firefoxCaches) {
            $candidate = Join-Path -Path $profile.FullName -ChildPath $cache
            if (Test-Path -LiteralPath $candidate) { $paths.Add($candidate) }
        }
        if ($paths.Count -gt 0) {
            $targets.Add(@{ Label = ('Firefox, profile {0}' -f $profile.Name); Paths = $paths })
        }
    }
}

$running = @(Get-Process -Name msedge, chrome, brave, firefox, opera, opera_gx -ErrorAction SilentlyContinue |
    Select-Object -ExpandProperty Name | Sort-Object -Unique)
if ($running.Count -gt 0) {
    Write-DebrisLine "  running right now: $($running -join ', ')"
    Write-DebrisLine '  their open cache files will be skipped'
    Write-DebrisLine ''
}

if ($targets.Count -eq 0) {
    Write-DebrisLine '  no browser cache folder found on this machine, nothing to do'
}

foreach ($target in $targets) {
    $items = New-Object System.Collections.Generic.List[string]
    foreach ($path in $target.Paths) {
        foreach ($item in @(Get-ChildItem -LiteralPath $path -Force -ErrorAction SilentlyContinue)) {
            $items.Add($item.FullName)
        }
    }

    if ($items.Count -eq 0) { continue }

    Invoke-DebrisStep ('delete {0} cached item(s) from {1}' -f $items.Count, $target.Label) -Action {
        foreach ($item in $items) {
            Remove-Item -LiteralPath $item -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

$left = 0
foreach ($target in $targets) {
    foreach ($path in $target.Paths) {
        $left += @(Get-ChildItem -LiteralPath $path -Force -ErrorAction SilentlyContinue).Count
    }
}

Write-DebrisLine ''
if (Test-DebrisDryRun) {
    Write-DebrisLine "$($targets.Count) profile cache(s) would be cleared. Cookies, logins, history, and bookmarks stay."
}
else {
    Write-DebrisLine "$($targets.Count) profile cache(s) cleared, $left item(s) left behind, which is what a running browser holds open."
    Write-DebrisLine 'The first visit to a site afterwards is slower while it is cached again.'
}

Complete-DebrisRun
