#!/bin/bash

current=$(grep -Po "version: '\K([0-9]*\.[0-9]*.[0-9]+)(?=')" meson.build)
major=$(cut -d '.' -f1 <<< "$current")
minor=$(cut -d '.' -f2 <<< "$current")
patch=$(cut -d '.' -f3 <<< "$current")

case $1 in
    major)
        next=$(echo $((major + 1)).0.0)
        ;;
    minor)
        next=$(echo $major.$((minor + 1)).0)
        ;;
    patch)
        next=$(echo $major.$minor.$((patch + 1)))
        ;;
    *)
        echo "Don't know what to do, exiting!"
        exit 1
    ;;
esac

sed -i "s/version: '$current'/version: '$next'/" meson.build
sed -i "s/version = \"$current\"/version = \"$next\"/" Cargo.toml
sed -i "s/version=\"$current\".*/version=\"$next\" date=\"$(date +%Y-%m-%d)\"\/>/" data/com.github.Cogitri.gxi.appdata.xml.in

cargo check

git commit -av
git tag -s $next

ninja -C build release
