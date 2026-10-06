//! A context menu: a menu box that opens where you say, runs until the user
//! picks an item or presses Esc, and returns the item's command. It is the
//! framework's MenuBox, so keys, hot keys and drawing match the drop-down
//! menus. Items can carry a check mark. Press the button, then pick Word
//! wrap twice to see its mark come and go.
//!
//! Parameters:
//! - `popup_menu(&mut app.terminal, point, menu)`: `point` is the menu's
//!   top-left corner on the screen (where the mouse was, for a right
//!   click); it returns `Some(command)`, or `None` when dismissed.
//! - `set_menu_item_checked(&mut menu, index, on)`: put or take off the
//!   check mark on item `index`; unchecked items are indented to line up.
//! - `is_menu_item_checked(&menu, index)`: whether it has the mark.
//!
//! See also: DualList, Key names

use crate::panel::Panel;
use std::cell::RefCell;
use turbo_vision::app::Application;
use turbo_vision::core::command::{CM_USER, CommandId};
use turbo_vision::core::geometry::{Point, Rect};
use turbo_vision::core::menu_data::{Menu, MenuItem, MenuItemBuilder};
use turbo_vision::views::button::Button;
use turbo_vision::views::msgbox::message_box_ok;
use tv_extensions::popup_menu::{is_menu_item_checked, popup_menu, set_menu_item_checked};

const OPEN: CommandId = CM_USER + 100;
const WRAP: CommandId = CM_USER + 101;
const COPY: CommandId = CM_USER + 102;
const PASTE: CommandId = CM_USER + 103;

thread_local! {
    /// The menu lives between openings, so the check mark stays.
    static MENU: RefCell<Menu> = RefCell::new(menu());
}

fn menu() -> Menu {
    let mut menu = Menu::from_items(vec![
        MenuItemBuilder::new()
            .text("Word wrap")
            .command(WRAP)
            .build(),
        MenuItem::separator(),
        MenuItemBuilder::new().text("~C~opy").command(COPY).build(),
        MenuItemBuilder::new()
            .text("~P~aste")
            .command(PASTE)
            .build(),
    ]);
    set_menu_item_checked(&mut menu, 0, true);
    menu
}

pub fn build(panel: &mut Panel) {
    panel.add(Button::new(
        Rect::new(0, 0, 20, 2),
        "~O~pen the menu",
        OPEN,
        true,
    ));
}

pub fn handle(app: &mut Application, command: CommandId) -> bool {
    if command != OPEN {
        return false;
    }
    let menu = MENU.with_borrow(Clone::clone);
    match popup_menu(&mut app.terminal, Point::new(30, 4), menu) {
        Some(WRAP) => MENU.with_borrow_mut(|menu| {
            let on = !is_menu_item_checked(menu, 0);
            set_menu_item_checked(menu, 0, on);
        }),
        Some(COPY) => {
            message_box_ok(app, "Copy was picked.");
        }
        Some(PASTE) => {
            message_box_ok(app, "Paste was picked.");
        }
        _ => {}
    }
    true
}
