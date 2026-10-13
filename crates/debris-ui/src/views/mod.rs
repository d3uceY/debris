//! Composition: the shell, and the three surfaces.
//!
//! # The shell
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────┐
//! │ 36  mark · breadcrumb      [ global search ]      appearance · panels ⋯  │
//! ├──────┬───────────────────────────┬──────────────────────────┬────────────┤
//! │ 48   │ 304                       │ center dock              │ 300        │
//! │ rail │ removal queue             │ header / toolbar / table │ inspector  │
//! │      │ (the ledger + confirm)    │                          │ (optional) │
//! ├──────┴───────────────────────────┴──────────────────────────┴────────────┤
//! │ 26  ● what just happened       │ elevation                              │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Three rules hold the shell together:
//!
//! 1. **Each zone owns its own scroll.** The rail, the queue, the table, and the
//!    inspector are four independent scroll regions; the shell itself never scrolls.
//! 2. **Bands align.** The queue's header and the center's panel header are the same
//!    34px, on the same hairline. That alignment is what makes four regions read as
//!    one window.
//! 3. **One search field.** The filter lives in the title bar and applies to
//!    whichever surface is up, so a screen never has its own private copy of it.
//!
//! # How this stays decoupled
//!
//! The views need two things: state to draw, and a way to report that something was
//! clicked. State arrives as a [`Snapshot`] — plain data — and clicks arrive as calls
//! on [`DebrisApp`], a trait the binary implements. So this crate can lay out every
//! surface, in every state, without a PowerShell process anywhere near it.
//!
//! Handlers are built with `cx.listener`, which hands the closure `&mut T`; the views
//! never hold a reference to the app, and the app never knows what a `div` is.

mod confirm;
mod inspector;
mod ledger;
mod packages;
mod presets;
mod scripts;
pub mod shared;

pub use shared::package_table;

use crate::components::button::button;
use crate::components::icon_rail::{rail_item, rail_shell, rail_spacer};
use crate::components::text_field::text_field;
use crate::components::{divider, status_bar};
use crate::icons::Icon;
use crate::snapshot::{Screen, Snapshot};
use crate::theme::{Appearance as ThemeAppearance, Theme, ThemeExt, metrics, space, text};
use gpui::{
    AnyElement, ClickEvent, Context, FocusHandle, FontWeight, IntoElement, SharedString, WeakEntity,
    Window, div, prelude::*, px,
};

/// What a debris surface can do.
///
/// Deliberately a list of events rather than a state machine: the views say *what
/// happened*, and the app decides what that means. Adding a surface means adding an
/// event, not reaching into the app.
pub trait DebrisApp: Sized + 'static {
    fn snapshot(&self) -> &Snapshot;

    /// The search field's focus handle, owned by the app so focus survives a render.
    fn search_focus(&self) -> &FocusHandle;
    /// The inventory's focus handle, for arrow-key navigation.
    fn list_focus(&self) -> &FocusHandle;
    /// The confirmation dialog's focus handle, so Enter and Escape reach it.
    fn confirm_focus(&self) -> &FocusHandle;

    fn navigate(&mut self, screen: Screen, cx: &mut Context<Self>);
    fn set_query(&mut self, query: String, cx: &mut Context<Self>);
    fn set_show_system(&mut self, show: bool, cx: &mut Context<Self>);
    fn stage(&mut self, name: &str, cx: &mut Context<Self>);
    fn unstage(&mut self, name: &str, cx: &mut Context<Self>);
    fn stage_all(&mut self, cx: &mut Context<Self>);
    fn unstage_all(&mut self, cx: &mut Context<Self>);
    fn scan(&mut self, cx: &mut Context<Self>);
    /// Run one script, now. Views go through [`DebrisApp::request_script`]; this is the
    /// executor the confirmation calls, so it is not the event a control binds to.
    fn run_script(&mut self, id: &str, cx: &mut Context<Self>);
    /// Ask to remove the staged packages. Opens the confirmation; it removes nothing.
    fn request_removal(&mut self, window: &mut Window, cx: &mut Context<Self>);
    /// Ask to run one script. A preview runs at once; a run that would change the
    /// machine opens the confirmation first.
    fn request_script(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>);
    /// Do the thing the confirmation is asking about.
    fn confirm_pending(&mut self, cx: &mut Context<Self>);
    /// Dismiss the confirmation without doing anything.
    fn cancel_pending(&mut self, cx: &mut Context<Self>);
    fn load_preset(&mut self, name: &str, cx: &mut Context<Self>);
    fn set_dry_run(&mut self, dry_run: bool, cx: &mut Context<Self>);
    /// Switch between the light and dark token sets.
    fn toggle_appearance(&mut self, cx: &mut Context<Self>);
    /// Show or hide the left column.
    fn toggle_sidebar(&mut self, cx: &mut Context<Self>);
    /// Show or hide the right column.
    fn toggle_inspector(&mut self, cx: &mut Context<Self>);
    /// Move the keyboard cursor through the inventory.
    fn move_cursor(&mut self, delta: i32, cx: &mut Context<Self>);
    /// Act on the row under the cursor.
    fn activate_cursor(&mut self, cx: &mut Context<Self>);
}

