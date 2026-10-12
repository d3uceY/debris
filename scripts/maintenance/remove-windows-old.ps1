### title: Remove the previous Windows installation
### category: maintenance
### risk: destructive
### requires-admin: yes
### dry-run: yes
###
### Deletes C:\Windows.old, the copy of the previous Windows installation left behind by
### an upgrade or a feature update. It is usually tens of gigabytes.
###
### This is not undoable. Windows.old is where "Go back to the previous version of
### Windows" gets its files, so afterwards the rollback in Settings is gone for good,
### along with anything that only existed in the old user profile. Windows deletes this
### folder by itself about ten days after the upgrade, and Settings > System > Storage
### has an entry that does the same job without any of the ownership work below, so
### reach for this only when those have not finished.
###
### The folder is owned by TrustedInstaller, so this takes ownership and grants
### administrators full control before deleting.

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun
Assert-DebrisAdmin -Reason 'take ownership of C:\Windows.old and delete it'

$target = Join-Path -Path $env:SystemDrive -ChildPath 'Windows.old'

if (-not (Test-Path -LiteralPath $target)) {
    Write-DebrisLine "  $target does not exist on this machine, so there is no previous installation to remove"
    Complete-DebrisRun
    return
}

$bytes = 0
foreach ($item in @(Get-ChildItem -LiteralPath $target -Recurse -Force -ErrorAction SilentlyContinue)) {
    if (-not $item.PSIsContainer) { $bytes += $item.Length }
}
$size = if ($bytes -gt 0) { '{0:N1} GB' -f ($bytes / 1GB) } else { 'size unknown' }

Write-DebrisLine "  $target is present and holds $size."
Write-DebrisLine '  Deleting it removes the rollback to the previous Windows build. There is no undo.'
Write-DebrisLine ''

Invoke-DebrisStep "take ownership of $target" -Action {
    & takeown.exe /F $target /R /D Y | Out-Null
}

Invoke-DebrisStep 'grant administrators full control of the folder tree' -Action {
    # The SID rather than the name: "Administrators" is spelled differently in other
    # display languages, and the SID is the same everywhere.
    & icacls.exe $target /Grant '*S-1-5-32-544:F' /T /C | Out-Null
}

Invoke-DebrisStep "delete $target ($size)" -Action {
    Remove-Item -LiteralPath $target -Recurse -Force -ErrorAction SilentlyContinue
}

Write-DebrisLine ''
if (Test-DebrisDryRun) {
    Write-DebrisLine "$target would be gone, and the previous Windows build would no longer be rollback-able."
}
elseif (Test-Path -LiteralPath $target) {
    Write-DebrisLine "$target is still there. Some files resisted removal, which some antivirus products cause."
    Write-DebrisLine 'Settings > System > Storage > Temporary files removes what is left, without needing this.'
}
else {
    Write-DebrisLine "$target is gone."
}

Complete-DebrisRun
