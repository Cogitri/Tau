#!/usr/bin/env python3

import os
import subprocess

if not os.environ.get('DESTDIR'):
    schemadir = os.path.join(os.environ['MESON_INSTALL_PREFIX'], 'share', 'glib-2.0', 'schemas')
    print('Compiling gsettings schemas...')
    subprocess.call(['glib-compile-schemas', schemadir])

    icondir = os.path.join(os.environ['MESON_INSTALL_PREFIX'], 'share', 'icons', 'hicolor')
    print('Updating icon cache...')
    subprocess.call(['gtk-update-icon-cache', '-f', '-t', icondir])
    print('Updating destkop (MIME) database...')
    subprocess.call(['update-desktop-database', icondir])
