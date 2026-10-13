//! The application view.
//!
//! This is the only place in the codebase that knows about both PowerShell and
//! pixels. Everything the interface asks for arrives as a method on
//! [`DebrisApp`]; everything the interface draws leaves as a [`Snapshot`].
//!
//! # Threading
//!
//! `Get-AppxPackage` takes a second or two and a script can take much longer, so no
//! handler blocks the render thread. [`Root::run_off_thread`] is the single place that
//! bridges: it runs the blocking part on GPUI's background executor and applies the
//! result back on the main thread. Handlers stay readable because the bridging is not
//! repeated at every call site.

use crate::paths::Layout;
use debris_backend::runner::{CommandRunner, ProcessRunner};
use debris_backend::{discovery, is_elevated, removal, scripts as script_runner};
use debris_core::{
    AppxPackage, Preset, RemovalPlan, ScriptMeta, Staging, SystemCatalog,
};
use debris_ui::prelude::*;
use gpui::{
    AsyncApp, Context, FocusHandle, IntoElement, Render, WeakEntity, Window, div, prelude::*,
};
use std::sync::Arc;

pub struct Root {
    catalog: SystemCatalog,
    staging: Staging,
    scripts: Arc<Vec<ScriptMeta>>,
    presets: Arc<Vec<Preset>>,
    snapshot: Snapshot,
    theme: Theme,
    runner: Arc<dyn CommandRunner>,

    search_focus: FocusHandle,
    list_focus: FocusHandle,
    confirm_focus: FocusHandle,

    busy: bool,
    scanned: bool,
    error: Option<String>,
    elevation: Option<bool>,
    dry_run: bool,
}

impl Root {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let layout = Layout::discover();
        let catalog = layout.system_catalog();
        let scripts = match layout.scripts() {
            Ok(scripts) => scripts,
            Err(error) => {
                // A library that will not load is worth saying out loud rather than
                // showing as an empty list with no explanation.
                eprintln!("debris: {error}");
                Vec::new()
            }
        };
        let presets = layout.presets();

        let theme = Theme::light();
        let mut snapshot = Snapshot::new();
        snapshot.scripts = Arc::new(scripts.clone());
        snapshot.presets = Arc::new(presets.clone());
        snapshot.dry_run = true;
        snapshot.status = Status::Info("Press Rescan to read the package inventory.".to_string());

        let mut root = Self {
            catalog,
            staging: Staging::default(),
            scripts: Arc::new(scripts),
            presets: Arc::new(presets),
            snapshot,
            theme,
            runner: Arc::new(ProcessRunner),
            search_focus: cx.focus_handle(),
            list_focus: cx.focus_handle(),
            confirm_focus: cx.focus_handle(),
            busy: false,
            scanned: false,
            error: None,
            elevation: None,
            dry_run: true,
        };

