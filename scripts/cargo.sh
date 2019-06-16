#!/bin/sh

set -e

# $1 -> @CURRENT_SOURCE_DIR@ as replaced by meson or syntect_plugin_path, we cd into it
# $2 -> Directory to place the finished binaries, meson replaces @OUTDIR@
# $3 -> The final name of the tau binary, set output: ['name'] in meson.build
# to change it.
# $4 -> passed to cargo --target-dir, .current_build_dir() from meson
# $5 -> Name of the binary created by cargo inside target/
# $6 -> Whether we should enable gtk3_22 support, enabled automatically
# if we have gtk+-3.0 >= 3.22

# These variables are used by Tau itself in src/globals.rs
# to decide where to look for certain system components
export TAU_PLUGIN_DIR="${7}"
export TAU_LOCALEDIR="${8}"
export TAU_VERSION="${9}"
export TAU_XI_BINARY_PATH="${10}"
export GRESOURCE_BINARY_PATH="${11}"

# ANSI codes for getting colors and resetting it
RED='\033[0;31m'
GREEN='\033[0;32m'
NO_COLOR='\033[0m'

echo -e \
"
\tTau Plugindir:\t\t${GREEN}${TAU_PLUGIN_DIR}${NO_COLOR}
\tTau Localedir:\t\t${GREEN}${TAU_LOCALEDIR}${NO_COLOR}
\tTau Version:\t\t${GREEN}${TAU_VERSION}${NO_COLOR}
\tDetected GTK+3.22:\t${GREEN}${6}${NO_COLOR}
"

cd "$1"

if [ "$6" = "true" ]; then
    features="--features gtk_v3_22"
fi

cargo build --target-dir "${4}" --release ${features}

# Cargo can place this here if we're crosscompiling
if [ -f "${4}/${RUST_TARGET}/release/${5}" ]; then
    path="${4}/${RUST_TARGET}/release/${5}"
elif [ -f "${4}/release/${5}" ]; then
    path="${4}/release/${5}"
else
    echo "${RED}Can't determine what dir cargo places compiled binaries in!${NO_COLOR}"
    exit 1
fi

cp "${path}" "${2}/${3}"
