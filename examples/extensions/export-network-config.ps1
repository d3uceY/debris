# Writes this machine's network configuration to a text report.
#
# Read-only: nothing here touches an adapter, a route or a resolver. That is why it
# needs no administrator token and why its catalogue entry omits `requiresAdmin`.

$ErrorActionPreference = 'Stop'

$stamp = Get-Date -Format 'yyyy-MM-dd-HHmmss'
$report = Join-Path $PSScriptRoot "network-config-$stamp.txt"
$out = New-Object System.Collections.Generic.List[string]

function Add-Section([string]$title, $rows) {
    $out.Add('')
    $out.Add($title)
    $out.Add('-' * $title.Length)
    foreach ($row in $rows) { $out.Add("  $row") }
}

$out.Add("Network configuration for $env:COMPUTERNAME")
$out.Add("Written $(Get-Date -Format 'u')")

Add-Section 'Adapters' (Get-NetIPConfiguration | ForEach-Object {
    '{0} [{1}] {2}  gateway: {3}' -f $_.InterfaceAlias,
        $_.NetAdapter.Status,
        ($_.IPv4Address.IPAddress -join ', '),
        $_.IPv4DefaultGateway.NextHop
})

Add-Section 'DNS servers' (Get-DnsClientServerAddress -AddressFamily IPv4 |
    Where-Object { $_.ServerAddresses } |
    ForEach-Object { '{0}: {1}' -f $_.InterfaceAlias, ($_.ServerAddresses -join ', ') })

Add-Section 'IPv4 routes' (Get-NetRoute -AddressFamily IPv4 |
    Sort-Object RouteMetric |
    Select-Object -First 15 |
    ForEach-Object { '{0,-20} {1,-18} metric {2}' -f $_.DestinationPrefix, $_.NextHop, $_.RouteMetric })

$winhttp = ((netsh winhttp show proxy) -join ' ').Trim()
$wininet = (Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings').ProxyServer
Add-Section 'Proxy' @("WinHTTP: $winhttp", "WinINET: $wininet")

[IO.File]::WriteAllLines($report, $out)
"Wrote $report"
