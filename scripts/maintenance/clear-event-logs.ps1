### title: Clear the Windows event logs
### category: maintenance
### risk: destructive
### requires-admin: yes
### dry-run: yes
###
### Empties the event logs, which frees however much space the diagnostic ones have
### quietly grown into. On a machine that has been logging warnings for years this is
### often a few hundred megabytes.
###
### This is not reversible. Once a log is cleared the events in it are gone, and
### anybody troubleshooting a problem with that history no longer has it. The Security
### log is deliberately left alone: it is the machine's audit trail, and destroying
### evidence of who did what is a different act from emptying a diagnostic log.
###
### To keep a copy first: wevtutil epl Application C:\somewhere\Application.evtx

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun
Assert-DebrisAdmin -Reason 'clear the event logs'

$kept = @('Security')

# Analytic and Debug logs are ETW tracing channels: they are off by default, and
# Windows refuses to clear them anyway, so including them would only add noise.
$logs = @(Get-WinEvent -ListLog '*' -ErrorAction SilentlyContinue |
    Where-Object { $kept -notcontains $_.LogName -and $_.LogType -notin @('Analytic', 'Debug') })

if ($logs.Count -eq 0) {
    Write-DebrisLine '  no event logs could be read, so there is nothing to clear'
    Complete-DebrisRun
    return
}

$bytes = 0
foreach ($log in $logs) { $bytes += $log.FileSize }

Write-DebrisLine "  $($logs.Count) log(s) will be emptied. Left alone: $($kept -join ', ')."
Write-DebrisLine '  The events in them are not recoverable afterwards.'
Write-DebrisLine ''

Invoke-DebrisStep ('clear {0} event log(s) holding {1:N1} MB' -f $logs.Count, ($bytes / 1MB)) -Action {
    $refused = New-Object System.Collections.Generic.List[string]
    foreach ($log in $logs) {
        & wevtutil.exe cl $log.LogName 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { $refused.Add($log.LogName) }
    }

    if ($refused.Count -gt 0) {
        Write-DebrisLine "    $($refused.Count) log(s) refused to clear, which usually means they were already empty:"
        foreach ($name in @($refused | Select-Object -First 5)) {
            Write-DebrisLine "      $name"
        }
        if ($refused.Count -gt 5) {
            Write-DebrisLine "      and $($refused.Count - 5) more"
        }
    }
}

Write-DebrisLine ''
Write-DebrisLine 'The logs are now empty and keep working: new events are recorded as they happen.'

Complete-DebrisRun
