// Copyright (C) 2017-2018 Brian Vincent <brainn@gmail.com>
// Copyright (C) 2019-2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: MIT

pub static LOCALEDIR: Option<&str> = option_env!("TAU_LOCALEDIR");
pub static VERSION: Option<&str> = option_env!("TAU_VERSION");
pub static PLUGIN_DIR: Option<&str> = option_env!("TAU_PLUGIN_DIR");
pub static APP_ID: Option<&str> = option_env!("TAU_APP_ID");
pub static NAME: Option<&str> = option_env!("TAU_NAME");
