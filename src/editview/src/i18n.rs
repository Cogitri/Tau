// Copyright (C) 2019-2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: MIT
// Based on fractal's fractal-gtk/src/util/i18n.rs. License: GPL-3.0-or-later

use gettextrs::ngettext;

pub fn freplace(input: String, args: &[&str]) -> String {
    let mut parts = input.split("{}");
    let mut output = parts.next().unwrap_or_default().to_string();
    for (p, a) in parts.zip(args.iter()) {
        output += &(a.to_string() + &p.to_string());
    }
    output
}

#[allow(dead_code)]
pub fn ni18n_f(single: &str, multiple: &str, number: u32, args: &[&str]) -> String {
    let s = ngettext(single, multiple, number);
    freplace(s, args)
}
