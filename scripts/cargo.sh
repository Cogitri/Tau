#!/bin/sh

export GXI_PLUGIN_DIR=$7
export GXI_LOCALEDIR=$8
export GXI_APP_ID="com.github.Cogitri.gxi"
export GXI_VERSION=$9

export GREEN='\033[0;32m'
export NO_COLOR='\033[0m'

echo "\tGXI Plugindir: ${GREEN}${GXI_PLUGIN_DIR}${NO_COLOR}
\tGXI Localedir: ${GREEN}${GXI_LOCALEDIR}${NO_COLOR}
\tGXI App-ID:    ${GREEN}${GXI_APP_ID}${NO_COLOR}
\tGXI Version:   ${GREEN}${GXI_VERSION}${NO_COLOR}
\tCrossbuild:    ${GREEN}${6}
"


cd $1
if [ "$6" = "true" ]; then
    path="${4}/${RUST_TARGET}/release/${5}"
else
    path="${4}/release/${5}"
fi
cargo build --target-dir $4 --release && cp "${path}" $2/$3
