#!/bin/sh

if [ -z "$DESTDIR" ]; then
    schemadir="$MESON_INSTALL_PREFIX/share/glib-2.0/schemas"
    icondir="$MESON_INSTALL_PREFIX/share/icons/hicolor"

    printf "%s\\n" "Compiling GSettings schemas in dir $schemadir"
    glib-compile-schemas $schemadir

    printf "%s\\n" "Updating GTK icon cache in dir $icondir"
    gtk-update-icon-cache -f -t $icondir

    printf "%s\\n" "Updating desktop (MIME) database"
    update-desktop-database

    if [ $? != 0 ]; then
        printf "Failed to compile desktop (MIME) database!"
    fi
fi
