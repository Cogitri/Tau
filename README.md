# gxi
GTK frontend for the [xi editor](https://github.com/google/xi-editor), written in Rust.

gxi is a work in progress!

## Instructions

### Installing dependencies on Debian/Ubuntu

```sh
sudo apt-get install libgtk-3-dev
```

### Installing xi-core

xi-core must be executable from your `PATH`.

```sh
git clone https://github.com/google/xi-editor.git
cd xi-editor/rust
cargo install
```

### Running gxi

```sh
git clone https://github.com/bvinc/gxi.git
cd gxi
cargo run
```
