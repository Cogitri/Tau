#!/bin/sh

export GXI_PLUGIN_DIR=$5

if [ "$6" = "true" ]; then
	path="${3}/${RUST_TARGET}/release/${4}"
else
	path="${3}/release/${4}"
fi

cargo build --manifest-path $1/Cargo.toml --target-dir $3 --release && cp "$path" $2
