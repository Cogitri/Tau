<a href="https://gitlab.gnome.org/World/tau">
    <img src="./data/icons//hicolor/scalable/apps/org.gnome.Tau.svg" alt="Tau logo" title="Tau" align="right" height="100" />
</a>

# Tau
[![Gitlab CI status](https://gitlab.gnome.org/World/Tau/badges/master/pipeline.svg)](https://gitlab.gnome.org/World/Tau/commits/master)
[![CII Best Practices](https://bestpractices.coreinfrastructure.org/projects/2711/badge)](https://bestpractices.coreinfrastructure.org/projects/2711)
<a href="https://flathub.org/apps/details/org.gnome.Tau">
    <img src="https://flathub.org/assets/badges/flathub-badge-i-en.png" width="85px" />
</a>
<a href="https://repology.org/metapackage/tau-editor">
    <img src="https://repology.org/badge/vertical-allrepos/tau-editor.svg" alt="Tau Packaging Status" align="right">
</a>


GTK frontend, written in Rust, for the [xi editor](https://github.com/xi-editor/xi-editor).
Previously called gxi, development now continues under the name "Tau".

![screenshot](/data/screenshot.png?raw=true)

## Installation

### Ubuntu >= 19.10, Debian >= Unstable, Fedora >= 31 and OpenSUSE Tumbleweed

See https://software.opensuse.org/package/tau for binary packages of Tau. See https://build.opensuse.org/package/show/home:Cogitri/Tau for the source files of the packages. 

### Arch Linux

You can install binary releases of tau by adding this to your `/etc/pacman.conf`:

```sh
[home_Cogitri_Arch_Community_standard]
SigLevel = Never
Server = https://download.opensuse.org/repositories/home:/Cogitri/Arch_Community_standard/$arch
```

Afterwards run `pacman -Syu tau-editor`. Alternatively you can install Tau as `tau-editor-git` from the AUR [as per standard procedure](https://wiki.archlinux.org/index.php/Arch_User_Repository).

### Void Linux

```sh
xbps-install -Syu tau
```

### Alpine Linux
```
apk add tau
```

### Flatpak

See the instructions on https://flathub.org/apps/details/org.gnome.Tau

## Contributing

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
	* Rust >= 1.36 # required for one of our deps

You can enable optional functionality with the `libhandy` meson switch,
like a more compact settings menu. You need the following dependencies
installed for that:

	* libhandy >= 0.10
	* GTK+3 >= 3.24.1

Now installing Tau should be as easy as doing:


```sh
meson --prefix=/usr/local -Dprofile=development build
ninja -C build
sudo ninja -C build install
```

During development you can quickly test Tau with the following command:

```sh
ninja -C build run
```

You can run tests with:

```sh
ninja -C build test
```

But be mindful that those currently require the source-code-pro font to be installed.

### Docs

Please see the documentation in Tau's source files for further information as to
how Tau works.
[gtk-rs' site](https://gtk-rs.org/) offers documentation and examples about how gtk-rs works.

### Translating

Visit [GNOME's Damned Lies Platform](https://l10n.gnome.org/module/tau/) to translate Tau.


### Installation on Windows

The following should give you a usable Tau binary:

0) Install Rust by visiting https://rustup.rs. After running the exe press `2` (right after you see the terminal of rustup-init.exe) to customize the settings and enter `x86_64-pc-windows-gnu` as default triplet (notice the `gnu` instead of `msvc`)
1) Go to https://www.msys2.org/ and download the appropriate installer (usually x86_64)
2) Go into your start menu and open the MSYS terminal
3) Enter `pacman -S mingw-w64-x86_64-toolchain mingw-w64-x86_64-gtk3 git` in the terminal
4) Open the `MinGW64` terminal from your start menu. Do `echo 'PATH="/c/Users/${USER}/.cargo/bin:${PATH}"' >> .bash_profile`
5) Reload the just made changes with `source .bash_profile`. Then clone Tau: `git clone https://gitlab.gnome.org/World/tau`.
6) `cd tau && cargo run` <- This should produce a debug build for you and run it.
