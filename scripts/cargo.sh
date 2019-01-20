#!/bin/sh

export GXI_PLUGIN_DIR=$6
export GXI_LOCALEDIR=$7
export GXI_APP_ID="com.github.Cogitri.gxi"
export GXI_VERSION=$8

cargo build --manifest-path $1/Cargo.toml --target-dir $3 --release && cp $3/$5/release/$4 $2