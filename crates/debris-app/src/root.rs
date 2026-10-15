//! The application view.
//!
//! This is the only place in the codebase that knows about both PowerShell and
//! pixels. Everything the interface asks for arrives as a method on [`DebrisApp`];
//! everything the interface draws leaves as a [`Snapshot`].
//!
//! # Threading
//!
//! `Get-AppxPackage` takes a second or two, PowerShell costs a fifth of a second to
//! start, and a network fetch can take much longer. So no handler blocks the render
//! thread. [`Root::run_off_thread`] is the single place that bridges: it runs the
//! blocking part on GPUI's background executor and applies the result back on the
//! main thread. Handlers stay readable because the bridging is not repeated at every
//! call site.
//!
//! # What lives here
//!
//! State the surfaces share, and nothing else. The fixer catalogue, the ticked sets,
//! the installed extensions and the settings are all fields, and [`Root::sync`] is the
//! one function that copies them into the snapshot a frame draws from.

use crate::bundle;
use crate::paths;
use debris_backend::runner::{CommandRunner, ProcessRunner};
use debris_backend::{
    apply_tweaks, discovery, extensions as ext_runner, fetch as fetch_url, registry, removal,
    reset_tweaks, scan_tweaks, scripts as script_runner,
};
use debris_core::{
    AppxPackage, Extension, Preset, RemovalPlan, ScriptMeta, Settings, Staging, SystemCatalog,
    Tweak, TweakScan, TweakStatus,
};
use debris_ui::snapshot::{CatalogEntry, Pending, Screen, Snapshot, Status};
use debris_ui::theme::Theme;
use debris_ui::views::DebrisApp;
use gpui::{
    AsyncApp, Context, FocusHandle, IntoElement, Render, WeakEntity, Window, div, prelude::*,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct Root {
    catalog: SystemCatalog,
    staging: Staging,

    scripts: Arc<Vec<ScriptMeta>>,
    script_selection: HashSet<String>,
    presets: Arc<Vec<Preset>>,

    /// Every fixer this install knows about: the shipped catalogue merged with the
    /// user's own files. Filtered by Windows version in [`Root::sync`], because the
    /// build is not known until the first probe comes back.
    tweaks: Vec<Tweak>,
    tweak_selection: HashSet<String>,
    tweak_scans: HashMap<String, TweakScan>,

    extensions: Vec<CatalogEntry>,
    installed_extensions: HashMap<String, String>,

    settings: Settings,
    snapshot: Snapshot,
    theme: Theme,
    runner: Arc<dyn CommandRunner>,

    search_focus: FocusHandle,
    list_focus: FocusHandle,
    confirm_focus: FocusHandle,
    source_focus: FocusHandle,

    busy: bool,
    scanned: bool,
    /// Whether the environment probe has reported the build. `None` keeps the
    /// Windows 11 rules visible, which is the better guess before an answer arrives.
    probe: Option<debris_backend::Environment>,
    error: Option<String>,
    elevation: Option<bool>,
}

impl Root {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let catalog = bundle::system_catalog();
        let bloat = bundle::bloatware();
        let scripts = match bundle::scripts() {
            Ok(scripts) => scripts,
            Err(error) => {
                // A library that will not load is worth saying out loud rather than
                // showing as an empty list with no explanation.
                eprintln!("debris: {error}");
                Vec::new()
            }
        };
        let presets = bundle::presets();

        // Shipped rules first, then the user's, so a custom file can deliberately
        // replace a shipped rule by reusing its name.
        let tweaks = merge_tweaks(bundle::fixers(), paths::load_custom_tweaks());

        let settings = paths::load_settings();
        let installed_extensions = paths::load_installed();
        let extensions = load_cached_catalogues(&settings.extension_sources);

        // The catalogue's recommended set is ticked from the start, so the surface
        // opens on something rather than on an empty page and a question.
        let tweak_selection: HashSet<String> = tweaks
            .iter()
            .filter(|tweak| tweak.default_selected)
            .map(|tweak| tweak.name.clone())
            .collect();

        let theme = if settings.dark {
            Theme::dark()
        } else {
            Theme::light()
        };

        let mut snapshot = Snapshot::new();
        snapshot.scripts = Arc::new(scripts.clone());
        snapshot.presets = Arc::new(presets.clone());
        snapshot.bloat = Arc::new(bloat.clone());
        snapshot.status = Status::Info("Press Rescan to read the package inventory.".to_string());

        let mut root = Self {
            catalog,
            staging: Staging::default(),
            scripts: Arc::new(scripts),
            script_selection: HashSet::new(),
            presets: Arc::new(presets),
            tweaks,
            tweak_selection,
            tweak_scans: HashMap::new(),
            extensions,
            installed_extensions,
            settings,
            snapshot,
            theme,
            runner: Arc::new(ProcessRunner),
            search_focus: cx.focus_handle(),
            list_focus: cx.focus_handle(),
            confirm_focus: cx.focus_handle(),
            source_focus: cx.focus_handle(),
            busy: false,
            scanned: false,
            probe: None,
            error: None,
            elevation: None,
        };

        cx.set_global(root.theme.clone());
        root.sync(cx);
        // Probing here rather than only on Rescan, because two things on the first
        // frame depend on the answer: which fixers apply to this Windows, and whether
        // to say that a run will need elevation before the user finds out the hard way.
        root.probe_environment(cx);
        root
    }

    /// Asks PowerShell what this session is, off the render thread.
    fn probe_environment(&self, cx: &mut Context<Self>) {
        let runner = self.runner.clone();
        self.run_off_thread(
            cx,
            move || script_runner::environment(&*runner),
            |root, probe, cx| {
                root.elevation = Some(probe.elevated);
                let changed = root.probe != Some(probe);
                root.probe = Some(probe);
                if changed {
                    root.reseed_default_tweaks();
                }
                root.sync(cx);
            },
        );
    }

    /// Rebuilds every derived value in the snapshot.
    ///
    /// This is the memoization the design asks for: the filtered inventory, the
    /// staged-name set and the OS-filtered fixer list are computed here, when something
    /// actually changes, rather than once per frame per row.
    fn sync(&mut self, cx: &mut Context<Self>) {
        let inventory: Vec<AppxPackage> = self
            .staging
            .visible()
            .iter()
            .filter_map(|index| self.staging.inventory().get(*index))
            .cloned()
            .collect();
        let staged: Vec<AppxPackage> = self.staging.removal().to_vec();

        self.snapshot.inventory = Arc::new(inventory);
        self.snapshot.staged_names = Arc::new(staged.iter().map(|p| p.name.clone()).collect());
        self.snapshot.plan = RemovalPlan::from_packages(&staged);
        self.snapshot.staged = Arc::new(staged);
        self.snapshot.counts = self.staging.counts();
        self.snapshot.query = self.staging.query().to_string();
        self.snapshot.show_system = self.staging.show_system();
        self.snapshot.scanned = self.scanned;
        self.snapshot.error = self.error.clone();
        self.snapshot.elevated = self.elevation;

        let tweaks: Vec<Tweak> = self
            .tweaks
            .iter()
            .filter(|tweak| self.applies(tweak))
            .cloned()
            .collect();
        self.snapshot.tweaks = Arc::new(tweaks);
        self.snapshot.tweak_selection = Arc::new(self.tweak_selection.clone());
        self.snapshot.tweak_scans = Arc::new(self.tweak_scans.clone());
        self.snapshot.backup_before_apply = self.settings.backup_before_apply;

        self.snapshot.script_selection = Arc::new(self.script_selection.clone());
        self.snapshot.extensions = Arc::new(self.extensions.clone());
        self.snapshot.extension_sources = Arc::new(self.settings.extension_sources.clone());
        self.snapshot.installed_extensions = Arc::new(self.installed_extensions.clone());

        self.snapshot.clamp_cursor();
        cx.notify();
    }

    /// Runs blocking work off the render thread, then applies the result on it.
    fn run_off_thread<W, R>(
        &self,
        cx: &mut Context<Self>,
        work: W,
        done: impl FnOnce(&mut Self, R, &mut Context<Self>) + 'static,
    ) where
        W: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        cx.spawn(
            async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                let output = cx.background_executor().spawn(async move { work() }).await;
                this.update(cx, |root, cx| done(root, output, cx)).ok();
            },
        )
        .detach();
    }

    fn begin(&mut self, cx: &mut Context<Self>, message: impl Into<String>) {
        let message = message.into();
        self.busy = true;
        self.snapshot.loading = Some(message.clone());
        self.snapshot.status = Status::Activity(message);
        cx.notify();
    }

    fn finish(&mut self, cx: &mut Context<Self>, status: Status) {
        self.busy = false;
        self.snapshot.loading = None;
        self.snapshot.status = status;
        self.sync(cx);
    }

    fn scan(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.error = None;
        self.begin(cx, "Reading the package inventory");

        let runner = self.runner.clone();
        let catalog = self.catalog.clone();

        self.run_off_thread(
            cx,
            move || {
                let probe = script_runner::environment(&*runner);
                let packages = discovery::discover(&*runner, &catalog);
                (probe, packages)
            },
            |root, (probe, packages), cx| {
                root.elevation = Some(probe.elevated);
                if root.probe != Some(probe) {
                    // A fixer hidden a second ago may now be offered, so the ticked set
                    // is taken from the catalogue again rather than left holding names
                    // that no longer render.
                    root.probe = Some(probe);
                    root.reseed_default_tweaks();
                }
                match packages {
                    Ok(packages) => {
                        let count = packages.len();
                        root.staging.set_inventory(packages);
                        root.scanned = true;
                        root.finish(
                            cx,
                            Status::Info(format!("{count} package(s) installed for this user.")),
                        );
                    }
                    Err(error) => {
                        root.error = Some(error.to_string());
                        root.finish(
                            cx,
                            Status::Error(
                                "The inventory could not be read. The panel says what PowerShell reported."
                                    .to_string(),
                            ),
                        );
                    }
                }
            },
        );
    }

    /// Whether this Windows is 11. The rule lives with the probe, so this file never
    /// has to know that 11 starts at build 22000.
    fn is_windows_11(&self) -> bool {
        self.probe.is_none_or(|probe| probe.is_windows_11())
    }

    /// Whether a fixer belongs on this machine.
    fn applies(&self, tweak: &Tweak) -> bool {
        tweak.applies_to(if self.is_windows_11() { 11 } else { 10 })
    }

    /// Takes the recommended set from the catalogue for the Windows this turned out
    /// to be.
    fn reseed_default_tweaks(&mut self) {
        let windows_11 = self.is_windows_11();
        self.tweak_selection = self
            .tweaks
            .iter()
            .filter(|tweak| {
                tweak.default_selected && tweak.applies_to(if windows_11 { 11 } else { 10 })
            })
            .map(|tweak| tweak.name.clone())
            .collect();
    }

    /// The ticked fixers as values, in catalogue order.
    fn selected_tweaks(&self) -> Vec<Tweak> {
        self.snapshot
            .selected_tweaks()
            .into_iter()
            .cloned()
            .collect()
    }

    fn run_removal(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.snapshot.plan.is_empty() {
            return;
        }
        let plan = self.snapshot.plan.clone();
        let count = plan.len();
        self.begin(cx, format!("Removing {count} package(s)"));

        let runner = self.runner.clone();
        self.run_off_thread(
            cx,
            move || removal::execute(&plan, &*runner),
            |root, outcomes, cx| {
                let removed = outcomes.iter().filter(|outcome| outcome.removed).count();
                let failed = outcomes.len().saturating_sub(removed);
                let status = if failed == 0 {
                    Status::Info(format!("Removed {removed} package(s)."))
                } else {
                    let first = outcomes
                        .iter()
                        .find(|outcome| !outcome.removed)
                        .map(|outcome| outcome.summary())
                        .unwrap_or_default();
                    Status::Error(format!("Removed {removed}, {failed} failed. {first}"))
                };
                root.finish(cx, status);
                // The inventory on disk has changed, so the one on screen is stale
                // until it is asked again.
                root.scan(cx);
            },
        );
    }

    /// Reads the registry for every ticked fixer, in one process.
    fn analyze_fixers(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let selected = self.selected_tweaks();
        if selected.is_empty() {
            self.snapshot.status = Status::Caution("Tick a fixer to check it.".to_string());
            cx.notify();
            return;
        }

        let count = selected.len();
        let names: Vec<String> = selected.iter().map(|tweak| tweak.name.clone()).collect();
        self.begin(cx, format!("Reading {count} registry value(s)"));
        let runner = self.runner.clone();

        self.run_off_thread(
            cx,
            move || scan_tweaks(&*runner, &selected),
            move |root, result, cx| match result {
                Ok(scans) => {
                    let unset = scans
                        .iter()
                        .filter(|scan| scan.status == TweakStatus::NotApplied)
                        .count();
                    for (name, scan) in names.into_iter().zip(scans) {
                        root.tweak_scans.insert(name, scan);
                    }
                    let status = if unset == 0 {
                        Status::Info(format!("All {count} already match."))
                    } else {
                        Status::Info(format!("{unset} of {count} need writing."))
                    };
                    root.finish(cx, status);
                }
                Err(error) => root.finish(cx, Status::Error(error.to_string())),
            },
        );
    }

    /// Writes every ticked fixer, optionally after a `.reg` backup.
    fn run_fixers(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let selected = self.selected_tweaks();
        if selected.is_empty() {
            return;
        }

        let count = selected.len();
        self.begin(cx, format!("Writing {count} fixer(s)"));
        let runner = self.runner.clone();
        let backup = self.settings.backup_before_apply;

        self.run_off_thread(
            cx,
            move || {
                // The backup is read before anything is written, and it is best effort:
                // failing to export is not a reason to refuse the run the user asked for.
                let backup_path = backup
                    .then(|| registry::backup(&*runner, &selected).ok())
                    .flatten()
                    .and_then(|text| paths::save_backup(&text).ok());
                let outcomes = apply_tweaks(&*runner, &selected);
                (backup_path, outcomes)
            },
            |root, (backup_path, outcomes), cx| {
                let failed = outcomes.iter().filter(|outcome| !outcome.ok).count();
                let applied = outcomes.len().saturating_sub(failed);
                let mut message = if failed == 0 {
                    format!("Wrote {applied} fixer(s).")
                } else {
                    let first = outcomes
                        .iter()
                        .find(|outcome| !outcome.ok)
                        .map(|outcome| outcome.detail.clone())
                        .unwrap_or_default();
                    format!("Wrote {applied}, {failed} failed. {first}")
                };
                if let Some(path) = backup_path {
                    message.push_str(&format!(" Backup: {}", path.display()));
                }
                let status = if failed == 0 {
                    Status::Info(message)
                } else {
                    Status::Error(message)
                };
                root.finish(cx, status);
                // A write changes what a scan would report, so the chips are stale now.
                root.analyze_fixers(cx);
            },
        );
    }

    /// Puts the ticked fixers back to the value Windows ships with.
    fn restore_fixers(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let selected: Vec<Tweak> = self
            .selected_tweaks()
            .into_iter()
            .filter(|tweak| tweak.is_restorable())
            .collect();
        if selected.is_empty() {
            return;
        }

        let count = selected.len();
        self.begin(cx, format!("Restoring {count} fixer(s)"));
        let runner = self.runner.clone();

        self.run_off_thread(
            cx,
            move || reset_tweaks(&*runner, &selected),
            |root, outcomes, cx| {
                let failed = outcomes.iter().filter(|outcome| !outcome.ok).count();
                let restored = outcomes.len().saturating_sub(failed);
                let status = if failed == 0 {
                    Status::Info(format!("Restored {restored} fixer(s) to the Windows default."))
                } else {
                    let first = outcomes
                        .iter()
                        .find(|outcome| !outcome.ok)
                        .map(|outcome| outcome.detail.clone())
                        .unwrap_or_default();
                    Status::Error(format!("Restored {restored}, {failed} failed. {first}"))
                };
                root.finish(cx, status);
                root.analyze_fixers(cx);
            },
        );
    }

    /// Runs every ticked script, one process each, and reports the ones that failed.
    fn run_scripts(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let selected: Vec<ScriptMeta> = self
            .snapshot
            .selected_scripts()
            .into_iter()
            .cloned()
            .collect();
        if selected.is_empty() {
            return;
        }

        let count = selected.len();
        self.begin(cx, format!("Running {count} script(s)"));
        let runner = self.runner.clone();

        self.run_off_thread(
            cx,
            move || {
                let mut failures: Vec<String> = Vec::new();
                for script in &selected {
                    let outcome = script_runner::run_script(script, &*runner);
                    match outcome {
                        Ok(output) if output.succeeded() => {}
                        Ok(output) => failures.push(format!(
                            "{}: {}",
                            script.title,
                            first_line(&output.combined())
                        )),
                        Err(error) => failures.push(format!("{}: {error}", script.title)),
                    }
                }
                failures
            },
            move |root, failures, cx| {
                let status = if failures.is_empty() {
                    Status::Info(format!("Ran {count} script(s)."))
                } else {
                    Status::Error(format!(
                        "{count} script(s) run, {} failed. {}",
                        failures.len(),
                        failures[0]
                    ))
                };
                root.finish(cx, status);
            },
        );
    }

    /// Fetches one catalogue and folds its entries into the list.
    fn load_catalogue(&mut self, cx: &mut Context<Self>, url: String) {
        let runner = self.runner.clone();
        self.run_off_thread(
            cx,
            move || {
                let (body, extensions) = ext_runner::load(&*runner, &url)?;
                // Cached so the entries survive a restart and still render offline.
                let _ = paths::cache_catalogue(&url, &body);
                Ok::<_, debris_core::DebrisError>((url, extensions))
            },
            |root, result, cx| {
                root.busy = false;
                root.snapshot.loading = None;
                root.snapshot.status = match result {
                    Ok((url, extensions)) => {
                        root.extensions.retain(|entry| entry.source != url);
                        let count = extensions.len();
                        root.extensions.extend(
                            extensions.into_iter().map(|extension| CatalogEntry {
                                source: url.clone(),
                                extension,
                            }),
                        );
                        Status::Info(format!("{count} entries loaded from {url}."))
                    }
                    Err(error) => Status::Error(error.to_string()),
                };
                root.sync(cx);
            },
        );
    }

    /// Downloads one extension's script, verifying it when the catalogue published a
    /// checksum.
    fn install_extension(&mut self, url: &str, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(entry) = self
            .extensions
            .iter()
            .find(|entry| entry.url() == url)
            .cloned()
        else {
            self.snapshot.status = Status::Error("That entry is no longer in a catalogue.".into());
            cx.notify();
            return;
        };

        let name = entry.extension.name.clone();
        let version = ext_runner::recorded_version(&entry.extension);
        let expected = entry.extension.sha256.clone();
        // Owned because the download happens off the render thread, and the closure
        // that does it has to outlive this borrow of the frame.
        let url = url.to_string();
        self.begin(cx, format!("Downloading {name}"));
        let runner = self.runner.clone();
        self.run_off_thread(
            cx,
            move || -> std::result::Result<(), String> {
                let body = fetch_url(&*runner, &url).map_err(|error| error.to_string())?;
                let path = paths::stage_extension(&url, &body).map_err(|error| error.to_string())?;
                // A copy that does not verify is deleted before anything records it,
                // so the extensions folder never holds a script the app refused.
                if let Err(error) = ext_runner::verify(&*runner, &path, expected.as_deref()) {
                    let _ = paths::remove_extension(&url);
                    return Err(error);
                }
                paths::record_installed(&url, &version).map_err(|error| error.to_string())
            },
            move |root, result, cx| match result {
                Ok(()) => {
                    root.installed_extensions = paths::load_installed();
                    root.finish(cx, Status::Info(format!("Installed {name}.")));
                }
                // A refusal to verify is not silent: it says which copy disagreed,
                // which is the difference between a stale catalogue and a tampered
                // download.
                Err(error) => root.finish(cx, Status::Error(error)),
            },
        );
    }

    /// Runs one installed extension.
    fn run_extension(&mut self, url: &str, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(path) = paths::extension_path(url) else {
            self.snapshot.status = Status::Error("That extension has no local file.".into());
            cx.notify();
            return;
        };
        if !path.is_file() {
            self.snapshot.status = Status::Error("That extension is not installed.".to_string());
            cx.notify();
            return;
        }

        let name = self
            .extensions
            .iter()
            .find(|entry| entry.url() == url)
            .map(|entry| entry.extension.name.clone())
            .unwrap_or_else(|| "extension".to_string());
        self.begin(cx, format!("Running {name}"));
        let runner = self.runner.clone();

        self.run_off_thread(
            cx,
            move || script_runner::run_path(&path, &*runner),
            move |root, result, cx| {
                let status = match result {
                    Ok(output) if output.succeeded() => Status::Info(format!("{name} finished.")),
                    Ok(output) => {
                        Status::Error(format!("{name}: {}", first_line(&output.combined())))
                    }
                    Err(error) => Status::Error(error.to_string()),
                };
                root.finish(cx, status);
            },
        );
    }

    /// Forgets an installed extension and deletes its script.
    fn uninstall_extension(&mut self, url: &str, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let name = self
            .extensions
            .iter()
            .find(|entry| entry.url() == url)
            .map(|entry| entry.extension.name.clone())
            .unwrap_or_else(|| url.to_string());
        match paths::remove_extension(url) {
            Ok(()) => {
                self.installed_extensions = paths::load_installed();
                self.snapshot.status = Status::Info(format!("Removed {name}."));
            }
            Err(error) => {
                self.snapshot.status =
                    Status::Error(format!("Could not remove {name}: {error}"));
            }
        }
        self.sync(cx);
    }
}

