#!/usr/bin/env python3

import os
import subprocess

if not os.environ.get('DESTDIR'):
    schemadir = os.path.join(os.environ['MESON_INSTALL_PREFIX'], 'share', 'glib-2.0', 'schemas')
    print('Compiling gsettings schemas...')
    subprocess.call(['glib-compile-schemas', schemadir])
