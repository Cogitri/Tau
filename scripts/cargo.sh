#!/bin/sh

export GXI_PLUGIN_DIR=$8
export GXI_LOCALEDIR=$9
export GXI_VERSION=$10

export GREEN='\033[0;32m'
export NO_COLOR='\033[0m'

echo "\tGXI Plugindir: ${GREEN}${GXI_PLUGIN_DIR}${NO_COLOR}
\tGXI Localedir:       ${GREEN}${GXI_LOCALEDIR}${NO_COLOR}
\tGXI Version:         ${GREEN}${GXI_VERSION}${NO_COLOR}
\tCrossbuild:          ${GREEN}${6}${NO_COLOR}
\tDetected GTK+3.22:   ${GREEN}${7}${NO_COLOR}
"

cd $1
if [ "$6" = "true" ]; then
    path="${4}/${RUST_TARGET}/release/${5}"
else
    path="${4}/release/${5}"
fi
if [ "$7" = "true" ]; then
    features="--features gtk_v3_22"
fi

cargo build --target-dir $4 --release ${features} && cp "${path}" $2/$3
