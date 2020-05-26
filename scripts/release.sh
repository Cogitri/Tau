#!/bin/bash -e

if ! [ "$MESON_BUILD_ROOT" ]; then
    echo "This can only be run via meson, exiting!"
    exit 1
fi

PKGVER=$1-$2
DEST=${MESON_BUILD_ROOT}
DIST=$DEST/dist/$PKGVER
SRC=${MESON_SOURCE_ROOT}


cd "${MESON_SOURCE_ROOT}"
mkdir -p "${DIST}"

ginst() {
	cp -rf $@ "${DIST}"
}

ginst \
	Cargo.toml \
	Cargo.lock \
	meson.build \
	meson_post_install.sh \
	meson_options.txt \
	LICENSE \
	README.md \
	scripts \
	src \
	po \
	data

pushd "${SRC}"/vendor/xi-editor/rust/syntect-plugin/
mkdir -p "${DIST}"/vendor/xi-editor/rust/syntect-plugin/.cargo/
# Replace full path with relative path via sed
cargo vendor syntect-vendor | sed -r 's|(^directory = ).*(syntect-vendor.*)|\1"\2|g' > "${DIST}"/vendor/xi-editor/rust/syntect-plugin/.cargo/config
popd

ginst vendor

mkdir -p "${DIST}"/.cargo
cargo vendor tau-vendor | sed 's/^directory = ".*"/directory = "tau-vendor"/g' > "${DIST}"/.cargo/config
ginst tau-vendor

# packaging
cd "${DEST}"/dist
tar cJvf $PKGVER.tar.xz $PKGVER

#if type gpg; then
#	gpg --armor --detach-sign $PKGVER.tar.xz
#	gpg --verify $PKGVER.tar.xz.asc $PKGVER.tar.xz
#fi
