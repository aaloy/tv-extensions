//! Dual List Example
//! Demonstrates tv-extensions' `DualList`: pick a subset of items by moving
//! them between two lists, as in Django admin's `filter_horizontal`.
//!
//! The dialog edits a pizza's toppings. Each topping has a numeric id, the
//! way a database row would, and the list hands back the chosen ids. Two
//! start out chosen. OK needs between one and five toppings; otherwise the
//! header over the chosen list turns into an error line and the dialog
//! stays open.
//!
//! Run with:
//!   `cargo run --example dual_list --features native`
//!
//! Type in a filter field to narrow the list under it, Down to move into
//! the list. Alt+T and Alt+P jump to the two filter fields. Space marks items, Enter or a double-click moves the marked
//! ones (or the focused one) across. The `>` `>>` `<` `<<` buttons move the
//! marked items or everything the filter shows. Tab steps through the
//! parts and on to the buttons. The chosen toppings are printed when the
//! dialog closes.

// (C) 2026 - Antoni Aloy

use std::cell::RefCell;
use std::rc::Rc;

use turbo_vision::app::Application;
use turbo_vision::core::command::{CM_CANCEL, CM_OK};
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::GroupLike;
use turbo_vision::views::button::ButtonBuilder;
use turbo_vision::views::dialog::DialogBuilder;
use tv_extensions::DualListBuilder;

const TOPPINGS: [&str; 24] = [
    "Anchovies",
    "Artichoke",
    "Bacon",
    "Basil",
    "Black olives",
    "Capers",
    "Cherry tomatoes",
    "Chorizo",
    "Corn",
    "Garlic",
    "Goat cheese",
    "Gorgonzola",
    "Ham",
    "Jalapeños",
    "Mozzarella",
    "Mushrooms",
    "Onion",
    "Oregano",
    "Parmesan",
    "Pepperoni",
    "Pineapple",
    "Rocket",
    "Sobrassada",
    "Tuna",
];

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;

    // Ids as a database would hand them out: 101, 102, ...
    let toppings: Vec<(u32, &str)> = (101..).zip(TOPPINGS).collect();
    // Mozzarella and Basil are on the pizza already
    let chosen = Rc::new(RefCell::new(vec![115, 104]));

    let (width, height) = app.terminal.size();
    let (w, h) = (66, 21);
    let x = (width - w).max(0) / 2;
    let y = (height - h).max(0) / 2;
    let mut dialog = DialogBuilder::new()
        .bounds(Rect::new(x, y, x + w, y + h))
        .title("Pizza toppings")
        .build();

    dialog.add(
        DualListBuilder::new()
            .bounds(Rect::new(2, 1, 62, 16))
            .items(toppings.iter().copied())
            .data(chosen.clone())
            .titles("~T~oppings", "On the ~p~izza")
            .min_chosen(1)
            .max_chosen(5)
            .min_message("Choose a topping")
            .max_message("Five toppings at most")
            .build(),
    );

    dialog.add(
        ButtonBuilder::new()
            .bounds(Rect::new(19, 17, 31, 19))
            .title("~O~K")
            .command(CM_OK)
            .default(true)
            .build(),
    );
    dialog.add(
        ButtonBuilder::new()
            .bounds(Rect::new(35, 17, 47, 19))
            .title("Cancel")
            .command(CM_CANCEL)
            .build(),
    );

    dialog.set_initial_focus();
    let result = dialog.execute(&mut app);
    drop(app); // restore the terminal before printing

    if result == CM_OK {
        println!("OK");
        for id in chosen.borrow().iter() {
            let name = toppings.iter().find(|(k, _)| k == id).map_or("?", |t| t.1);
            println!("{id} {name}");
        }
    } else {
        println!("Canceled");
    }

    Ok(())
}
