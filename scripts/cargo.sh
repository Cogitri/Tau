#!/bin/sh

export GXI_PLUGIN_DIR=$5

cargo build --manifest-path $1/Cargo.toml --release && cp $3/target/${RUST_TARGET}/release/$4 $2