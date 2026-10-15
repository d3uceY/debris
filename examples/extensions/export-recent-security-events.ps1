#Requires -RunAsAdministrator
# Writes the newest entries in the Security log to a text report.
#
# The Security log refuses to open without an administrator token, which is why its
# catalogue entry sets `requiresAdmin` and why this file opens with `#Requires`. The
# directive is the belt: the app warns before the run, and PowerShell refuses after it.

$ErrorActionPreference = 'Stop'

$stamp = Get-Date -Format 'yyyy-MM-dd-HHmmss'
$report = Join-Path $PSScriptRoot "security-events-$stamp.txt"
$lines = New-Object System.Collections.Generic.List[string]

$lines.Add('Security log — newest 50 entries')
$lines.Add("Written $(Get-Date -Format 'u')")

foreach ($event in Get-WinEvent -LogName Security -MaxEvents 50) {
    $lines.Add('')
    $lines.Add('{0:u}  id {1,-6} {2}' -f $event.TimeCreated, $event.Id, $event.ProviderName)
    $message = ($event.Message -replace '\s+', ' ').Trim()
    if ($message.Length -gt 240) { $message = $message.Substring(0, 240) + '...' }
    $lines.Add("  $message")
}

[IO.File]::WriteAllLines($report, $lines)
"Wrote $report"
