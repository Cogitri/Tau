# gxi
[![Build Status](https://drone.exqa.de/api/badges/Cogitri/gxi/status.svg)](https://drone.exqa.de/Cogitri/gxi)

GTK frontend, written in Rust, for the [xi editor](https://github.com/google/xi-editor).

gxi is a work in progress!

![screenshot](/screenshot.jpg?raw=true)

## Installing

You need the following dependencies installed:

	* Cairo >= 1.16
	* GDK-Pixbuf-2.0
	* GLib-2.0 >= 2.36
	* GTK+3>= 3.20
	* Pango >= 1.38
	* Meson >= 0.46
	* Rust >= 1.31

Run the following commands to install gxi if it's not available via your package manager:

```sh
meson build
ninja -C build
sudo ninja -C build install
```

This will install the gxi binary to /usr/local/bin/gxi and the syntect plugin to /usr/local/lib/gxi/plugins/syntect.
This plugin has to be installed for some functionality, such as syntax highlighting, auto indention and control
whether or not tabs should be replaced with spaces. It has to be compiled of the same git rev as the xi-core-lib
that's built into gxi, so please don't use `cargo` to install gxi, as that won't install syntect! Installing syntect
from a different rev can lead to very weird bugs.


After these steps you should be able to run gxi simply by invoking `gxi`