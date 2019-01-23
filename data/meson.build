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

if appstream_util.found()
    test(
        'validate-appdata', appstream_util,
        args: [
            'validate-relax', '--nonet', appdata_file.full_path(),
	    ],
    )
endif

install_data('com.github.Cogitri.gxi.gschema.xml', install_dir: join_paths(get_option('datadir'), 'glib-2.0', 'schemas'))