/// The whole window.
///
/// Everything the tree needs is cloned out of the app *before* a single element is
/// built, so no element ever holds a borrow of the app and no listener can conflict
/// with the tree being assembled.
pub fn root<T: DebrisApp>(this: &mut T, cx: &mut Context<T>) -> AnyElement {
    let theme = cx.theme().clone();
    let snapshot = this.snapshot().clone();
    let search_focus = this.search_focus().clone();
    let list_focus = this.list_focus().clone();
    let confirm_focus = this.confirm_focus().clone();
    let entity: WeakEntity<T> = cx.entity().downgrade();

    let center: AnyElement = match snapshot.screen {
        Screen::Packages => packages::screen(&theme, &snapshot, &entity, &list_focus, cx),
        Screen::Scripts => scripts::screen(&theme, &snapshot, cx),
        Screen::Presets => presets::screen(&theme, &snapshot, cx),
    };

    // Built ahead of the tree rather than inside a `when` closure: each of these
    // needs `cx` mutably for its listeners, and two closures capturing `cx` in one
    // expression cannot overlap.
    let queue = snapshot
        .sidebar_open
        .then(|| ledger::ledger(&theme, &snapshot, &entity, cx));
    let inspector = snapshot
        .inspector_open
        .then(|| inspector::inspector(&theme, &snapshot, cx));
    let confirm = snapshot.pending.as_ref().map(|pending| {
        confirm::confirm_overlay(&theme, &snapshot, pending, &confirm_focus, cx)
    });

    div()
        .relative()
        .flex()
        .flex_col()
        .size_full()
        .bg(theme.background)
        .font_family(theme.font_ui.clone())
        .text_size(px(text::BODY))
        .text_color(theme.foreground)
        .child(title_bar(&theme, &snapshot, &search_focus, cx))
        .child(
            div()
                .flex()
                .flex_row()
                .flex_1()
                .min_h_0()
                .w_full()
                .child(rail(&theme, &snapshot, cx))
                .when_some(queue, |el, queue| el.child(queue))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                        .child(center),
                )
                .when_some(inspector, |el, inspector| el.child(inspector)),
        )
        .child(status_bar::status_bar(
            &theme,
            &snapshot.status,
            snapshot.elevated,
            snapshot.is_busy(),
        ))
        // The dialog paints last, so it sits over the whole shell and over the scroll
        // regions inside it. `absolute` keeps it out of the shell's flex flow.
        .when_some(confirm, |el, confirm| el.child(confirm))
        .into_any_element()
}

/// Which mark and tooltip the appearance toggle shows, given the current tokens.
fn appearance_toggle(theme: &Theme) -> (Icon, SharedString) {
    if theme.appearance == ThemeAppearance::Dark {
        (Icon::Sun, "Switch to light appearance".into())
    } else {
        (Icon::Moon, "Switch to dark appearance".into())
    }
}

