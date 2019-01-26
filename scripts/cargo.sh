#!/bin/sh

export GXI_PLUGIN_DIR=$6
export GXI_LOCALEDIR=$7
export GXI_APP_ID="com.github.Cogitri.gxi"
export GXI_VERSION=$8

cd $1
cargo build --target-dir $4 --release && cp $4/$6/release/$5 $2/$3
