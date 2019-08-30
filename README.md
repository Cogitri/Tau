<a href="https://gitlab.gnome.org/World/tau">
    <img src="./data/icons//hicolor/scalable/apps/org.gnome.Tau.svg" alt="Tau logo" title="Tau" align="right" height="100" />
</a>

# Tau
[![Gitlab CI status](https://gitlab.gnome.org/World/Tau/badges/master/pipeline.svg)](https://gitlab.gnome.org/World/Tau/commits/master)
[![Translation Progress](https://hosted.weblate.org/widgets/Tau/-/translation/svg-badge.svg)](https://hosted.weblate.org/engage/Tau/?utm_source=widget)
[![CII Best Practices](https://bestpractices.coreinfrastructure.org/projects/2711/badge)](https://bestpractices.coreinfrastructure.org/projects/2711)
<a href="https://flathub.org/apps/details/org.gnome.Tau">
<img src="https://flathub.org/assets/badges/flathub-badge-i-en.png" width="190px" />
</a>

GTK frontend, written in Rust, for the [xi editor](https://github.com/xi-editor/xi-editor).
Previously called gxi, development now continues under the name "Tau".

![screenshot](/data/screenshot.png?raw=true)

## Contributing

### Getting started

Clone the repo and its submodules:

```sh
git clone --recurse-submodules https://gitlab.gnome.org/World/tau
```

You need the following dependencies installed:

	* Cairo >= 1.16
	* GDK-Pixbuf-2.0
	* GLib-2.0 >= 2.36
	* GTK+3 >= 3.20
	* Pango >= 1.38
	* Rust >= 1.35 # required for one of our deps

You can enable optional functionality with the `libhandy` meson switch,
like a more compact settings menu. You need the following dependencies
installed for that:

	* libhandy >= 0.10
	* GTK+3 >= 3.24.1

You have two ways of installing Tau:


#### Installation with cargo (e.g. for developing)

```sh
# install the syntect plugin, which adds a lot of funtionality to Tau,
# but isn't strictly required.
export XI_CONFIG_DIR="${PWD}"
make -C vendor/xi-editor/rust/syntect-plugin install

# Set accordingly if you want to use a custom xi-core binary. This will use
# whatever xi-core is in PATH and is the default if you don't set this env var.
# Please make sure that you have xi-core installed, e.g. with `cargo install --path vendor/xi-editor/rust xi-core`.
# Meson will do this for you.
export TAU_XI_BINARY_PATH="xi-core"

glib-compile-schemas data
env GSETTINGS_SCHEMA_DIR=data TAU_PLUGIN_DIR="${XI_CONFIG_DIR}/plugins" cargo run
```

This will launch Tau without you having to alter your system.

#### Permanent(-ish) installs (e.g. for distro packaging/day-to-day usage)


```sh
meson --prefix=/usr/local build
ninja -C build
sudo -E ninja -C build install
```

This will install Tau and its components to `/usr/local`. If you wish to install to a different prefix change the `--prefix`
argument you pass to meson. Do note that `sudo -E` isn't strictly necessary, but can avoid problems if you're using rustup.

### Docs

Please see the docs on https://gxi.cogitri.dev/docs to learn more about Tau's inner workings.
[gtk-rs' site](https://gtk-rs.org/) offers documentation and examples about how gtk-rs works.

### Translating

Visit [Weblate](https://hosted.weblate.org/engage/Tau/) to translate Tau.

## Install on different platforms

### Installation on Arch/Manjaro

There's currently one package for Tau in Arch Linux's
[AUR](https://aur.archlinux.org/), [tau-editor-git](https://aur.archlinux.org/packages/tau-editor-git/).  Building and installing (including dependencies) the first package
can be accomplished with:

```sh
yay -S tau-editor-git
```

Alternatively use `makepkg`:

```sh
curl -L -O https://aur.archlinux.org/cgit/aur.git/snapshot/tau-editor-git.tar.gz
tar xvf tau-editor-git.tar.gz
cd gxi
makepkg -Csri
```

Please consult the [Arch Wiki](https://wiki.archlinux.org/index.php/Arch_User_Repository#Installing_packages)
for more information regarding installing packages from the AUR.

### Installation on Alpine Linux

```sh
apk add tau
```

### Installation on Void Linux

```sh
xbps-install -S tau
```

### Flatpak

You can install the Tau Flatpak as described on [Flathub](https://flathub.org/apps/details/org.gnome.Tau)


### Installation on Windows

The following should give you a usable Tau binary:

0) Install Rust by visiting https://rustup.rs. After running the exe press `2` (right after you see the terminal of rustup-init.exe) to customize the settings and enter `x86_64-pc-windows-gnu` as default triplet (notice the `gnu` instead of `msvc`)
1) Go to https://www.msys2.org/ and download the appropriate installer (usually x86_64)
2) Go into your start menu and open the MSYS terminal
3) Enter `pacman -S mingw-w64-x86_64-toolchain mingw-w64-x86_64-gtk3 git` in the terminal
4) Open the `MinGW64` terminal from your start menu. Do `echo 'PATH="/c/Users/${USER}/.cargo/bin:${PATH}"' >> .bash_profile`
5) Reload the just made changes with `source .bash_profile`. Then clone Tau: `git clone https://gitlab.gnome.org/World/tau`.
6) `cd tau && cargo run` <- This should produce a debug build for you and run it.
