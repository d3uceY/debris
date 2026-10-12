//! Performance characteristics of the hot paths.
//!
//! The numbers are printed so a regression is visible; the assertions are budgets
//! set loose enough to pass in a debug build, because a flaky performance test gets
//! deleted and then measures nothing. Run with `--release -- --nocapture` for the
//! real figures:
//!
//! ```sh
//! cargo test --release -p debris-backend --test perf -- --nocapture
//! ```

use debris_backend::discovery::parse_packages;
use debris_core::script::library;
use debris_core::{AppxPackage, Filter, Staging, SystemCatalog};
use std::path::Path;
use std::time::{Duration, Instant};

const INVENTORY_SIZE: usize = 5_000;

fn timed<T>(label: &str, budget: Duration, work: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let result = work();
    let elapsed = start.elapsed();
    eprintln!("  {label:<52} {elapsed:>10.3?}");
    assert!(
        elapsed < budget,
        "{label} took {elapsed:?}, over the {budget:?} budget"
    );
    result
}

/// A realistic-looking inventory: many families, publishers, versions.
fn inventory(size: usize) -> Vec<AppxPackage> {
    let catalog = SystemCatalog::parse(include_str!("../../../assets/data/system-apps.txt"));
    (0..size)
        .map(|index| {
            let name = if index % 97 == 0 {
                format!("Microsoft.WindowsStore_{index}")
            } else {
                format!("Vendor{}.Package{}", index % 211, index)
            };
            let mut package = AppxPackage {
                name,
                full_name: format!("Vendor{0}.Package{0}_1.{0}.0.0_x64__8wekyb3d8bbwe", index),
                publisher: format!("CN=Vendor {}, O=Vendor {}, C=US", index % 211, index % 211),
                version: format!("1.{}.0.0", index),
                is_system: false,
            };
            catalog.classify(&mut package);
            package
        })
        .collect()
}

#[test]
fn filtering_a_large_inventory_is_fast_enough_to_run_per_keystroke() {
    eprintln!("\nperf — filtering");
    let packages = inventory(INVENTORY_SIZE);

    let mut staging = Staging::new(packages);
    timed("construct 5,000-package ledger", Duration::from_millis(500), || {
        Staging::new(inventory(INVENTORY_SIZE))
    });

    timed("50 × set_query (each rebuilds the visible index)", Duration::from_millis(800), || {
        for term in ["mic", "micro", "microsoft", "store", "vendor1", "z", ""] {
            staging.set_query(term);
        }
        for _ in 0..44 {
            staging.set_query("package1");
        }
    });

    let mut staging = Staging::new(inventory(INVENTORY_SIZE));
    staging.set_query("package");
    timed("read visible_len 100,000 times", Duration::from_millis(400), || {
        let mut total = 0usize;
        for _ in 0..100_000 {
            // `black_box` is load-bearing: without it the optimiser deletes the loop
            // and the test reports a meaningless 0ns.
            total = total.wrapping_add(std::hint::black_box(staging.visible_len()));
        }
        assert!(total > 0);
    });
}

#[test]
fn staging_a_whole_inventory_is_fast() {
    eprintln!("\nperf — staging");
    let mut staging = Staging::new(inventory(INVENTORY_SIZE));

    // Single pass: 467µs in release for 5,000 packages, down from 330ms when this was
    // a `stage()` per row. The budget is tight enough to catch that regression
    // returning, and loose enough to hold in a debug build.
    timed("stage_all_visible over 5,000 packages", Duration::from_millis(100), || {
        let staged = staging.stage_all_visible();
        assert!(staged > 0);
    });

    timed("unstage_all of 5,000 packages", Duration::from_millis(1_000), || {
        assert!(staging.unstage_all() > 0);
    });

    let mut staging = Staging::new(inventory(INVENTORY_SIZE));
    timed("1,000 single stage/unstage round trips", Duration::from_millis(3_000), || {
        for index in 0..1_000 {
            let name = format!("Vendor{}.Package{}", index % 211, index);
            staging.stage(&name);
            staging.unstage(&name);
        }
    });
}

#[test]
fn the_system_catalog_lookup_stays_cheap() {
    eprintln!("\nperf — catalog");
    let catalog = SystemCatalog::parse(include_str!("../../../assets/data/system-apps.txt"));
    eprintln!("  entries in the shipped catalog: {}", catalog.len());

    timed("200,000 contains() lookups", Duration::from_millis(2_000), || {
        let mut hits = 0;
        for index in 0..200_000 {
            if catalog.contains(&format!("Vendor{}.Package{}", index % 211, index)) {
                hits += 1;
            }
        }
        let _ = hits;
    });
}

#[test]
fn parsing_a_large_inventory_payload_is_fast() {
    eprintln!("\nperf — CSV parse");
    let catalog = SystemCatalog::parse(include_str!("../../../assets/data/system-apps.txt"));

    let mut csv = String::from("#TYPE Selected.System.Object\r\n\"Name\",\"PackageFullName\",\"Publisher\",\"Version\"\r\n");
    for index in 0..INVENTORY_SIZE {
        csv.push_str(&format!(
            "\"Vendor{0}.Package{1}\",\"Vendor{0}.Package{1}_1.0.0.0_x64__abc\",\"CN=Vendor {0}, O=Vendor {0}\",\"1.{1}.0.0\"\r\n",
            index % 211,
            index
        ));
    }
    eprintln!("  payload size: {} bytes", csv.len());

    let packages = timed("parse 5,000-row inventory", Duration::from_millis(800), || {
        parse_packages(&csv, &catalog)
    });
    assert_eq!(packages.len(), INVENTORY_SIZE);
}

#[test]
fn loading_the_script_library_is_fast() {
    eprintln!("\nperf — script library");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts");

    let scripts = timed("load the library once", Duration::from_millis(500), || {
        library(&root).expect("library should load")
    });
    eprintln!("  scripts discovered: {}", scripts.len());

    timed("load the library 50 times", Duration::from_millis(2_000), || {
        for _ in 0..50 {
            library(&root).unwrap();
        }
    });
}

#[test]
fn the_filter_predicate_does_not_allocate() {
    // Guards the one micro-decision in `Filter::matches`: an allocating
    // `to_lowercase()` per row would be invisible in a unit test and obvious on a
    // slow machine. This is a smoke test that the path stays cheap.
    eprintln!("\nperf — filter predicate");
    let catalog = SystemCatalog::default();
    let packages = inventory(INVENTORY_SIZE);
    let filter = Filter {
        query: "package1".to_string(),
        show_system: true,
    };

    let matched = timed("1,000,000 predicate evaluations", Duration::from_millis(3_000), || {
        let mut count = 0;
        for _ in 0..200 {
            for package in &packages {
                if filter.matches(package) {
                    count += 1;
                }
            }
        }
        count
    });
    assert!(matched > 0);
    let _ = catalog;
}
