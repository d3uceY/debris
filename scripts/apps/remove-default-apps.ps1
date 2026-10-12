### title: Remove the default app set
### category: apps
### risk: reversible
### requires-admin: yes
### dry-run: yes
###
### Removes the apps Windows ships preinstalled for a broad audience: news, weather,
### games, the Office hub, the Xbox stack. Anything on the system-critical list is
### left alone; comment out a line to keep that app.
### credit: adapted from Debloat-Windows-10 (MIT)

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun
Assert-DebrisAdmin -Reason 'remove packages installed for all users'

$apps = @(
    # Microsoft's preinstalled set
    'Microsoft.BingNews'
    'Microsoft.BingWeather'
    'Microsoft.BingFinance'
    'Microsoft.BingSports'
    'Microsoft.Getstarted'
    'Microsoft.GetHelp'
    'Microsoft.Microsoft3DViewer'
    'Microsoft.MicrosoftOfficeHub'
    'Microsoft.MicrosoftSolitaireCollection'
    'Microsoft.MixedReality.Portal'
    'Microsoft.MSPaint'
    'Microsoft.Office.OneNote'
    'Microsoft.Office.Sway'
    'Microsoft.OneConnect'
    'Microsoft.People'
    'Microsoft.Print3D'
    'Microsoft.SkypeApp'
    'Microsoft.Wallet'
    'Microsoft.WindowsAlarms'
    'Microsoft.WindowsCamera'
    'Microsoft.WindowsFeedbackHub'
    'Microsoft.WindowsMaps'
    'Microsoft.WindowsSoundRecorder'
    'Microsoft.YourPhone'
    'Microsoft.ZuneMusic'
    'Microsoft.ZuneVideo'
    'Microsoft.MicrosoftStickyNotes'
    'Microsoft.Messaging'
    'Microsoft.CommsPhone'
    'Microsoft.ConnectivityStore'
    'microsoft.windowscommunicationsapps'

    # Xbox stack
    'Microsoft.Xbox.TCUI'
    'Microsoft.XboxApp'
    'Microsoft.XboxGameOverlay'
    'Microsoft.XboxGamingOverlay'
    'Microsoft.XboxSpeechToTextOverlay'

    # Third-party bundles that ship with OEM images
    '4DF9E0F8.Netflix'
    'SpotifyAB.SpotifyMusic'
    'king.com.CandyCrushSaga'
    'king.com.CandyCrushFriends'
    'king.com.FarmHeroesSaga'
    'E046963F.LenovoCompanion'
    'LenovoCorporation.LenovoID'
    'LenovoCorporation.LenovoSettings'
)

# Deliberately absent, and worth knowing why:
#   Microsoft.WindowsStore          - cannot be reinstalled normally
#   Microsoft.Windows.Photos        - many people want it
#   Microsoft.WindowsCalculator     - same
#   Microsoft.WindowsStore          - on the system-critical list, so it stays

foreach ($app in $apps) {
    Remove-DebrisAppx -Name $app
}

Complete-DebrisRun
