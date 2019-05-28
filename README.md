<a href="https://cogitri.github.io/gxi">
    <img src="./data/icons//hicolor/scalable/apps/com.github.Cogitri.gxi.svg" alt="gxi logo" title="gxi" align="right" height="100" />
</a>

# gxi
[![Drone CI Build Status](https://drone.exqa.de/api/badges/Cogitri/gxi/status.svg)](https://drone.exqa.de/Cogitri/gxi)
[![Travis CI Build Status](https://travis-ci.com/Cogitri/gxi.svg?branch=master)](https://travis-ci.com/Cogitri/gxi)
[![Translation Progress](https://hosted.weblate.org/widgets/gxi/-/translation/svg-badge.svg)](https://hosted.weblate.org/engage/gxi/?utm_source=widget)
[![CII Best Practices](https://bestpractices.coreinfrastructure.org/projects/2711/badge)](https://bestpractices.coreinfrastructure.org/projects/2711)

GTK frontend, written in Rust, for the [xi editor](https://github.com/google/xi-editor).

![screenshot](/data/screenshot.png?raw=true)

## Contributing

### Getting started

You need the following dependencies installed:

	* Cairo >= 1.16
	* GDK-Pixbuf-2.0
	* GLib-2.0 >= 2.36
	* GTK+3>= 3.20
	* Pango >= 1.38
	* Rust >= 1.31

You have two ways of installing gxi:


#### Installation with cargo (e.g. for developing)

#### Installing the syntect plugin

```sh
# install the syntect plugin, which adds a lot of funtionality to gxi,
# but isn't strictly required.
export XI_CONFIG_DIR="${PWD}"
make -C vendor/xi-editor/rust/syntect-plugin install

glib-compile-schemas data
env GSETTINGS_SCHEMA_DIR=data GXI_PLUGIN_DIR="${XI_CONFIG_DIR}/plugins" cargo run
```

This will launch gxi without you having to alter your system.

#### Permanent(-ish) installs (e.g. for distro packaging/day-to-day usage)


```sh
meson --prefix=/usr/local build
ninja -C build
sudo -E ninja -C build install
```

This will install gxi and its components to `/usr/local`. If you wish to install to a different prefix change the `--prefix`
argument you pass to meson. Do note that `sudo -E` isn't strictly necessary, but can avoid problems if you're using rustup.

### Docs

Please see the docs on https://gxi.cogitri.dev/docs to learn more about gxi's inner workings. 
[gtk-rs' site](https://gtk-rs.org/) offers documentation and examples about how gtk-rs works.

### Translating

Visit [Weblate](https://hosted.weblate.org/engage/gxi/) to translate gxi.

## Install on different platforms

### Installation on Arch/Manjaro

There are two packages for gxi in Arch Linux's
[AUR](https://aur.archlinux.org/). The first is the regular release cycle
package [gxi](https://aur.archlinux.org/packages/gxi/) and the second is the git
repository tracking package
[gxi-git](https://aur.archlinux.org/packages/gxi-git/). Building and installing
(including dependencies) the first package can be accomplished with:

```sh
yay -S gxi
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

### Installation on Windows

I'll try to have binary releases for Windows by gxi version 0.7.0. If you don't want to wait,
or want to use a debug build for developing follow the following instructions:

0) Install Rust by visiting https://rustup.rs. After running the exe press `2` (right after you see the terminal of rustup-init.exe) to customize the settings and enter `x86_64-pc-windows-gnu` as default triplet (notice the `gnu` instead of `msvc`)
1) Go to https://www.msys2.org/ and download the appropriate installer (usually x86_64)
2) Go into your start menu and open the MSYS terminal
3) Enter `pacman -S mingw-w64-x86_64-toolchain mingw-w64-x86_64-gtk3 git` in the terminal
4) Open the `MinGW64` terminal from your start menu. Do `echo 'PATH="/c/Users/${USER}/.cargo/bin:${PATH}"' >> .bash_profile`
5) Reload the just made changes with `source .bash_profile`. Then clone gxi: `git clone https://github.com/Cogitri/gxi`.
6) `cd gxi && cargo run` <- This should produce a debug build for you and run it.
