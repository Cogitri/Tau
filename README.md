<a href="https://cogitri.github.io/gxi">
    <img src="./data/icons//hicolor/scalable/apps/com.github.Cogitri.gxi.svg" alt="gxi logo" title="gxi" align="right" height="100" />
</a>

# gxi
[![Drone CI Build Status](https://drone.exqa.de/api/badges/Cogitri/gxi/status.svg)](https://drone.exqa.de/Cogitri/gxi)
[![Travis CI Build Status](https://travis-ci.com/Cogitri/gxi.svg?branch=master)](https://travis-ci.com/Cogitri/gxi)
[![Translation Progress](https://hosted.weblate.org/widgets/gxi/-/translation/svg-badge.svg)](https://hosted.weblate.org/engage/gxi/?utm_source=widget)

GTK frontend, written in Rust, for the [xi editor](https://github.com/google/xi-editor).

![screenshot](/data/screenshot.png?raw=true)

## Contributing

Please see the docs on https://cogitri.github.io/gxi to learn more about gxi's inner workings. 
[gtk-rs' site](https://gtk-rs.org/) offers documentation and examples about how gtk-rs works.

Visit [Weblate](https://hosted.weblate.org/engage/gxi/) to translate gxi.

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

### Installation on Arch/Manjaro

There are two packages for gxi in Arch Linux's
[AUR](https://aur.archlinux.org/). The first is the regular release cycle
package [gxi](https://aur.archlinux.org/packages/gxi/) and the second is the git
repository tracking package
[gxi-git](https://aur.archlinux.org/packages/gxi-git/). Building and installing
(including dependencies) the first package can be accomplished with:

```sh
yaourt -Sy gxi
```

Alternatively use `makepkg`:

```sh
curl -L -O https://aur.archlinux.org/cgit/aur.git/snapshot/gxi.tar.gz
tar -xvf gxi.tar.gz
cd gxi
makepkg -Csri
```

Building and installing the git tracking package is identical, just replace all occurrences of
`gxi` with `gxi-git`.

Please consult the [Arch Wiki](https://wiki.archlinux.org/index.php/Arch_User_Repository#Installing_packages)
for more information regarding installing packages from the AUR.
