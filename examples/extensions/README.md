# Writing an extension

An extension is a PowerShell script this app does not ship. You publish it wherever you
like, list it in a JSON file, and anyone who adds that file's URL to the **Extensions**
surface can read the list, install a script from it, and run it after confirming.

Nothing is audited on the way in. The catalogue says what each script is, the app shows
that text, and the run is confirmed by hand. Treat a catalogue you did not write as
untrusted.

This folder is a working example, not a mock-up:

```
catalog.json                          two entries, pointing at the two scripts below
export-network-config.ps1             read-only, no admin
export-recent-security-events.ps1     read-only, needs admin
```

Serve this folder and the app will list both, or copy any of the three files as the
starting point for your own.

## The catalogue

An array of entries. That is the whole format — no wrapper object.

| Field | Required | Meaning |
|---|---|---|
| `name` | yes | Shown in the list. An entry with a blank name is dropped. |
| `url` | yes | Where the script lives. Must end in `.ps1` or the entry is dropped. |
| `description` | no | One line, shown under the name. |
| `version` | no | Shown as a `v1.2` chip. Also what the app records as installed. |
| `requiresAdmin` | no | Also accepts `requires_admin`. Defaults to false. |
| `sha256` | no | See [Checksums](#checksums). |

```json
[
  {
    "name": "Export network configuration",
    "description": "Writes your addresses, gateways, DNS servers and proxy settings to a text report.",
    "url": "https://raw.githubusercontent.com/d3uceY/debris/main/examples/extensions/export-network-config.ps1",
    "version": "1.0"
  },
  {
    "name": "Export recent security events",
    "description": "Writes the newest Security log entries to a text report. Needs an administrator token.",
    "url": "https://raw.githubusercontent.com/d3uceY/debris/main/examples/extensions/export-recent-security-events.ps1",
    "version": "1.0",
    "requiresAdmin": true
  }
]
```

A bad entry is dropped, not a reason to discard the file: one author's typo should not
hide everyone else's scripts. Entries are sorted by name. URLs must be `http` or `https`
— `file:` is refused, because "add a catalogue" is not "read whatever path I name".

**The installed file name comes from the last path segment of the URL**, percent-decoded.
Nothing else names the file, so choose it deliberately: a URL ending in
`Export%20Network%20Config.ps1` installs as `Export Network Config.ps1`. A tail that
would escape the folder — anything with a slash, a colon, or a leading dot — is dropped.

## The script

It is run as a plain file:

```
powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "…\extensions\<name>.ps1"
```

Four things follow from that, and they are the ones that catch people:

- **It gets no arguments.** There is no way to pass one, so a script that needs a
  choice has to read it from a file or pick a sane default.
- **`-NonInteractive` is on.** A script that prompts for input does not wait; it fails.
- **It runs from `%APPDATA%\debris\extensions\`,** so `$PSScriptRoot` is that folder and
  the script library's `lib\Debris.Common.psm1` is not there. An extension is one file
  and nothing else.
- **Standard output is not shown when it succeeds.** The status bar reports
  `Name finished.` and discards the rest. A failure reports the error instead. If your
  script produces something the user wants, write it to a file — that is what both
  examples here do.

Unlike the scripts in `scripts/`, an extension does **not** need the `###` header. That
header exists so the library can refuse a destructive script that cannot be previewed;
an extension is never previewed, so there is nothing to declare. Put the same
information in the catalogue's `description` and `requiresAdmin` instead, where the user
reads it before deciding.

If your script genuinely needs an administrator token, say so twice: `requiresAdmin` in
the catalogue, so the app shows the requirement before the run, and
`#Requires -RunAsAdministrator` in the script, so PowerShell refuses if it gets there
anyway. The second is what `export-recent-security-events.ps1` does — run it from a
normal session and it stops on that line, having done nothing.

## Checksums

Optional, and worth publishing for a script people download over the network. When the
entry has one, a download that does not match is deleted before anything records it as
installed. Case and separators are ignored when comparing, so a digest written with
colons or in lowercase matches.

**Hash the bytes the URL serves, not your working copy.** On Windows git rewrites line
endings on commit, so a hash taken from a checked-out file is the hash of the CRLF copy
while the server hands out the LF one — and the install would fail with a mismatch that
looks like tampering.

The easy way to get the right bytes is to let the app fetch them. Publish the entry
without a checksum, install it, and hash the copy the app just wrote, which is the same
file it will compare against next time:

```powershell
(Get-FileHash -Algorithm SHA256 -LiteralPath `
    "$env:APPDATA\debris\extensions\your-script.ps1").Hash
```

Add that to the entry, publish it, then **Remove** the installed copy and **Install** it
again so the download is checked. There is no update-in-place: an installed script shows
*Run*, not *Install*, precisely so a catalogue edit cannot swap the file underneath you.

## Trying it before you publish

The app fetches over HTTP, so serve the folder and point it at your own machine:

```sh
python -m http.server 8000
```

Then the catalogue URL is `http://localhost:8000/catalog.json`. Remember that a
catalogue is fetched only when you add it or press **Refresh all** — there is no polling,
and the cached copy on disk is what the app reads in between.
