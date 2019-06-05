#![deny(clippy::all)]

#[macro_use]
extern crate enclose;

pub mod edit_view;
pub mod fonts;
pub mod main_state;
pub mod theme;
mod view_item;

pub use crate::edit_view::EditView;
pub use crate::main_state::{MainState, Settings};
