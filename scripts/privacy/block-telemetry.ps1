### title: Block telemetry endpoints
### category: privacy
### risk: reversible
### requires-admin: yes
### dry-run: yes
###
### Adds the listed Microsoft telemetry hosts to the hosts file behind a marked
### block, and adds outbound firewall rules that block them. Everything is written
### between debris markers, so it can be removed cleanly and nothing outside those
### markers is touched. Backs up the previous hosts file first.

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun
Assert-DebrisAdmin -Reason 'write the hosts file and add firewall rules'

$marker = 'debris-telemetry-block'
$hostsPath = Join-Path -Path $env:SystemRoot -ChildPath 'System32\drivers\etc\hosts'

$hosts = @(
    'vortex.data.microsoft.com'
    'vortex-win.data.microsoft.com'
    'telecommand.telemetry.microsoft.com'
    'settings-win.data.microsoft.com'
    'watson.telemetry.microsoft.com'
    'sqm.telemetry.microsoft.com'
    'oca.telemetry.microsoft.com'
    'reports.wes.df.telemetry.microsoft.com'
    'geo.settings-win.data.microsoft.com.akadns.net'
    'df.telemetry.microsoft.com'
)

Invoke-DebrisStep "back up the hosts file to hosts.debris.bak" -Action {
    Copy-Item -Path $hostsPath -Destination "$hostsPath.debris.bak" -Force
}

Invoke-DebrisStep "write $($hosts.Count) host entries between markers" -Action {
    $lines = Get-Content -Path $hostsPath -ErrorAction SilentlyContinue
    # Strip any previous debris block so re-running does not stack duplicates.
    $kept = New-Object System.Collections.Generic.List[string]
    $inside = $false
    foreach ($line in $lines) {
        if ($line -like "*# $marker start*") { $inside = $true; continue }
        if ($line -like "*# $marker end*") { $inside = $false; continue }
        if (-not $inside) { $kept.Add($line) }
    }

    $kept.Add('')
    $kept.Add("# $marker start")
    foreach ($hostName in $hosts) { $kept.Add("0.0.0.0 $hostName") }
    $kept.Add("# $marker end")

    Set-Content -Path $hostsPath -Value $kept -Encoding ASCII
}

Invoke-DebrisStep 'add the outbound firewall rules' -Action {
    Get-NetFirewallRule -DisplayName "$marker*" -ErrorAction SilentlyContinue |
        Remove-NetFirewallRule -ErrorAction SilentlyContinue
    foreach ($hostName in $hosts) {
        New-NetFirewallRule -DisplayName "$marker $hostName" `
            -Direction Outbound -Action Block -RemoteAddress 'any' `
            -Description 'Added by debris. Remove with the matching Remove-NetFirewallRule.' | Out-Null
    }
}

Write-DebrisLine ''
Write-DebrisLine 'To undo: delete the marked block in the hosts file (a backup is alongside it)'
Write-DebrisLine "and run Get-NetFirewallRule -DisplayName '$marker*' | Remove-NetFirewallRule"

Complete-DebrisRun
