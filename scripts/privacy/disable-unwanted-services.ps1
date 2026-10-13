### title: Disable unwanted services
### category: privacy
### risk: reversible
### requires-admin: yes
### dry-run: yes
###
### Sets the listed background services to Disabled and stops them. Every one of
### these can be returned to its previous state with Set-Service -StartupType, which
### is why this is reversible and not destructive. Deliberately excluded:
### WlanSvc, WSearch and the print spooler, which people notice missing.

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun
Assert-DebrisAdmin -Reason 'change service startup types'

$services = @(
    'diagnosticshub.standardcollector.service'  # Diagnostics Hub Standard Collector
    'DiagTrack'                                 # Connected User Experiences and Telemetry
    'dmwappushservice'                          # WAP Push Message Routing
    'lfsvc'                                     # Geolocation
    'MapsBroker'                                # Downloaded Maps Manager
    'NetTcpPortSharing'                         # Net.Tcp Port Sharing
    'RemoteAccess'                              # Routing and Remote Access
    'RemoteRegistry'                            # Remote Registry
    'SharedAccess'                              # Internet Connection Sharing
    'TrkWks'                                    # Distributed Link Tracking Client
    'WbioSrvc'                                  # Windows Biometric (needed for fingerprint / face)
    'WMPNetworkSvc'                             # WMP Network Sharing
    'XblAuthManager'                            # Xbox Live Auth Manager
    'XblGameSave'                               # Xbox Live Game Save
    'XboxNetApiSvc'                             # Xbox Live Networking
    'ndu'                                       # Network Data Usage Monitor
)

foreach ($service in $services) {
    Disable-DebrisService -Name $service
}

Write-DebrisLine ''
Write-DebrisLine 'Re-enable any of these with: Set-Service -Name <name> -StartupType Manual'

Complete-DebrisRun
