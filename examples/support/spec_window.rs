//! A window that shows a form spec at the left of the desktop, for the
//! `form_json` examples to run the form beside it.
//!
//! The form is modal, so the window cannot scroll: the spec is laid out to
//! fit instead. An object or array stays on one line when it fits, and is
//! opened up when it does not; what still does not fit is cut with `…`.

// (C) 2026 - Antoni Aloy

use serde_json::Value;
use turbo_vision::app::Application;
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::GroupLike;
use turbo_vision::views::text_viewer::TextViewer;
use turbo_vision::views::window::WindowBuilder;

/// `value` as text lines no wider than `width` where it can be.
fn layout(value: &Value, indent: usize, width: usize, out: &mut Vec<String>, lead: &str) {
    let pad = " ".repeat(indent);
    let one = inline(value);
    if indent + lead.chars().count() + one.chars().count() <= width {
        out.push(format!("{pad}{lead}{one}"));
        return;
    }
    let (open, close, items): (_, _, Vec<(Option<&str>, &Value)>) = match value {
        Value::Object(map) => ('{', '}', map.iter().map(|(k, v)| (Some(k.as_str()), v)).collect()),
        Value::Array(list) => ('[', ']', list.iter().map(|v| (None, v)).collect()),
        _ => {
            out.push(format!("{pad}{lead}{one}"));
            return;
        }
    };
    out.push(format!("{pad}{lead}{open}"));
    for (i, (key, item)) in items.iter().enumerate() {
        let comma = if i + 1 < items.len() { "," } else { "" };
        let key = key.map(|k| format!("{}: ", Value::from(k))).unwrap_or_default();
        layout(item, indent + 2, width, out, &key);
        if let Some(last) = out.last_mut() {
            last.push_str(comma);
        }
    }
    out.push(format!("{pad}{close}"));
}

/// `value` on one line, with spaces after `:`, `,` and inside the braces.
fn inline(value: &Value) -> String {
    match value {
        Value::Object(map) if map.is_empty() => "{}".into(),
        Value::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", Value::from(k.as_str()), inline(v)))
                .collect();
            format!("{{ {} }}", items.join(", "))
        }
        Value::Array(list) => {
            let items: Vec<String> = list.iter().map(inline).collect();
            format!("[{}]", items.join(", "))
        }
        other => other.to_string(),
    }
}

fn clip(line: &str, width: usize) -> String {
    if line.chars().count() <= width {
        line.to_string()
    } else {
        let mut cut: String = line.chars().take(width.saturating_sub(1)).collect();
        cut.push('…');
        cut
    }
}

/// Put `spec` in a window on the left of the desktop, half the screen wide at
/// most.
///
/// # Errors
///
/// When the screen size does not fit the window's coordinates.
pub fn show(app: &mut Application, spec: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let (screen_w, screen_h) = app.terminal.size();
    let right = (screen_w / 2).clamp(24, 64);
    let bottom = screen_h - 2;
    let inner_w = usize::try_from(right - 2)?;
    let inner_h = usize::try_from(bottom - 2)?;

    let mut lines = Vec::new();
    layout(spec, 0, inner_w, &mut lines, "");
    if lines.len() > inner_h {
        lines.truncate(inner_h);
        lines[inner_h - 1] = "…".into();
    }
    let text: Vec<String> = lines.iter().map(|l| clip(l, inner_w)).collect();

    let bounds = Rect::new(0, 0, right, bottom);
    let mut window = WindowBuilder::new().bounds(bounds).title("Spec").build();
    let mut viewer = TextViewer::new(Rect::new(1, 1, right - 1, bottom - 1));
    viewer.set_text(&text.join("\n"));
    window.add(viewer);
    app.desktop.add(window);
    Ok(())
}
