//! Scroll and Popup Example
//! Demonstrates core turbo-vision's `TabbedPane` (tabbed pages, one `Group`
//! per page) alongside tv-extensions' `ScrollPane` and `popup_menu`:
//! - `TabbedPane`: tabbed pages (click a tab or Ctrl+PgUp / Ctrl+PgDn)
//! - `ScrollPane`: a form taller than its window (wheel / Ctrl+Up / Ctrl+Down)
//! - `popup_menu()`: a context menu on F9 with a check-mark item
//!
//! Run with:
//!   `cargo run --example scroll_and_popup --features native`
//!
//! The example uses a hand-rolled event loop (`get_event`/`handle_event`) so
//! it can open the popup menu, which needs terminal access — the same
//! pattern an application would use for right-click context menus.

// (C) 2026 - Enzo Lombardi

use turbo_vision::app::Application;
use turbo_vision::core::command::CM_QUIT;
use turbo_vision::core::event::{EventType, KB_ESC_ESC, KB_F9};
use turbo_vision::core::geometry::{Point, Rect};
use turbo_vision::core::menu_data::MenuItemBuilder;
use turbo_vision::core::menu_data::{Menu, MenuItem};
use turbo_vision::core::status_data::StatusItemBuilder;
use turbo_vision::views::group::Group;
use turbo_vision::views::static_text::StaticText;
use turbo_vision::views::status_line::StatusLine;
use turbo_vision::views::window::{Window, WindowBuilder};
use turbo_vision::views::{ComboBox, GroupLike, Spinner, TabbedPane};
use tv_extensions::{ScrollPane, is_menu_item_checked, popup_menu, set_menu_item_checked};

const COMBO_LANG: u16 = 1;
const CM_POPUP: u16 = 1000;
const CM_TOGGLE_WRAP: u16 = 1010;
const CM_SAY_HELLO: u16 = 1011;

/// `TabbedPane` window: three pages (General/Advanced/About), one `Group` each.
fn build_tabbed_pane_window() -> Window {
    let mut nb_window = WindowBuilder::new()
        .bounds(Rect::new(3, 2, 50, 16))
        .title("Tabbed Pane")
        .build();

    let mut pane = TabbedPane::new(Rect::new(1, 1, 44, 12));

    let mut general = Group::new(pane.page_area());
    general.add(StaticText::new(Rect::new(1, 1, 40, 2), "Language:"));
    general.add(ComboBox::with_items(
        Rect::new(12, 1, 32, 2),
        COMBO_LANG,
        vec!["Rust".into(), "Pascal".into(), "C++".into()],
    ));
    general.set_initial_focus();
    pane.add_page("~G~eneral", general);

    let mut advanced = Group::new(pane.page_area());
    advanced.add(StaticText::new(Rect::new(1, 1, 40, 2), "Workers:"));
    let mut workers = Spinner::new(Rect::new(12, 1, 22, 2), 1, 64);
    workers.set_value(8);
    advanced.add(workers);
    advanced.set_initial_focus();
    pane.add_page("~A~dvanced", advanced);

    let mut about = Group::new(pane.page_area());
    about.add(StaticText::new(
        Rect::new(1, 1, 42, 3),
        "Tabbed pages, one Group per page.\nCtrl+PgUp / Ctrl+PgDn switch tabs.",
    ));
    pane.add_page("~A~bout", about);

    pane.set_initial_focus();
    nb_window.add(pane);
    nb_window
}

/// `ScrollPane` window: a 30-row virtual form inside an ~11-row viewport.
fn build_scroll_pane_window() -> Window {
    let mut sp_window = WindowBuilder::new()
        .bounds(Rect::new(52, 3, 88, 17))
        .title("Tall Form")
        .build();

    let mut scroll_pane = ScrollPane::new(Rect::new(1, 1, 33, 12), 30);
    for i in 0..10 {
        let y = i * 3;
        scroll_pane.add(
            Box::new(StaticText::new(
                Rect::new(1, y, 30, y + 1),
                &format!("Field {} — scroll with the wheel", i + 1),
            )),
            Rect::new(1, y, 30, y + 1),
        );
    }
    sp_window.add(scroll_pane);
    sp_window
}

/// The context menu, with its "Word wrap" check mark set.
fn build_context_menu() -> Menu {
    let mut menu = Menu::from_items(vec![
        MenuItemBuilder::new()
            .text("Word wrap")
            .command(CM_TOGGLE_WRAP)
            .build(),
        MenuItem::separator(),
        MenuItemBuilder::new()
            .text("Say hello")
            .command(CM_SAY_HELLO)
            .build(),
        MenuItemBuilder::new()
            .text("E~x~it")
            .command(CM_QUIT)
            .build(),
    ]);
    set_menu_item_checked(&mut menu, 0, true);
    menu
}

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;

    let (width, height) = app.terminal.size();
    app.set_status_line(StatusLine::new(
        Rect::new(0, height - 1, width, height),
        vec![
            StatusItemBuilder::new()
                .text("~F9~ Menu")
                .key("F9")
                .command(CM_POPUP)
                .build(),
            StatusItemBuilder::new()
                .text("~Alt-X~ Exit")
                .key("Alt+X")
                .command(CM_QUIT)
                .build(),
            StatusItemBuilder::new()
                .text("~Esc-Esc~ Exit")
                .key_code(KB_ESC_ESC)
                .command(CM_QUIT)
                .build(),
        ],
    ));

    app.desktop.add(build_tabbed_pane_window());
    app.desktop.add(build_scroll_pane_window());

    // The menu keeps state across openings so the check mark persists
    let mut context_menu = build_context_menu();

    while app.running {
        let Some(mut event) = app.get_event() else {
            continue;
        };

        // Let the app dispatch first: the status line converts F9 into
        // CM_POPUP, and unhandled commands survive dispatch
        if !(event.what == EventType::Keyboard && event.key_code == KB_F9) {
            app.handle_event(&mut event);
        }

        // F9 (raw key or the status line's command) opens the popup
        let popup_requested = (event.what == EventType::Keyboard && event.key_code == KB_F9)
            || (event.what == EventType::Command && event.command == CM_POPUP);
        if popup_requested {
            match popup_menu(&mut app.terminal, Point::new(10, 5), context_menu.clone()) {
                Some(CM_TOGGLE_WRAP) => {
                    let now = !is_menu_item_checked(&context_menu, 0);
                    set_menu_item_checked(&mut context_menu, 0, now);
                }
                Some(CM_QUIT) => app.running = false,
                Some(_) | None => {}
            }
        }
    }

    Ok(())
}
