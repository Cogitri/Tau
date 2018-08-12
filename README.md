# gxi
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

```sh
git clone https://github.com/bvinc/gxi.git
cd gxi
cargo run
```
