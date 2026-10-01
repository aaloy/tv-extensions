//! Controls Example
//! Demonstrates core turbo-vision value controls: `ComboBox`, `Spinner`,
//! `Slider`, and `ProgressBar` (standing in for the old turbo-vision-extras
//! `Gauge`).
//!
//! Run with:
//!   cargo run --example controls --features native
//!
//! Tab moves between controls. The combo box opens with F4, Down, or a
//! click; the spinner reacts to Up/Down/PgUp/PgDn and its steppers; the
//! slider follows Left/Right/Home/End and mouse clicks on the track.

// (C) 2026 - Enzo Lombardi

use turbo_vision::app::Application;
use turbo_vision::core::command::CM_QUIT;
use turbo_vision::core::event::KB_ESC_ESC;
use turbo_vision::core::geometry::Rect;
use turbo_vision::core::status_data::StatusItemBuilder;
use turbo_vision::views::static_text::StaticText;
use turbo_vision::views::status_line::StatusLine;
use turbo_vision::views::window::{Window, WindowBuilder};
use turbo_vision::views::{ComboBox, GroupLike, ProgressBar, Slider, Spinner};

const COMBO_THEME: u16 = 1;

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
        .bounds(Rect::new(10, 3, 66, 18))
        .title("Controls")
        .build();

    // Spinner — numeric value with steppers
    window.add(StaticText::new(Rect::new(2, 5, 16, 6), "Tab size:"));
    let mut tab_size = Spinner::new(Rect::new(16, 5, 26, 6), 1, 16);
    tab_size.set_value(4);
    let tab_size_id = window.add(tab_size);

    // Slider
    window.add(StaticText::new(Rect::new(2, 8, 16, 9), "Volume:"));
    let mut slider = Slider::new(Rect::new(16, 8, 50, 9), 0, 100);
    slider.set_value(65);
    slider.set_step(5);
    window.add(slider);

    // ProgressBar — stands in for the old Gauge
    window.add(StaticText::new(Rect::new(2, 11, 16, 12), "Progress:"));
    let mut progress = ProgressBar::new(Rect::new(16, 11, 50, 12), 100);
    progress.set_value(65);
    window.add(progress);

    // ComboBox — added LAST so its drop-down draws on top of the controls
    // below it (children paint in add order)
    window.add(StaticText::new(Rect::new(2, 2, 16, 3), "Theme:"));
    let theme_combo = ComboBox::with_items(
        Rect::new(16, 2, 42, 3),
        COMBO_THEME,
        vec![
            "Classic Blue".into(),
            "Monochrome".into(),
            "Solarized".into(),
            "High Contrast".into(),
        ],
    );
    let theme = theme_combo.state();
    window.add(theme_combo);

    window.set_initial_focus();
    let window_id = app.desktop.add(window);

    app.run();

    // Shared values are read back after the app exits
    let tab_size_value = app
        .desktop
        .child_by_id_mut(window_id)
        .and_then(|w| w.as_any_mut().downcast_mut::<Window>())
        .and_then(|w| w.child_by_id_mut(tab_size_id))
        .and_then(|v| v.as_any_mut().downcast_mut::<Spinner>())
        .map_or(0, |s| s.value());
    println!(
        "Theme:    {}",
        theme.borrow().selected_text().unwrap_or("<none>")
    );
    println!("Tab size: {tab_size_value}");

    Ok(())
}
