use crate::runner::{CommandRunner, Invocation};
use debris_core::{AppxPackage, DebrisError, Result, SystemCatalog};

/// The inventory query.
///
/// The original tool drove PowerShell's object API and regex-stripped `@{Name=...}`
/// off the results. Asking for CSV instead keeps the boundary to plain text that can
/// be parsed once and verified in a test, with no regex and no .NET marshalling.
pub const LIST_PACKAGES: &str =
    "Get-AppxPackage | Select-Object Name, PackageFullName, Publisher, Version | ConvertTo-Csv -NoTypeInformation";

pub fn discover(runner: &dyn CommandRunner, catalog: &SystemCatalog) -> Result<Vec<AppxPackage>> {
    let output = runner.run(&Invocation::powershell(LIST_PACKAGES))?;
    if !output.succeeded() {
        return Err(DebrisError::Failed {
            program: "Get-AppxPackage".to_string(),
            detail: output.combined(),
        });
    }
    Ok(parse_packages(&output.stdout, catalog))
}

/// Parses the CSV and classifies every row against the system catalog.
///
/// Columns are located by header name rather than by position, so a differently
/// ordered `Select-Object` still works. A missing `Name` column yields an empty
/// inventory instead of a panic — the honest answer to "I could not read this".
pub fn parse_packages(csv: &str, catalog: &SystemCatalog) -> Vec<AppxPackage> {
    let mut lines = csv
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'));

    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let columns = parse_row(header);
    let index_of = |name: &str| columns.iter().position(|column| column.eq_ignore_ascii_case(name));

    let Some(name_at) = index_of("Name") else {
        return Vec::new();
    };
    let full_at = index_of("PackageFullName");
    let publisher_at = index_of("Publisher");
    let version_at = index_of("Version");

    let field = |fields: &[String], at: Option<usize>| -> String {
        at.and_then(|at| fields.get(at))
            .map(|value| value.trim().to_string())
            .unwrap_or_default()
    };

    let mut packages: Vec<AppxPackage> = Vec::new();
    for line in lines {
        let fields = parse_row(line);
        let name = field(&fields, Some(name_at));
        if name.is_empty() {
            continue;
        }
        let full_name = field(&fields, full_at);
        // The same package can be reported more than once; a package full name is
        // its true identity, so it is the right thing to deduplicate on.
        if !full_name.is_empty()
            && packages
                .iter()
                .any(|existing| existing.full_name == full_name)
        {
            continue;
        }
        let mut package = AppxPackage {
            name,
            full_name,
            publisher: field(&fields, publisher_at),
            version: field(&fields, version_at),
            is_system: false,
        };
        catalog.classify(&mut package);
        packages.push(package);
    }

    packages.sort_by(|a, b| a.name.cmp(&b.name).then(a.full_name.cmp(&b.full_name)));
    packages
}

