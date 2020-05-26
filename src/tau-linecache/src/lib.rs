// Copyright (C) 2017-2018 Brian Vincent <brainn@gmail.com>
// Copyright (C) 2019-2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: MIT

#![recursion_limit = "128"]

pub mod linecache;

pub use crate::linecache::{Line, LineCache};
