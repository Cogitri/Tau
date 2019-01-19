#!/bin/sh

export GXI_PLUGIN_DIR=$6

cargo build --manifest-path $1/Cargo.toml --target-dir $3 --release && cp $3/$5/release/$4 $2