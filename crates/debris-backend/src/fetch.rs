//! Fetching a catalogue over HTTPS.
//!
//! Through PowerShell rather than an HTTP client crate. `Invoke-WebRequest` is
//! already on the machine, already has the proxy and certificate configuration the
//! user's network needs, and arrives through the same [`CommandRunner`] seam as
//! everything else — so a test can assert the exact URL that would be requested
//! without a socket ever opening.
//!
//! There is no polling and no background refresh. A URL is fetched when the user
//! asks for it, and the copy on disk is what the app reads afterwards.

use crate::runner::{CommandRunner, Invocation};
use debris_core::{DebrisError, Result, is_fetchable};
use std::path::Path;

pub fn fetch_invocation(url: &str) -> Invocation {
    // A GitHub page URL is not a file URL — this is the one place every fetch passes
    // through, so the rewrite belongs here rather than at each caller.
    let escaped = debris_core::raw_url(url).replace('\'', "''");
    Invocation::powershell(&format!(
        "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12;\
$r=Invoke-WebRequest -UseBasicParsing -TimeoutSec 30 -Uri '{escaped}';\
if($r.Content -is [byte[]]){{[Text.Encoding]::UTF8.GetString($r.Content)}}else{{$r.Content}}"
    ))
}

/// Fetches a URL, as text.
pub fn fetch(runner: &dyn CommandRunner, url: &str) -> Result<String> {
    if !is_fetchable(url) {
        return Err(DebrisError::Url {
            detail: format!("`{url}` is not an http(s) address"),
        });
    }
    let output = runner.run(&fetch_invocation(url))?;
    if !output.succeeded() {
        return Err(DebrisError::Failed {
            program: "Invoke-WebRequest".to_string(),
            detail: first_line(&output.combined()),
        });
    }
    let body = output.stdout.trim();
    if body.is_empty() {
        return Err(DebrisError::Failed {
            program: "Invoke-WebRequest".to_string(),
            detail: "the server sent nothing back".to_string(),
        });
    }
    Ok(body.to_string())
}

fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        // PowerShell prefixes its errors with the calling line; the message is the
        // part worth showing.
        .nth(1)
        .map(str::to_string)
        .unwrap_or_else(|| text.trim().to_string())
}

/// The SHA-256 of a file, as `Get-FileHash` reports it.
///
/// Through PowerShell for the same reason the fetch is: hashing correctly is not a
/// thing to write by hand, and the machine already has an implementation.
pub fn sha256_invocation(path: &Path) -> Invocation {
    let escaped = path.display().to_string().replace('\'', "''");
    Invocation::powershell(&format!(
        "(Get-FileHash -Algorithm SHA256 -LiteralPath '{escaped}').Hash"
    ))
}

/// The checksum of a file, uppercase hex, or an error naming what went wrong.
pub fn sha256(runner: &dyn CommandRunner, path: &Path) -> Result<String> {
    let output = runner.run(&sha256_invocation(path))?;
    if !output.succeeded() {
        return Err(DebrisError::Failed {
            program: "Get-FileHash".to_string(),
            detail: first_line(&output.combined()),
        });
    }
    Ok(output.stdout.trim().to_ascii_uppercase())
}

