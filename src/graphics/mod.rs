// (C) 2026 - Enzo Lombardi

//! Pictures in a text UI: ANSI-art backgrounds ([`AnsiBackground`], parsed by
//! [`ansi`]) and bitmap images over the Kitty graphics protocol
//! ([`KittyImage`], with the protocol helpers in [`kitty`]).

pub mod ansi;
mod ansi_background;
pub mod kitty;
mod kitty_image;

pub use ansi_background::{AnsiBackground, AnsiBackgroundBuilder};
pub use kitty_image::{KittyImage, KittyImageBuilder};
