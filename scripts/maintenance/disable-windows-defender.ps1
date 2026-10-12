### title: Disable Windows Defender
### category: maintenance
### risk: destructive
### requires-admin: yes
### dry-run: yes
###
### NOT RECOMMENDED. Sets the Defender policy key to disable real-time protection.
### Modern Windows enforces Tamper Protection, so this will usually be reverted by
### the OS on its own - which means the most likely outcome is a machine that is
### briefly unprotected and then protected again, with no lasting benefit. Kept in
### the library because the original tool shipped it, marked destructive because
### that is what it is.

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun
Assert-DebrisAdmin -Reason 'write the Defender policy key'

Write-DebrisLine '  This leaves the machine unprotected against malware while it holds.'
Write-DebrisLine ''

$targets = @(
    @{ Path = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows Defender'; Name = 'DisableAntiSpyware' }
    @{ Path = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows Defender\Real-Time Protection'; Name = 'DisableRealtimeMonitoring' }
)

foreach ($target in $targets) {
    Set-DebrisRegistryValue -Path $target.Path -Name $target.Name -Value 1
}

Write-DebrisLine ''
Write-DebrisLine 'Reverse with: Set-ItemProperty -Path <path> -Name <name> -Value 0'
Write-DebrisLine 'Or just re-enable it in Windows Security, which is the better answer.'

Complete-DebrisRun