        cx.set_global(root.theme.clone());
        root.sync(cx);
        root
    }

    /// Rebuilds every derived value in the snapshot.
    ///
    /// This is the memoization the design asks for: the filtered inventory and the
    /// staged-name set are computed here, when something actually changes, rather
    /// than once per frame per visible row.
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
        self.snapshot.dry_run = self.dry_run;
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

    fn scan(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.error = None;
        self.snapshot.loading = Some("Reading the package inventory".to_string());
        self.snapshot.status = Status::Activity("Reading the package inventory".to_string());
        cx.notify();

        let runner = self.runner.clone();
        let catalog = self.catalog.clone();

        self.run_off_thread(
            cx,
            move || {
                let elevated = is_elevated(&*runner);
                let packages = discovery::discover(&*runner, &catalog);
                (elevated, packages)
            },
            |root, (elevated, packages), cx| {
                root.elevation = Some(elevated);
                root.busy = false;
                root.snapshot.loading = None;
                match packages {
                    Ok(packages) => {
                        let count = packages.len();
                        root.staging.set_inventory(packages);
                        root.scanned = true;
                        root.snapshot.status = Status::Info(format!(
                            "{count} package(s) installed for this user."
                        ));
                    }
                    Err(error) => {
                        root.error = Some(error.to_string());
                        root.snapshot.status = Status::Error(
                            "The inventory could not be read. The panel says what PowerShell reported."
                                .to_string(),
                        );
                    }
                }
                root.sync(cx);
            },
        );
    }

    fn run_removal(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.snapshot.plan.is_empty() {
            return;
        }
        self.busy = true;
        let plan = self.snapshot.plan.clone();
        let count = plan.len();
        self.snapshot.loading = Some(format!("Removing {count} package(s)"));
        self.snapshot.status = Status::Activity(format!("Removing {count} package(s)"));
        cx.notify();

        let runner = self.runner.clone();
        self.run_off_thread(
            cx,
            move || removal::execute(&plan, &*runner),
            |root, outcomes, cx| {
                let removed = outcomes.iter().filter(|outcome| outcome.removed).count();
                let failed = outcomes.len().saturating_sub(removed);
                root.busy = false;
                root.snapshot.loading = None;
                root.snapshot.status = if failed == 0 {
                    Status::Info(format!("Removed {removed} package(s)."))
                } else {
                    let first = outcomes
                        .iter()
                        .find(|outcome| !outcome.removed)
                        .map(|outcome| outcome.summary())
                        .unwrap_or_default();
                    Status::Error(format!("Removed {removed}, {failed} failed. {first}"))
                };
                root.sync(cx);
                // The inventory on disk has changed, so the one on screen is stale
                // until it is asked again.
                root.scan(cx);
            },
        );
    }
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

    fn request_removal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.snapshot.plan.is_empty() {
            return;
        }
        self.snapshot.pending = Some(Pending::Removal);
        window.focus(&self.confirm_focus, cx);
        cx.notify();
    }

    fn request_script(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        // A preview changes nothing, so it does not need asking twice. A run that
        // would write does.
        if self.dry_run {
            self.run_script(id, cx);
            return;
        }
        if !self.scripts.iter().any(|script| script.id == id) {
            self.snapshot.status = Status::Error(format!("No script called {id}."));
            cx.notify();
            return;
        }
        self.snapshot.pending = Some(Pending::Script(id.to_string()));
        window.focus(&self.confirm_focus, cx);
        cx.notify();
    }

    fn confirm_pending(&mut self, cx: &mut Context<Self>) {
        match self.snapshot.pending.take() {
            Some(Pending::Removal) => self.run_removal(cx),
            Some(Pending::Script(id)) => self.run_script(&id, cx),
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

    fn run_script(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(script) = self
            .scripts
            .iter()
            .find(|script| script.id == id)
            .cloned()
        else {
            self.snapshot.status = Status::Error(format!("No script called {id}."));
            cx.notify();
            return;
        };

        let dry_run = self.dry_run;
        let title = script.title.clone();
        let verb = if dry_run { "Previewing" } else { "Running" };

        self.busy = true;
        self.snapshot.loading = Some(format!("{verb} {title}"));
        self.snapshot.status = Status::Activity(format!("{verb} {title}"));
        cx.notify();

        let runner = self.runner.clone();
        let finished_title = title.clone();
        self.run_off_thread(
            cx,
            move || script_runner::run_script(&script, &*runner, dry_run),
            move |root, result, cx| {
                root.busy = false;
                root.snapshot.loading = None;
                root.snapshot.status = match result {
                    Ok(output) if output.succeeded() => {
                        // The script's own last line is a better report than anything
                        // this app could invent: it knows what it did.
                        let last = output
                            .combined()
                            .lines()
                            .map(str::trim)
                            .rfind(|line| !line.is_empty())
                            .unwrap_or("finished")
                            .to_string();
                        Status::Info(format!("{finished_title}: {last}"))
                    }
                    Ok(output) => {
                        let first = output
                            .combined()
                            .lines()
                            .map(str::trim)
                            .find(|line| !line.is_empty())
                            .unwrap_or("no output")
                            .to_string();
                        Status::Error(format!("{finished_title}: {first}"))
                    }
                    Err(error) => Status::Error(error.to_string()),
                };
                root.sync(cx);
            },
        );
    }

    fn set_dry_run(&mut self, dry_run: bool, cx: &mut Context<Self>) {
        self.dry_run = dry_run;
        self.snapshot.dry_run = dry_run;
        self.snapshot.status = if dry_run {
            Status::Info("Scripts will preview with -DryRun. Nothing will change.".to_string())
        } else {
            Status::Caution(
                "Preview is off. Running a script will change this machine.".to_string(),
            )
        };
        cx.notify();
    }

    fn toggle_appearance(&mut self, cx: &mut Context<Self>) {
        self.theme.toggle_appearance();
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
        let Some(package) = self
            .staging
            .visible_package(self.snapshot.cursor)
            .cloned()
        else {
            return;
        };
        if self.staging.is_staged(&package.name) {
            self.unstage(&package.name, cx);
        } else {
            self.stage(&package.name, cx);
        }
    }
}
