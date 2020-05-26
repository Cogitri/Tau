// Copyright (C) 2017-2018 Brian Vincent <brainn@gmail.com>
// Copyright (C) 2019-2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: GPL-3.0-or-later

#![deny(clippy::all)]

pub mod draw_invisible;
pub mod edit_view;
pub mod fonts;
pub mod main_state;
pub mod theme;
mod view_item;

pub use crate::edit_view::EditView;
pub use crate::main_state::{MainState, Settings};
pub use crate::view_item::TopBar;
