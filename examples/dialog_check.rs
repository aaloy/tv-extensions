// (C) 2026 - Antoni Aloy

//! Check a dialog JSON file: the errors that stop it loading, warnings about
//! its layout, what code must bind, the data it returns, and (with
//! `--render`) a text picture of it.
//!
//! ```sh
//! cargo run --example dialog_check --features native,dialog -- dialog.json --render
//! ```
//!
//! The exit status is 1 when the file is not a valid dialog, so a script, or
//! an agent writing the JSON, can loop: write, check, read the messages, fix.
//! See `docs/dialog-json.md` for the format.

use turbo_vision::app::Application;
use turbo_vision::core::event::{Event, KB_ESC};
use turbo_vision::core::geometry::Rect;
use turbo_vision::test_util::test_terminal;
use turbo_vision::views::button::set_press_animation;
use turbo_vision::views::tabbed_pane::TabbedPane;
use tv_extensions::dialog_spec::{Control, DialogSpec, JsonDialog, Kind};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let render = args.iter().any(|a| a == "--render");
    let Some(path) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("usage: dialog_check FILE.json [--render]");
        std::process::exit(2);
    };
    let spec = match DialogSpec::load(path) {
        Ok(spec) => spec,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "ok: \"{}\" {}x{}, {} controls",
        spec.title,
        spec.width,
        spec.height,
        count(&spec.controls)
    );

    let mut dialog = JsonDialog::new(&spec).expect("a checked spec builds");
    println!("\ndata returned when accepted (the starting values):");
    println!(
        "{}",
        serde_json::to_string_pretty(&dialog.data()).unwrap_or_default()
    );
    let unbound = dialog.unbound();
    if !unbound.is_empty() {
        println!("\nyour code binds: {}", unbound.join(", "));
    }

    let mut warnings = Vec::new();
    let (iw, ih) = spec.inner_size();
    lint(&spec.controls, (iw, ih), "the dialog", &mut warnings);
    if !spec
        .controls
        .iter()
        .chain(nested(&spec.controls).iter().copied())
        .any(|c| matches!(&c.kind, Kind::Button { command, .. } if command == "ok"))
    {
        warnings.push("no button has the command \"ok\": nothing accepts the dialog".into());
    }
    if warnings.is_empty() {
        println!("\nno warnings");
    } else {
        println!("\nwarnings:");
        for w in &warnings {
            println!("  - {w}");
        }
    }

    if render {
        println!("\n{}", picture(&spec));
    }
}

fn count(controls: &[Control]) -> usize {
    controls.len() + nested(controls).len()
}

/// Every control inside a container, at any depth.
fn nested(controls: &[Control]) -> Vec<&Control> {
    let mut out = Vec::new();
    for c in controls {
        for list in c.child_lists() {
            for inner in list {
                out.push(inner);
                out.extend(nested(std::slice::from_ref(inner)));
            }
        }
    }
    out
}

/// What `controls` get wrong about their places in a container `inner`
/// big, named `what`.
fn lint(controls: &[Control], inner: (i16, i16), what: &str, out: &mut Vec<String>) {
    for c in controls {
        let d = c.describe();
        if c.x < 0 || c.y < 0 || c.x + c.w > inner.0 || c.y + c.h > inner.1 {
            out.push(format!(
                "{d} at x={} y={} w={} h={} does not fit inside {what} ({}x{})",
                c.x, c.y, c.w, c.h, inner.0, inner.1
            ));
        }
        if c.kind.has_value() && c.name.is_empty() {
            out.push(format!("{d} has no name, so its value is not returned"));
        }
        if let Kind::Button { text, .. } = &c.kind {
            let shown = text.chars().filter(|&ch| ch != '~').count();
            if usize::try_from(c.w).unwrap_or(0) < shown + 4 {
                out.push(format!("{d} is too narrow: make w at least {}", shown + 4));
            }
            if c.h < 2 {
                out.push(format!("{d} needs h=2 (its shadow takes the second row)"));
            }
        }
        let area = match &c.kind {
            Kind::GroupBox { .. } => Some(((c.w - 2).max(0), (c.h - 2).max(0))),
            Kind::Tabs { .. } => {
                let a: Rect = TabbedPane::new(Rect::new(0, 0, c.w, c.h)).page_area();
                Some((a.width(), a.height()))
            }
            _ => None,
        };
        if let Some(area) = area {
            for (i, list) in c.child_lists().into_iter().enumerate() {
                let inside = match &c.kind {
                    Kind::Tabs { pages, .. } => format!("{d}, page \"{}\"", pages[i].title),
                    _ => d.clone(),
                };
                lint(list, area, &inside, out);
            }
        }
    }
    // Controls that sit on top of each other, apart from decoration and
    // containers (a frame legitimately surrounds what it holds).
    let plain = |c: &&Control| !c.is_container() && !matches!(c.kind, Kind::Line { .. });
    let flat: Vec<&Control> = controls.iter().filter(plain).collect();
    for (i, a) in flat.iter().enumerate() {
        for b in &flat[i + 1..] {
            if a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h {
                out.push(format!("{} and {} overlap", a.describe(), b.describe()));
            }
        }
    }
}

/// The dialog as the person would see it, drawn on a text screen.
fn picture(spec: &DialogSpec) -> String {
    set_press_animation(std::time::Duration::ZERO);
    let w = u16::try_from(spec.width.max(40) + 8).unwrap_or(120);
    let h = u16::try_from(spec.height.max(10) + 6).unwrap_or(40);
    let mut app = Application::with_terminal(test_terminal(w, h));
    let Ok(mut dialog) = JsonDialog::new(spec) else {
        return String::new();
    };
    // The dialog draws, takes the key, and closes.
    app.terminal
        .event_injector()
        .send(Event::keyboard(KB_ESC))
        .expect("the terminal takes events");
    let _ = dialog.run(&mut app);
    let (tw, th) = app.terminal.size();
    let mut rows = Vec::new();
    for y in 0..th {
        let row: String = (0..tw)
            .map(
                |x| match app.terminal.read_cell(x, y).map_or(' ', |c| c.ch) {
                    '░' => ' ', // the desktop behind the dialog
                    ch => ch,
                },
            )
            .collect();
        rows.push(row.trim_end().to_string());
    }
    while rows.last().is_some_and(String::is_empty) {
        rows.pop();
    }
    while rows.first().is_some_and(String::is_empty) {
        rows.remove(0);
    }
    rows.join("\n")
}