/// Later entries win, so a custom file can replace a shipped rule by reusing its name.
fn merge_tweaks(shipped: Vec<Tweak>, custom: Vec<Tweak>) -> Vec<Tweak> {
    let mut merged = shipped;
    for tweak in custom {
        merged.retain(|existing| !existing.name.eq_ignore_ascii_case(&tweak.name));
        merged.push(tweak);
    }
    merged
}

/// Reads every catalogue body from the cache, so the previous session's entries are
/// on screen before any network call happens.
fn load_cached_catalogues(sources: &[String]) -> Vec<CatalogEntry> {
    let mut entries = Vec::new();
    for source in sources {
        let Some(body) = paths::cached_catalogue(source) else {
            continue;
        };
        let Ok(extensions) = Extension::parse_catalog(&body) else {
            continue;
        };
        entries.extend(extensions.into_iter().map(|extension| CatalogEntry {
            source: source.clone(),
            extension,
        }));
    }
    entries
}

fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("no output")
        .to_string()
}

impl Render for Root {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The whole tree is built from a clone of the snapshot, so no element holds
        // a borrow of `self` and no listener can conflict with the build.
        div().size_full().child(debris_ui::views::root(self, cx))
    }
}

impl DebrisApp for Root {
    fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    fn search_focus(&self) -> &FocusHandle {
        &self.search_focus
    }