/// An RFC 4180 subset: quoted fields, `""` as an escaped quote, commas inside
/// quotes preserved. `ConvertTo-Csv` emits exactly this and nothing more.
fn parse_row(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(character) = chars.next() {
        match character {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' => in_quotes = true,
            ',' if !in_quotes => fields.push(std::mem::take(&mut current)),
            other => current.push(other),
        }
    }
    fields.push(current);
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATALOG: &str = "Microsoft.WindowsStore\nMicrosoft.LockApp\n";

    fn sample() -> String {
        [
            "\"Name\",\"PackageFullName\",\"Publisher\",\"Version\"",
            "\"Microsoft.BingNews\",\"Microsoft.BingNews_4.12_x64__8wekyb3d8bbwe\",\"CN=Microsoft Corporation, O=Microsoft\",\"4.12.0.0\"",
            "\"Microsoft.WindowsStore\",\"Microsoft.WindowsStore_2024_x64__8wekyb3d8bbwe\",\"CN=Microsoft Corporation\",\"2024.1.0.0\"",
        ]
        .join("\r\n")
    }

    #[test]
    fn parses_the_header_and_rows() {
        let catalog = SystemCatalog::parse(CATALOG);
        let packages = parse_packages(&sample(), &catalog);
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "Microsoft.BingNews");
        assert_eq!(packages[0].version, "4.12.0.0");
        assert_eq!(packages[0].full_name, "Microsoft.BingNews_4.12_x64__8wekyb3d8bbwe");
    }

    #[test]
    fn a_comma_inside_a_quoted_publisher_is_preserved() {
        let catalog = SystemCatalog::parse(CATALOG);
        let packages = parse_packages(&sample(), &catalog);
        assert_eq!(packages[0].publisher, "CN=Microsoft Corporation, O=Microsoft");
        assert_eq!(packages[0].publisher_short(), "Microsoft Corporation");
    }

    #[test]
    fn classifies_against_the_catalog() {
        let catalog = SystemCatalog::parse(CATALOG);
        let packages = parse_packages(&sample(), &catalog);
        let store = packages
            .iter()
            .find(|p| p.name == "Microsoft.WindowsStore")
            .unwrap();
        assert!(store.is_system);
        assert!(!packages[0].is_system);
    }

    #[test]
    fn skips_the_type_hint_line_that_convert_to_csv_adds() {
        let catalog = SystemCatalog::parse(CATALOG);
        let with_hint = format!("#TYPE Selected.System.Object\r\n{}", sample());
        assert_eq!(parse_packages(&with_hint, &catalog).len(), 2);
    }

    #[test]
    fn columns_are_found_by_name_not_position() {
        let catalog = SystemCatalog::parse(CATALOG);
        let reordered = "\"Version\",\"Name\",\"PackageFullName\",\"Publisher\"\r\n\"1.0.0.0\",\"A.B\",\"A.B_1\",\"CN=Someone\"";
        let packages = parse_packages(reordered, &catalog);
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "A.B");
        assert_eq!(packages[0].version, "1.0.0.0");
    }

    #[test]
    fn a_missing_name_column_yields_an_empty_inventory_rather_than_panicking() {
        let catalog = SystemCatalog::parse(CATALOG);
        let wrong = "\"Publisher\",\"Version\"\r\n\"CN=X\",\"1.0\"";
        assert!(parse_packages(wrong, &catalog).is_empty());
    }

    #[test]
    fn empty_output_yields_an_empty_inventory() {
        let catalog = SystemCatalog::parse(CATALOG);
        assert!(parse_packages("", &catalog).is_empty());
        assert!(parse_packages("   \r\n\r\n", &catalog).is_empty());
        assert!(parse_packages("\"Name\",\"PackageFullName\"", &catalog).is_empty());
    }

    #[test]
    fn duplicate_package_full_names_are_removed() {
        let catalog = SystemCatalog::parse(CATALOG);
        let duplicated = "\"Name\",\"PackageFullName\"\r\n\"A.B\",\"A.B_1\"\r\n\"A.B\",\"A.B_1\"";
        assert_eq!(parse_packages(duplicated, &catalog).len(), 1);
    }

    #[test]
    fn rows_with_an_empty_name_are_skipped() {
        let catalog = SystemCatalog::parse(CATALOG);
        let blank = "\"Name\",\"PackageFullName\"\r\n\"\",\"\"\r\n\"A.B\",\"A.B_1\"";
        let packages = parse_packages(blank, &catalog);
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "A.B");
    }

    #[test]
    fn results_are_sorted_by_name() {
        let catalog = SystemCatalog::parse(CATALOG);
        let unsorted = "\"Name\",\"PackageFullName\"\r\n\"Z.Z\",\"Z.Z_1\"\r\n\"A.A\",\"A.A_1\"";
        let packages = parse_packages(unsorted, &catalog);
        assert_eq!(packages[0].name, "A.A");
    }

    #[test]
    fn parse_row_handles_escaped_quotes() {
        assert_eq!(parse_row("\"a\"\"b\",c"), vec!["a\"b", "c"]);
        assert_eq!(parse_row("a,b,c"), vec!["a", "b", "c"]);
        assert_eq!(parse_row("\"\""), vec![""]);
    }

    #[test]
    fn discovery_reports_a_powershell_failure_rather_than_a_blank_list() {
        use crate::runner::ScriptedRunner;
        let runner = ScriptedRunner::new(vec![crate::runner::RunOutput {
            status: 1,
            stdout: "Access denied".into(),
            stderr: String::new(),
        }]);
        let catalog = SystemCatalog::parse(CATALOG);
        let err = discover(&runner, &catalog).unwrap_err();
        match err {
            DebrisError::Failed { detail, .. } => assert!(detail.contains("Access denied")),
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[test]
    fn discovery_returns_packages_on_success() {
        use crate::runner::ScriptedRunner;
        let runner = ScriptedRunner::new(vec![crate::runner::RunOutput::ok(0, sample())]);
        let catalog = SystemCatalog::parse(CATALOG);
        assert_eq!(discover(&runner, &catalog).unwrap().len(), 2);
    }
}
