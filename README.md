# gxi
[![Build Status](https://drone.exqa.de/api/badges/Cogitri/gxi/status.svg)](https://drone.exqa.de/Cogitri/gxi)

GTK frontend, written in Rust for the [xi editor](https://github.com/google/xi-editor).

gxi is a work in progress!

![screenshot](https://raw.githubusercontent.com/bvinc/gxi/master/screenshot.png)

## Instructions

You need to have the Rust compiler installed.  I recommend using [rustup](https://rustup.rs/).

### Installing dependencies on Debian/Ubuntu

```sh
sudo apt-get install libgtk-3-dev
```

### Installing dependencies on Redhat

```sh
sudo yum install gtk3-devel
```

### Enabling the syntect syntax highlighting plugin

Running these commands will put the syntect plugin into your `~/.config/xi/plugins` directory.

```sh
git clone https://github.com/google/xi-editor/
cd xi-editor/rust/syntect-plugin/
make install
```

### Running gxi
Running this command will put the gxi binary in `$CARGO_HOME/bin`, which usually is
`$HOME/.cargo/bin` and should be in your `PATH` if you've used rustup.

```sh
cargo install --git https://github.com/Cogitri/gxi
```

After this you can run gxi via

```sh
gxi
```