/// Whether a downloaded file matches the checksum its catalogue published.
///
/// Separators and case are ignored: catalogues in the wild write their digests with
/// colons, spaces, or lowercase, and none of those mean a different file.
pub fn checksum_matches(actual: &str, expected: &str) -> bool {
    let clean = |text: &str| -> String {
        text.chars()
            .filter(char::is_ascii_hexdigit)
            .collect::<String>()
            .to_ascii_uppercase()
    };
    let expected = clean(expected);
    !expected.is_empty() && clean(actual) == expected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{DryRunRunner, RunOutput, ScriptedRunner};

    #[test]
    fn a_non_http_url_is_refused_before_powershell_is_started() {
        let runner = DryRunRunner::new();
        let error = fetch(&runner, "file:///C:/secrets.json").unwrap_err();
        assert!(error.to_string().contains("not an http(s) address"));
        assert!(runner.log().is_empty(), "nothing should have run");
    }

    #[test]
    fn the_url_is_the_only_thing_that_varies_and_is_quoted() {
        let invocation = fetch_invocation("https://example.com/catalog.json");
        let script = invocation.args.last().unwrap();
        assert!(script.contains("-Uri 'https://example.com/catalog.json'"));
        assert!(script.contains("Tls12"), "the old default protocol may be refused");
        assert!(script.contains("-TimeoutSec 30"));
    }

    #[test]
    fn a_quote_in_the_url_cannot_break_out_of_the_command() {
        let invocation = fetch_invocation("https://example.com/a'; Remove-Item C:\\ -Recurse; '");
        let script = invocation.args.last().unwrap();
        assert!(script.contains("''"));
        assert!(!script.contains("a'; Remove-Item"));
    }

    #[test]
    fn a_github_page_url_is_requested_as_the_address_that_serves_the_file() {
        let invocation = fetch_invocation(
            "https://github.com/d3uceY/debris/blob/main/examples/extensions/catalog.json",
        );
        let script = invocation.args.last().unwrap();
        assert!(script.contains(
            "-Uri 'https://raw.githubusercontent.com/d3uceY/debris/main/examples/extensions/catalog.json'"
        ));
        // The page URL answers a non-browser client with a 503, so it must not be the
        // one on the wire.
        assert!(!script.contains("github.com/d3uceY/debris/blob"));
    }

    #[test]
    fn the_body_is_returned_as_written() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "[{\"name\":\"x\"}]\n")]);
        let body = fetch(&runner, "https://example.com/a.json").unwrap();
        assert_eq!(body, "[{\"name\":\"x\"}]");
    }

    #[test]
    fn a_failed_request_becomes_an_error_with_the_message() {
        let runner = ScriptedRunner::new(vec![RunOutput {
            status: 1,
            stdout: String::new(),
            stderr: "Remote server returned an error: (404) Not Found.".to_string(),
        }]);
        let error = fetch(&runner, "https://example.com/missing.json").unwrap_err();
        assert!(error.to_string().contains("404"));
    }

    #[test]
    fn an_empty_response_is_a_failure_rather_than_an_empty_catalogue() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "   \n")]);
        let error = fetch(&runner, "https://example.com/empty.json").unwrap_err();
        assert!(error.to_string().contains("nothing back"));
    }

    #[test]
    fn a_checksum_reads_back_uppercase_and_trimmed() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "  abc123  \r\n")]);
        let hash = sha256(&runner, Path::new("C:/tmp/x.ps1")).unwrap();
        assert_eq!(hash, "ABC123");
        let script = runner.log()[0].args.last().unwrap().clone();
        assert!(script.contains("Get-FileHash -Algorithm SHA256"));
        assert!(script.contains("C:/tmp/x.ps1"));
    }

    #[test]
    fn a_missing_file_makes_the_checksum_fail_rather_than_return_empty() {
        let runner = ScriptedRunner::new(vec![RunOutput {
            status: 1,
            stdout: String::new(),
            stderr: "Cannot find path 'C:\\nope.ps1' because it does not exist.".to_string(),
        }]);
        assert!(sha256(&runner, Path::new("C:/nope.ps1")).is_err());
    }

    #[test]
    fn a_checksum_comparison_ignores_separators_and_case() {
        assert!(checksum_matches("ab12cd", "AB:12:CD"));
        assert!(checksum_matches("AB12CD", "ab12cd"));
        assert!(!checksum_matches("ab12cd", "ab12ce"));
        // An empty expectation is not a match; it means nobody published one.
        assert!(!checksum_matches("ab12cd", ""));
    }
}
