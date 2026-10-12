//! Composition: the shell, and the three surfaces.
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

mod packages;
mod presets;
mod scripts;
mod shared;

pub use shared::{package_list, section_title};

use crate::components::button::{ButtonVariant, button};
use crate::components::{checkbox, stat};
use crate::icons::icon;
use crate::snapshot::{Screen, Snapshot};
use crate::theme::{Appearance, Theme, ThemeExt, layout, text};
use gpui::{
    AnyElement, ClickEvent, Context, FocusHandle, FontWeight, IntoElement, KeyDownEvent,
    WeakEntity, Window, div, prelude::*, px, rgba,
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
    /// The inventory list's focus handle, for arrow-key navigation.
    fn list_focus(&self) -> &FocusHandle;

    fn navigate(&mut self, screen: Screen, cx: &mut Context<Self>);
    fn set_query(&mut self, query: String, cx: &mut Context<Self>);
    fn set_show_system(&mut self, show: bool, cx: &mut Context<Self>);
    fn stage(&mut self, name: &str, cx: &mut Context<Self>);
    fn unstage(&mut self, name: &str, cx: &mut Context<Self>);
    fn stage_all(&mut self, cx: &mut Context<Self>);
    fn unstage_all(&mut self, cx: &mut Context<Self>);
    fn scan(&mut self, cx: &mut Context<Self>);
    fn run_removal(&mut self, cx: &mut Context<Self>);
    fn load_preset(&mut self, name: &str, cx: &mut Context<Self>);
    fn run_script(&mut self, id: &str, cx: &mut Context<Self>);
    fn set_dry_run(&mut self, dry_run: bool, cx: &mut Context<Self>);
    fn set_reduce_motion(&mut self, on: bool, cx: &mut Context<Self>);
    /// Switch between the light and dark token sets.
    fn toggle_appearance(&mut self, cx: &mut Context<Self>);
    fn dismiss_status(&mut self, cx: &mut Context<Self>);
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
    let entity: WeakEntity<T> = cx.entity().downgrade();

    let body = match snapshot.screen {
        Screen::Packages => packages::screen(
            &theme,
            &snapshot,
            &entity,
            &search_focus,
            &list_focus,
            cx,
        ),
        Screen::Scripts => scripts::screen(&theme, &snapshot, cx),
        Screen::Presets => presets::screen(&theme, &snapshot, cx),
    };

    div()
        .flex()
        .flex_col()
        .size_full()
        .bg(theme.surface)
        .font_family(theme.font_ui.clone())
        .text_color(theme.ink)
        .child(header(&theme, &snapshot, cx))
        .child(
            div()
                .flex()
                .flex_1()
                .min_h_0()
                .child(rail(&theme, &snapshot, cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .child(body),
                ),
        )
        .child(crate::components::status_bar::status_bar(
            &theme,
            &snapshot.status,
            snapshot.elevated,
            theme.reduce_motion,
        ))
        .into_any_element()
}

/// The chrome: a mark, the two counts that matter, and the two actions that apply to
/// the whole app.
fn header<T: DebrisApp>(theme: &Theme, snapshot: &Snapshot, cx: &mut Context<T>) -> AnyElement {
    let busy = snapshot.loading.is_some();
    let staged = snapshot.counts.staged;

    div()
        .flex()
        .items_center()
        .gap_6()
        .h(px(layout::HEADER))
        .px_4()
        .flex_shrink_0()
        .bg(theme.surface)
        .border_b_1()
        .border_color(theme.rule)
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(div().w(px(12.0)).h(px(12.0)).flex_shrink_0().bg(theme.ink))
                .child(
                    div()
                        .font_family(theme.font_mono.clone())
                        .text_size(px(text::BODY))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("debris"),
                ),
        )
        .child(stat(theme, snapshot.counts.installed.to_string(), "installed"))
        .child(stat(theme, staged.to_string(), "staged"))
        .child(div().flex_1())
        .child(button(
            theme,
            "rescan",
            if busy { "Scanning" } else { "Rescan" },
            ButtonVariant::Default,
            !busy,
            cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| this.scan(cx)),
        ))
        .child(button(
            theme,
            "unstage-all-header",
            "Clear staged",
            ButtonVariant::Quiet,
            staged > 0,
            cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| this.unstage_all(cx)),
        ))
        .into_any_element()
}

/// The rail: three surfaces, a hairline, and the two settings that change how the
/// rest of the interface behaves.
fn rail<T: DebrisApp>(theme: &Theme, snapshot: &Snapshot, cx: &mut Context<T>) -> AnyElement {
    let mut rail = div()
        .flex()
        .flex_col()
        .gap_1()
        .w(px(layout::RAIL))
        .py_4()
        .flex_shrink_0()
        .bg(theme.surface_sunken)
        .border_r_1()
        .border_color(theme.rule);

    for screen in Screen::ALL {
        let selected = screen == snapshot.screen;
        // On a sunken rail, the selected item is the *raised* one. Emphasis by
        // surface rather than by colour, so the accent stays reserved for removal.
        let background = if selected { theme.surface } else { rgba(0x00000000) };
        let foreground = if selected { theme.ink } else { theme.ink_muted };
        let marker = if selected { theme.ink } else { rgba(0x00000000) };
        let hover = theme.surface_hover;
        let font = theme.font_ui.clone();

        rail = rail.child(
            div()
                .id(screen.label().to_string())
                .flex()
                .items_center()
                .gap_3()
                .h(px(36.0))
                .pl_4()
                .pr_4()
                .bg(background)
                .text_color(foreground)
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(
                    cx.listener(
                        move |this: &mut T, _event: &ClickEvent, _window, cx| {
                            this.navigate(screen, cx)
                        },
                    ),
                )
                .child(div().w(px(2.0)).h(px(14.0)).flex_shrink_0().bg(marker))
                .child(icon(screen.icon(), 14.0))
                .child(
                    div()
                        .font_family(font)
                        .text_size(px(text::BODY))
                        .child(screen.label()),
                ),
        );
    }

    rail = rail.child(div().flex_1());
    rail = rail.child(
        div()
            .px_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(checkbox(
                theme,
                "reduce-motion",
                theme.reduce_motion,
                "Reduce motion",
                cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                    let next = !this.snapshot().reduce_motion;
                    this.set_reduce_motion(next, cx)
                }),
            ))
            .child(checkbox(
                theme,
                "preview-scripts",
                snapshot.dry_run,
                "Preview scripts",
                cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                    let next = !this.snapshot().dry_run;
                    this.set_dry_run(next, cx)
                }),
            ))
            .child(checkbox(
                theme,
                "dark-appearance",
                theme.appearance == Appearance::Dark,
                "Dark appearance",
                cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                    this.toggle_appearance(cx)
                }),
            )),
    );

    rail.into_any_element()
}

/// The two keyboard strokes the inventory listens for, shared by every list.
pub fn list_key_handler<T: DebrisApp>(
    cx: &mut Context<T>,
) -> impl Fn(&KeyDownEvent, &mut Window, &mut gpui::App) + 'static {
    cx.listener(|this: &mut T, event: &KeyDownEvent, _window, cx| {
        match event.keystroke.key.as_str() {
            "up" => this.move_cursor(-1, cx),
            "down" => this.move_cursor(1, cx),
            "enter" | "space" => this.activate_cursor(cx),
            _ => {}
        }
    })
}
