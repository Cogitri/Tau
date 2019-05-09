desktop_file = i18n.merge_file ('desktop-file',
  type: 'desktop',
  input: 'com.github.Cogitri.gxi.desktop.in',
  output: 'com.github.Cogitri.gxi.desktop',
  po_dir: gxi_podir,
  install: true,
  install_dir: join_paths (gxi_datadir, 'applications'),
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
  input: 'com.github.Cogitri.gxi.appdata.xml.in',
  output: 'com.github.Cogitri.gxi.appdata.xml',
  po_dir: gxi_podir,
  install: true,
  install_dir: join_paths (gxi_datadir, 'metainfo'),
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

conf = configuration_data()
conf.set('app_id', application_id)
configure_file(input: 'com.github.Cogitri.gxi.gschema.xml.in', install_dir: join_paths(gxi_datadir, 'glib-2.0', 'schemas'), output: application_id + '.gschema.xml', install: true, configuration: conf)

install_subdir('icons', install_dir: gxi_datadir)