/// The 36px band at the top: where you are, one search field, and the actions that
/// apply to the whole app.
///
/// The two outer regions are both `flex_1`, which is what keeps the search field
/// optically centred instead of merely *between* its neighbours.
fn title_bar<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    search_focus: &FocusHandle,
    cx: &mut Context<T>,
) -> AnyElement {
    let (appearance_mark, appearance_tip) = appearance_toggle(theme);

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::SM))
        .h(px(metrics::TITLE_BAR))
        .w_full()
        .flex_shrink_0()
        .px(px(space::SM))
        .bg(theme.title_bar)
        .border_b_1()
        .border_color(theme.border)
        // — left: identity and location —
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(space::SM))
                .flex_1()
                .min_w_0()
                .child(brand_mark(theme))
                .child(
                    div()
                        .font_family(theme.font_mono.clone())
                        .text_size(px(text::SMALL))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child("debris"),
                )
                .child(divider(theme))
                .child(
                    div()
                        .truncate()
                        .font_family(theme.font_ui.clone())
                        .text_size(px(text::SMALL))
                        .text_color(theme.muted_foreground)
                        .child(snapshot.screen.label()),
                ),
        )
        // — center: the one search field —
        .child(
            div()
                .w(px(metrics::COMMAND_SHELL))
                .flex_shrink_0()
                .child(text_field(
                    theme,
                    "global-filter",
                    &snapshot.query,
                    snapshot.screen.hint(),
                    search_focus,
                    shared::search_key_handler(cx),
                    cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                        this.set_query(String::new(), cx)
                    }),
                )),
        )
        // — right: the actions that outlive a surface —
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_end()
                .gap(px(space::XS))
                .flex_1()
                .min_w_0()
                .child(
                    button(theme, "toggle-appearance")
                        .ghost()
                        .small()
                        .icon(appearance_mark)
                        .icon_only()
                        .tooltip(appearance_tip)
                        .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                            this.toggle_appearance(cx)
                        })),
                )
                .child(divider(theme))
                .child(
                    button(theme, "toggle-queue")
                        .ghost()
                        .small()
                        .icon(Icon::PanelLeft)
                        .icon_only()
                        .tooltip("Show or hide the removal queue")
                        .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                            this.toggle_sidebar(cx)
                        })),
                )
                .child(
                    button(theme, "toggle-inspector")
                        .ghost()
                        .small()
                        .icon(Icon::PanelRight)
                        .icon_only()
                        .tooltip("Show or hide the inspector")
                        .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                            this.toggle_inspector(cx)
                        })),
                )
                .child(divider(theme))
                .child(
                    button(theme, "rescan")
                        .outline()
                        .label(if snapshot.is_busy() {
                            "Scanning"
                        } else {
                            "Rescan"
                        })
                        .icon(Icon::Refresh)
                        .small()
                        .disabled(snapshot.is_busy())
                        .tooltip("Ask Windows what is installed for this user")
                        .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                            this.scan(cx)
                        })),
                ),
        )
        .into_any_element()
}

/// The mark: a square with a square knocked out of it. Two `div`s, no asset, and it
/// reads as a monogram rather than a placeholder.
fn brand_mark(theme: &Theme) -> AnyElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(16.0))
        .h(px(16.0))
        .flex_shrink_0()
        .rounded(px(4.0))
        .bg(theme.foreground)
        .child(div().w(px(6.0)).h(px(6.0)).rounded(px(1.5)).bg(theme.title_bar))
        .into_any_element()
}

/// The 48px vertical switcher.
fn rail<T: DebrisApp>(theme: &Theme, snapshot: &Snapshot, cx: &mut Context<T>) -> AnyElement {
    let mut rail = rail_shell(theme).child(div().h(px(space::SM)));
    for screen in Screen::ALL {
        let active = screen == snapshot.screen;
        rail = rail.child(rail_item(
            theme,
            screen.label(),
            screen.icon(),
            screen.label(),
            active,
            cx.listener(move |this: &mut T, _event: &ClickEvent, _window, cx| {
                this.navigate(screen, cx)
            }),
        ));
    }

    let (appearance_mark, appearance_tip) = appearance_toggle(theme);

    rail
        .child(rail_spacer())
        .child(
            // The inspector toggle lives on the rail as well as in the title bar: the
            // rail is where the panes are, and the title bar is where the window is.
            rail_item(
                theme,
                "rail-inspector",
                Icon::PanelRight,
                "Show or hide the inspector",
                snapshot.inspector_open,
                cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                    this.toggle_inspector(cx)
                }),
            ),
        )
        .child(rail_item(
            theme,
            "rail-appearance",
            appearance_mark,
            appearance_tip,
            false,
            cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                this.toggle_appearance(cx)
            }),
        ))
        .into_any_element()
}
