//! Autocomplete Example
//! Demonstrates tv-extensions' `AutoComplete`: text fields that filter a
//! suggestion list as you type, with the matched text highlighted.
//!
//! Run with:
//!   cargo run --example autocomplete --features native
//!
//! Type to filter (accented Latin-1 letters work too); Up/Down move the
//! highlight, Enter or a click accepts a suggestion, Esc closes the list.
//! Tab moves between the fields.
//!
//! The two fields show the two modes. Country requires a match: leave it
//! holding text that is not a country and it goes back to the last one
//! chosen (typing "méxico" becomes "México"). Fruit takes free text: the
//! list only suggests.

// (C) 2026 - Antoni Aloy

use std::cell::RefCell;
use std::rc::Rc;

use turbo_vision::app::Application;
use turbo_vision::core::command::CM_QUIT;
use turbo_vision::core::event::KB_ESC_ESC;
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::status_data::StatusItemBuilder;
use turbo_vision::views::GroupLike;
use turbo_vision::views::static_text::StaticText;
use turbo_vision::views::status_line::StatusLine;
use turbo_vision::views::window::WindowBuilder;
use tv_extensions::{AutoComplete, AutoCompleteBuilder};

fn countries() -> Vec<String> {
    [
        "Argentina",
        "Australia",
        "Austria",
        "Belgium",
        "Brazil",
        "Canada",
        "Chile",
        "China",
        "Denmark",
        "España",
        "Finland",
        "France",
        "Germany",
        "Greece",
        "Iceland",
        "India",
        "Indonesia",
        "Ireland",
        "Italy",
        "Japan",
        "Kenya",
        "México",
        "Netherlands",
        "New Zealand",
        "Norway",
        "Perú",
        "Poland",
        "Portugal",
        "Sweden",
        "Switzerland",
        "Thailand",
        "Türkiye",
        "United Kingdom",
        "United States",
        "Uruguay",
        "Vietnam",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn fruits() -> Vec<String> {
    [
        "Apple",
        "Apricot",
        "Banana",
        "Blackberry",
        "Blueberry",
        "Cherry",
        "Clementine",
        "Cranberry",
        "Date",
        "Fig",
        "Grape",
        "Grapefruit",
        "Guava",
        "Kiwi",
        "Lemon",
        "Lime",
        "Mango",
        "Melon",
        "Nectarine",
        "Orange",
        "Papaya",
        "Peach",
        "Pear",
        "Pineapple",
        "Plum",
        "Pomegranate",
        "Raspberry",
        "Strawberry",
        "Tangerine",
        "Watermelon",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;

    let (width, height) = app.terminal.size();
    app.set_status_line(StatusLine::new(
        Rect::new(0, height - 1, width, height),
        vec![
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

    let mut window = WindowBuilder::new()
        .bounds(Rect::new(10, 2, 66, 22))
        .title("AutoComplete")
        .build();

    // An open list grows its field's bounds downward (up to six rows here),
    // so each field leaves that much room before the next one.
    window.add(StaticText::new(Rect::new(2, 2, 16, 3), "Country:"));
    let country = Rc::new(RefCell::new(String::new()));
    let mut country_field =
        AutoComplete::new(Rect::new(16, 2, 46, 3), countries(), country.clone());
    country_field.set_require_match(true);
    window.add(country_field);
    window.add(StaticText::new(Rect::new(47, 2, 54, 3), "(list)"));

    window.add(StaticText::new(Rect::new(2, 10, 16, 11), "Fruit:"));
    window.add(StaticText::new(Rect::new(47, 10, 54, 11), "(free)"));
    let fruit = Rc::new(RefCell::new(String::new()));
    window.add(
        AutoCompleteBuilder::new()
            .bounds(Rect::new(16, 10, 46, 11))
            .items(fruits())
            .data(fruit.clone())
            .max_drop_rows(5)
            .build(),
    );

    window.set_initial_focus();
    app.desktop.add(window);

    app.run();

    // Shared values are read back after the app exits
    println!("Country: {}", country.borrow());
    println!("Fruit:   {}", fruit.borrow());

    Ok(())
}
