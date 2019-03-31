#!/bin/sh

export GXI_PLUGIN_DIR="${8}"
export GXI_LOCALEDIR="${9}"
export GXI_VERSION="${10}"
export GXI_APP_ID="${11}"
export GXI_BUILD_PROFILE="${12}"
export GXI_NAME="${13}"

export GREEN='\033[0;32m'
export NO_COLOR='\033[0m'

echo "\tGXI Plugindir: ${GREEN}${GXI_PLUGIN_DIR}${NO_COLOR}
\tGXI Localedir:       ${GREEN}${GXI_LOCALEDIR}${NO_COLOR}
\tGXI Version:         ${GREEN}${GXI_VERSION}${NO_COLOR}
\tCrossbuild:          ${GREEN}${6}${NO_COLOR}
\tDetected GTK+3.22:   ${GREEN}${7}${NO_COLOR}
\tBuild profile:       ${GREEN}${GXI_BUILD_PROFILE}${NO_COLOR}
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

if [ "$GXI_BUILD_PROFILE" = "development" ]; then
	path=$(echo $path | sed 's|release|debug|')
	cargo build --target-dir ${4} ${features} && cp "${path}" ${2}/${3}
else
	cargo build --target-dir ${4} --release ${features} && cp "${path}" ${2}/${3}
fi
