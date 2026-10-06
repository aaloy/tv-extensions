// (C) 2026 - Antoni Aloy
//! Where a demo builds its views.
//!
//! A demo's views go straight into the gallery's dialog, so they take the
//! focus and answer keys like any dialog's controls. A nested `Group` would
//! not: a plain group never takes the focus. `Panel` keeps the demo's coordinates simple:
//! `(0, 0)` is the corner of the "Try it" box, as for a group's children.

use turbo_vision::core::geometry::{Point, Rect};
use turbo_vision::views::dialog::Dialog;
use turbo_vision::views::{GroupLike, View, ViewId};

/// The area of the gallery's dialog that a demo builds in.
pub struct Panel<'a> {
    dialog: &'a mut Dialog,
    origin: Point,
}

impl<'a> Panel<'a> {
    /// The area of `dialog` whose top-left corner is `origin`.
    pub fn new(dialog: &'a mut Dialog, origin: Point) -> Self {
        Self { dialog, origin }
    }

    /// Add `view`, placed relative to the panel's corner, and return its id
    /// (to link a label to it, for instance).
    pub fn add<V: View + 'static>(&mut self, view: V) -> ViewId {
        self.dialog.add(self.placed(view))
    }

    /// `view`, moved from the panel's coordinates to the dialog's.
    fn placed<V: View>(&self, mut view: V) -> V {
        let b = view.bounds();
        let (x, y) = (self.origin.x, self.origin.y);
        view.set_bounds(Rect::new(b.a.x + x, b.a.y + y, b.b.x + x, b.b.y + y));
        view
    }
}
