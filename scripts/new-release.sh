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
sed -i "s/version = \"$current\"/version = \"$next\"/" src/tau/Cargo.toml
${EDITOR:=nano} data/org.gnome.Tau.appdata.xml.in.in

printf "%s\\n\\n%s" "$(./scripts/make-changelog.sh v${next} v${current})" "$(cat Changelog.md)" > Changelog.md

ninja -C _build test

git commit -av
git tag v$next

ninja -C _build release
