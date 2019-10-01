desktop_file = i18n.merge_file ('desktop-file',
  type: 'desktop',
  input: configure_file(
    input: files('org.gnome.Tau.desktop.in.in'),
    output: 'org.gnome.Tau.desktop.in',
    configuration: conf,
  ),
  output: '@0@.desktop'.format(application_id),
  po_dir: tau_podir,
  install: true,
  install_dir: join_paths (tau_datadir, 'applications'),
)

desktop_file_validate = find_program('desktop-file-validate', required: false)

if desktop_file_validate.found()
    test(
        'validate-desktop', desktop_file_validate,
        args: [
            desktop_file.full_path(),
	    ],
    )
endif

appdata_file = i18n.merge_file ('appdata-file',
  input: configure_file(
    input: files('org.gnome.Tau.appdata.xml.in.in'),
    output: 'org.gnome.Tau.appdata.xml.in',
    configuration: conf,
  ),
  output: '@0@.appdata.xml'.format(application_id),
  po_dir: tau_podir,
  install: true,
  install_dir: join_paths (tau_datadir, 'metainfo'),
)

appstream_util = find_program('appstream-util', required: false)

# We have to check if appstream util has been found here, or meson will die if he haven't found it
if appstream_util.found()
  # https://github.com/hughsie/appstream-glib/issues/146
  appstream_min_version = '0.7.2'

  appstream_ver_stdout = run_command([appstream_util, '--version'])
  appstream_ver = appstream_ver_stdout.stdout().split('	')[1].strip()

  if appstream_ver.version_compare('<' + appstream_min_version)
    # Using 'warning' here would be a good idea, but meson on Ubuntu 14.04 isn't recent enough.
    message('WARN: Found appstream, but it is too old! Wanted >= @0@, found @1@'.format(appstream_ver, appstream_min_version))
  else
    test(
        'validate-appdata', appstream_util,
        args: [
            'validate-relax', '--nonet', appdata_file.full_path(),
	    ],
    )
   endif
endif

compile_schemas = find_program('glib-compile-schemas', required: false)
if compile_schemas.found()
  test('Validate schema file', compile_schemas,
    args: ['--strict', '--dry-run', meson.current_source_dir()]
  )
endif

install_data('org.gnome.Tau.gschema.xml', install_dir: join_paths(tau_datadir, 'glib-2.0', 'schemas'))

subdir('icons')
