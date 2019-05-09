#!/bin/bash

if [ -z $1 ]; then
	echo "Please supply a version number!"
	exit 1
fi

mkdir -p release/var/lib/pacman ./tmp

pacman --noconfirm --cache ./cache --root ./release -Syu mingw-w64-x86_64-gtk3

cp -r release/mingw64/var release
rm -r release/mingw64/var

mv release/mingw64/* release

mkdir -p release/share/glib-2.0/schemas

cp data/com.github.Cogitri.gxi.gschema.xml release/share/glib-2.0/schemas

sed -i 's/@app_id@/com.github.Cogitri.gxi/' release/share/glib-2.0/schemas/com.github.Cogitri.gxi.gschema.xml

glib-compile-schemas ./release/share/glib-2.0/schemas
gdk-pixbuf-query-loaders.exe > release/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache

cp target/release/gxi.exe release/bin/

cat <<EOF <relase/README
Run bin/gxi.exe to start gxi.
EOF

mv release/README

zip -r gxi-"$1".zip gxi-"$1"
