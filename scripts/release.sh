#!/bin/bash

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
	meson_options.txt \
	meson_post_install.py \
	LICENSE \
	README.md \
	scripts \
	src \
	po \
	data

# cargo vendor
pushd $SRC/vendor/xi-editor/rust/syntect-plugin/
mkdir -p $DIST/vendor/xi-editor/rust/syntect-plugin/.cargo/
cargo vendor --no-merge-sources  cargo-vendor | sed -r 's|(^directory = ).*(vendor.*)|\\1"\\2"|g' > $DIST/vendor/xi-editor/rust/syntect-plugin/.cargo/config
popd

ginst vendor

mkdir $DIST/.cargo
cargo vendor cargo-vendor | sed 's/^directory = ".*"/directory = "cargo-vendor"/g' > $DIST/.cargo/config
ginst cargo-vendor
ginst .cargo

# packaging
cd $DEST/dist
tar cJvf $PKGVER.tar.xz $PKGVER
