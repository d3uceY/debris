### title: Reset the network stack
### category: maintenance
### risk: destructive
### requires-admin: yes
### dry-run: yes
###
### Flushes the DNS resolver cache, then resets the Winsock catalog and the IPv4
### stack. These are the fixes for a machine that cannot reach the network at all.
### They are the wrong tool for anything short of that, and there is no way to undo
### them from here.
###
### What it costs. The Winsock reset discards every layered service provider
### installed on the machine, which is how some VPN clients, proxy tools, and
### antivirus products hook into the network. The IP reset removes the custom IPv4
### settings from every adapter, including static IP addresses, custom DNS servers,
### and any manual route. Both are written back to the Windows defaults.
###
### A restart is required before the network comes back properly, and a remote session
### will drop when it does. Run this only when someone can physically reach the machine.

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun
Assert-DebrisAdmin -Reason 'reset the Winsock catalog and the IP stack'

$resetLog = Join-Path -Path $env:Temp -ChildPath 'debris-ip-reset.log'

Write-DebrisLine '  This discards VPN and antivirus network hooks and any static IP settings.'
Write-DebrisLine '  A restart is required afterwards.'
Write-DebrisLine ''

Invoke-DebrisStep 'flush the DNS resolver cache' -Action {
    & ipconfig.exe /FlushDns | Out-Null
    if ($LASTEXITCODE -ne 0) { Write-DebrisLine "    ipconfig exited with code $LASTEXITCODE" }
}

Invoke-DebrisStep 'reset the Winsock catalog, discarding installed providers' -Action {
    & netsh.exe winsock reset | Out-Null
    if ($LASTEXITCODE -ne 0) { Write-DebrisLine "    netsh exited with code $LASTEXITCODE" }
}

Invoke-DebrisStep 'reset the IPv4 stack to its defaults, discarding custom addresses and DNS' -Action {
    & netsh.exe int ip reset "$resetLog" | Out-Null
    if ($LASTEXITCODE -ne 0) { Write-DebrisLine "    netsh exited with code $LASTEXITCODE" }
}

Write-DebrisLine ''
if (Test-DebrisDryRun) {
    Write-DebrisLine 'Nothing was reset. A run would leave the network needing a restart.'
}
else {
    Write-DebrisLine "netsh wrote its record of the IPv4 changes to $resetLog"
    Write-DebrisLine 'Restart when you are ready. Until then the network may be slower or absent.'
}

Complete-DebrisRun
