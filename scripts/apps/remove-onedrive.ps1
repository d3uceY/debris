### title: Remove OneDrive
### category: apps
### risk: reversible
### requires-admin: yes
### dry-run: yes
###
### Removes the OneDrive client, clears its scheduled task, and disables its sync
### policy. Explorer is deliberately left running: the original script killed it,
### which closes the user's windows for no benefit. Cloud files are not touched;
### local sync stops.
### credit: adapted from Debloat-Windows-10 (MIT)

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun
Assert-DebrisAdmin -Reason 'remove the OneDrive client and write its policy key'

$setup = @(
    "$env:SystemRoot\System32\OneDriveSetup.exe"
    "$env:SystemRoot\SysWOW64\OneDriveSetup.exe"
) | Where-Object { Test-Path -Path $_ }

if ($setup) {
    foreach ($exe in $setup) {
        Invoke-DebrisStep "run $exe /uninstall" -Action { & $exe /uninstall | Out-Null }
    }
}
else {
    Write-DebrisLine '  OneDriveSetup.exe not found, skipping the uninstaller'
}

Invoke-DebrisStep 'unregister the OneDrive scheduled tasks' -Action {
    Get-ScheduledTask -TaskName 'OneDrive*' -ErrorAction SilentlyContinue |
        Unregister-ScheduledTask -Confirm:$false -ErrorAction SilentlyContinue
}

Invoke-DebrisStep 'disable OneDrive sync by machine policy' -Action {
    $path = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\OneDrive'
    if (-not (Test-Path -Path $path)) { New-Item -Path $path -Force | Out-Null }
    Set-ItemProperty -Path $path -Name 'DisableFileSyncNGSC' -Value 1
}

$leftovers = @(
    "$env:LOCALAPPDATA\Microsoft\OneDrive"
    "$env:ProgramData\Microsoft OneDrive"
    "$env:SystemDrive\OneDriveTemp"
)

foreach ($folder in $leftovers) {
    Invoke-DebrisStep "remove leftover folder $folder" -Action {
        if (Test-Path -Path $folder) {
            Remove-Item -Path $folder -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

Complete-DebrisRun