    fn list_focus(&self) -> &FocusHandle {
        &self.list_focus
    }

    fn confirm_focus(&self) -> &FocusHandle {
        &self.confirm_focus
    }

    fn source_focus(&self) -> &FocusHandle {
        &self.source_focus
    }

    fn navigate(&mut self, screen: Screen, cx: &mut Context<Self>) {
        if self.snapshot.screen != screen {
            self.snapshot.screen = screen;
            cx.notify();
        }
    }

    fn set_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.staging.set_query(query);
        self.snapshot.cursor = 0;
        self.sync(cx);
    }

    fn set_show_system(&mut self, show: bool, cx: &mut Context<Self>) {
        self.staging.set_show_system(show);
        if show {
            self.snapshot.status = Status::Caution(
                "System components are shown. Windows needs these to work.".to_string(),
            );
        }
        self.sync(cx);
    }

    fn stage(&mut self, name: &str, cx: &mut Context<Self>) {
        let moved = self.staging.stage(name);
        if moved > 0 {
            self.snapshot.status = Status::Info(format!("Staged {name}"));
        }
        self.sync(cx);
    }

    fn unstage(&mut self, name: &str, cx: &mut Context<Self>) {
        let moved = self.staging.unstage(name);
        if moved > 0 {
            self.snapshot.status = Status::Info(format!("Returned {name} to the inventory"));
        }
        self.sync(cx);
    }

    fn stage_all(&mut self, cx: &mut Context<Self>) {
        let moved = self.staging.stage_all_visible();
        self.snapshot.status = Status::Info(format!("Staged {moved} package(s)"));
        self.sync(cx);
    }

    fn unstage_all(&mut self, cx: &mut Context<Self>) {
        let moved = self.staging.unstage_all();
        if moved > 0 {
            self.snapshot.status = Status::Info(format!("Cleared {moved} package(s) from staging"));
        }
        self.sync(cx);
    }

    fn scan(&mut self, cx: &mut Context<Self>) {
        Root::scan(self, cx);
    }

    // — the script library —

    fn toggle_script(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.script_selection.remove(id) {
            self.script_selection.insert(id.to_string());
        }
        self.sync(cx);
    }

    fn tick_all_scripts(&mut self, ticked: bool, cx: &mut Context<Self>) {
        if ticked {
            // Only what the library will actually run. Ticking a refused script and
            // failing on it at run time would be a promise the app cannot keep.
            self.script_selection = self
                .scripts
                .iter()
                .filter(|script| script.is_runnable())
                .map(|script| script.id.clone())
                .collect();
            self.snapshot.status = Status::Info(format!(
                "Ticked {} script(s).",
                self.script_selection.len()
            ));
        } else {
            self.script_selection.clear();
        }
        self.sync(cx);
    }

    fn request_scripts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.script_selection.is_empty() {
            return;
        }
        self.snapshot.pending = Some(Pending::Scripts);
        window.focus(&self.confirm_focus, cx);
        cx.notify();
    }

    // — the fixer catalogue —

    fn toggle_fixer(&mut self, name: &str, cx: &mut Context<Self>) {
        if !self.tweak_selection.remove(name) {
            self.tweak_selection.insert(name.to_string());
        }
        self.sync(cx);
    }

    fn tick_all_fixers(&mut self, ticked: bool, cx: &mut Context<Self>) {
        if ticked {
            self.tweak_selection = self
                .snapshot
                .tweaks
                .iter()
                .map(|tweak| tweak.name.clone())
                .collect();
        } else {
            self.tweak_selection.clear();
        }
        self.sync(cx);
    }

    fn analyze_fixers(&mut self, cx: &mut Context<Self>) {
        Root::analyze_fixers(self, cx);
    }

    fn request_fixers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.tweak_selection.is_empty() {
            return;
        }
        self.snapshot.pending = Some(Pending::Fixers);
        window.focus(&self.confirm_focus, cx);
        cx.notify();
    }

    fn request_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.snapshot.pending = Some(Pending::Restore);
        window.focus(&self.confirm_focus, cx);
        cx.notify();
    }

    fn toggle_backup(&mut self, cx: &mut Context<Self>) {
        self.settings.backup_before_apply = !self.settings.backup_before_apply;
        let _ = paths::save_settings(&self.settings);
        self.snapshot.status = if self.settings.backup_before_apply {
            Status::Info("A .reg backup will be written before each run.".to_string())
        } else {
            Status::Caution(
                "No backup will be written. Restore always uses the Windows default."
                    .to_string(),
            )
        };
        self.sync(cx);
    }

    // — the extension catalogues —

    fn set_source_input(&mut self, value: String, cx: &mut Context<Self>) {
        self.snapshot.source_input = value;
        cx.notify();
    }

    fn add_source(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let url = self.snapshot.source_input.trim().to_string();
        if !debris_core::is_fetchable(&url) {
            self.snapshot.status =
                Status::Error("A catalogue has to be an http(s) address.".to_string());
            cx.notify();
            return;
        }
        self.snapshot.source_input.clear();
        if !self.settings.add_source(&url) {
            self.snapshot.status = Status::Info(format!("{url} is already a catalogue."));
            cx.notify();
            return;
        }
        let _ = paths::save_settings(&self.settings);
        self.begin(cx, format!("Fetching {url}"));
        self.load_catalogue(cx, url);
    }

    fn remove_source(&mut self, url: &str, cx: &mut Context<Self>) {
        if self.settings.remove_source(url) {
            let _ = paths::save_settings(&self.settings);
        }
        // The cache goes with the source: keeping a body nothing points at is how a
        // directory grows a copy of a file the user deliberately removed.
        paths::forget_catalogue(url);
        self.extensions.retain(|entry| entry.source != url);
        self.snapshot.status = Status::Info(format!("Removed {url}."));
        self.sync(cx);
    }

    fn refresh_catalogues(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.settings.extension_sources.is_empty() {
            return;
        }
        let sources = self.settings.extension_sources.clone();
        let count = sources.len();
        self.begin(cx, format!("Fetching {count} catalogue(s)"));
        let runner = self.runner.clone();

        self.run_off_thread(
            cx,
            move || {
                let mut loaded = Vec::new();
                let mut failed = Vec::new();
                for source in sources {
                    match ext_runner::load(&*runner, &source) {
                        Ok((body, extensions)) => {
                            let _ = paths::cache_catalogue(&source, &body);
                            loaded.push((source, extensions));
                        }
                        Err(error) => failed.push(format!("{source}: {error}")),
                    }
                }
                (loaded, failed)
            },
            |root, (loaded, failed), cx| {
                for (url, extensions) in loaded {
                    root.extensions.retain(|entry| entry.source != url);
                    root.extensions.extend(
                        extensions
                            .into_iter()
                            .map(|extension| CatalogEntry {
                                source: url.clone(),
                                extension,
                            }),
                    );
                }
                let status = if failed.is_empty() {
                    Status::Info(format!("{} entries loaded.", root.extensions.len()))
                } else {
                    Status::Error(format!("{} catalogue(s) failed. {}", failed.len(), failed[0]))
                };
                root.finish(cx, status);
            },
        );
    }

    fn install_extension(&mut self, url: &str, cx: &mut Context<Self>) {
        Root::install_extension(self, url, cx);
    }

    fn uninstall_extension(&mut self, url: &str, cx: &mut Context<Self>) {
        Root::uninstall_extension(self, url, cx);
    }

    fn request_extension(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if !self.installed_extensions.contains_key(url) {
            self.snapshot.status = Status::Error("Install it first.".to_string());
            cx.notify();
            return;
        }
        self.snapshot.pending = Some(Pending::Extension(url.to_string()));
        window.focus(&self.confirm_focus, cx);
        cx.notify();
    }

    fn request_removal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.snapshot.plan.is_empty() {
            return;
        }
        self.snapshot.pending = Some(Pending::Removal);
        window.focus(&self.confirm_focus, cx);
        cx.notify();
    }

    fn confirm_pending(&mut self, cx: &mut Context<Self>) {
        match self.snapshot.pending.take() {
            Some(Pending::Removal) => self.run_removal(cx),
            Some(Pending::Scripts) => self.run_scripts(cx),
            Some(Pending::Fixers) => self.run_fixers(cx),
            Some(Pending::Restore) => self.restore_fixers(cx),
            Some(Pending::Extension(url)) => self.run_extension(&url, cx),
            None => {}
        }
        cx.notify();
    }

    fn cancel_pending(&mut self, cx: &mut Context<Self>) {
        if self.snapshot.pending.take().is_some() {
            cx.notify();
        }
    }

    fn load_preset(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(preset) = self.presets.iter().find(|preset| preset.name == name).cloned() else {
            self.snapshot.status = Status::Error(format!("No preset called {name}."));
            cx.notify();
            return;
        };
        let loaded = self.staging.load_removal(&preset.packages);
        self.snapshot.screen = Screen::Packages;
        self.snapshot.status = if loaded == 0 {
            Status::Caution(format!(
                "{} names {} package(s); none of them are installed.",
                preset.name,
                preset.len()
            ))
        } else {
            Status::Info(format!(
                "Loaded {loaded} of {} package(s) from {}",
                preset.len(),
                preset.name
            ))
        };
        self.sync(cx);
    }

    fn toggle_appearance(&mut self, cx: &mut Context<Self>) {
        self.theme.toggle_appearance();
        self.settings.dark = matches!(self.theme.appearance, debris_ui::theme::Appearance::Dark);
        let _ = paths::save_settings(&self.settings);
        cx.set_global(self.theme.clone());
        self.sync(cx);
    }

    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.snapshot.sidebar_open = !self.snapshot.sidebar_open;
        cx.notify();
    }

    fn toggle_inspector(&mut self, cx: &mut Context<Self>) {
        self.snapshot.inspector_open = !self.snapshot.inspector_open;
        cx.notify();
    }

    fn move_cursor(&mut self, delta: i32, cx: &mut Context<Self>) {
        let len = self.snapshot.inventory.len();
        if len == 0 {
            return;
        }
        let next = (self.snapshot.cursor as i32 + delta).clamp(0, len as i32 - 1) as usize;
        if next != self.snapshot.cursor {
            self.snapshot.cursor = next;
            cx.notify();
        }
    }

    fn activate_cursor(&mut self, cx: &mut Context<Self>) {
        let Some(package) = self.staging.visible_package(self.snapshot.cursor).cloned() else {
            return;
        };
        if self.staging.is_staged(&package.name) {
            self.unstage(&package.name, cx);
        } else {
            self.stage(&package.name, cx);
        }
    }
}
