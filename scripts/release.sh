#!/bin/bash

if ! [ "$MESON_BUILD_ROOT" ]; then
    echo "This can only be run via meson, exiting!"
    exit 1
fi

PKGVER=$1-$2
DEST=${MESON_BUILD_ROOT}
DIST=$DEST/dist/$PKGVER
SRC=${MESON_SOURCE_ROOT}


cd "${MESON_SOURCE_ROOT}"
mkdir -p $DIST

ginst() {
	cp -rf $@ $DIST
}

ginst build.rs \
	Cargo.toml \
	Cargo.lock \
	meson.build \
	meson_post_install.sh \
	LICENSE \
	README.md \
	scripts \
	src \
	po \
	data

# cargo vendor
pushd $SRC/vendor/xi-editor/rust/syntect-plugin/
mkdir -p $DIST/vendor/xi-editor/rust/syntect-plugin/.cargo/
# Replace full path with relative path via sed
cargo vendor --no-merge-sources | sed -r 's|(^directory = ).*(vendor.*)|\1"\2|g' > $DIST/vendor/xi-editor/rust/syntect-plugin/.cargo/config
popd

ginst vendor

mkdir $DIST/.cargo
cargo vendor cargo-vendor | sed 's/^directory = ".*"/directory = "cargo-vendor"/g' > $DIST/.cargo/config
ginst cargo-vendor
ginst .cargo

# packaging
cd $DEST/dist
tar cJvf $PKGVER.tar.xz $PKGVER
