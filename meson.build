project(
  'tau', 'rust',
  version: '0.9.1',
  license: 'MIT',
)

dependency('glib-2.0', version: '>= 2.38')
dependency('gio-2.0', version: '>= 2.38')
dependency('cairo', version: '>= 1')
dependency('pango', version: '>=1.38')
dependency('gtk+-3.0', version: '>= 3.20')
# This is only used to decide what transition code to use for the search/replace Popover
opt_gtk3_22_dep = dependency('gtk+-3.0', version: '>= 3.22', required: false)

tau_prefix = get_option('prefix')
tau_bindir = join_paths(tau_prefix, get_option('bindir'))
tau_datadir = get_option('datadir')
tau_localedir = join_paths(tau_prefix, get_option('localedir'))
tau_libexecdir = join_paths(tau_prefix, get_option('libexecdir'), 'tau')
tau_plugin_dir = join_paths(tau_libexecdir, 'plugins')
tau_podir = join_paths(meson.current_source_dir(), 'po')

i18n = import('i18n')

cargo = find_program('cargo', required: true)
gresource = find_program('glib-compile-resources', required: true)
cargo_script = find_program('scripts/cargo.sh', required: true)
xi_core = find_program('xi-core', required: false)

xi_opt = get_option('xi-core')

if xi_opt == 'auto'
  if xi_core.found()
    message('Found xi-core, using system\'s xi-core')
    xi_opt = 'system'
  else
    message('Couldn\'t find xi-core, using builtin xi-core')
    xi_opt = 'builtin'
  endif
elif xi_opt == 'system'
  if not xi_core.found()
    message('WARN: Requested system xi-core but couldn\'t find the xi-core binary! Please make sure to have it available during run-time')
  endif
endif

sources = files(
  'src/gschema-config-storage/src/lib.rs',
  'src/gschema-config-storage/src/pref_storage.rs',
  'src/gschema-config-storage/src/errors.rs',
  'src/gschema-config-storage/src/macros.rs',
  'src/editview/src/main_state.rs',
  'src/editview/src/edit_view.rs',
  'src/editview/src/lib.rs',
  'src/editview/src/view_item.rs',
  'src/editview/src/fonts.rs',
  'src/editview/src/theme.rs',
  'src/tau/src/main_win.rs',
  'src/tau/src/main.rs',
  'src/tau/src/errors.rs',
  'src/tau/src/about_win.rs',
  'src/tau/src/prefs_win.rs',
  'src/tau/src/globals.rs',
  'src/tau/src/panic_handler.rs',
  'Cargo.toml',
  'Cargo.lock',
)

if opt_gtk3_22_dep.found()
  gtk3_22='true'
else
  gtk3_22='false'
endif

if xi_opt == 'builtin'
  xi_binary_path = join_paths(tau_libexecdir, 'xi-core')
else
  xi_binary_path = xi_core.path()
endif

custom_target('cargo-build-tau',
                        input: sources,
                        output: ['tau'],
                        install: true,
                        install_dir: tau_bindir,
                        console: true,
                        command: [cargo_script,
                                  meson.current_source_dir(),
                                  join_paths(meson.build_root(),'@OUTDIR@'),
                                  '@OUTPUT@',
                                  meson.current_build_dir(),
                                  'tau',
                                  gtk3_22,
                                  tau_plugin_dir,
                                  tau_localedir,
                                  meson.project_version(),
                                  xi_binary_path,
                                  gresource.path(),
                                 ])

git = find_program('git', native: true, required: false)

subdir_xi_check = run_command('sh', '-c', 'ls -d $MESON_SOURCE_ROOT/vendor/xi-editor')
subdir_syntect_check = run_command('sh', '-c', 'ls -d $MESON_SOURCE_ROOT/vendor/xi-editor/rust/syntect-plugin/assets/')

if subdir_xi_check.returncode() != 0 or subdir_syntect_check.returncode() != 0
    if git.found()
        message('Couldn\'t find xi-editor subdir, trying to clone it')
        if run_command(git, 'status').returncode() == 128
            error('Can\'t clone submodule when not in a git repo! Make sure you\'re using the github release and not the automatically generated tarball!')
        endif
        submodule_update = run_command(git, 'submodule', 'update', '--init')
        message(submodule_update.stdout())
    else
        error('Couldn\'t find xi-editor subdir! If you\'re on a git checkout please install git and try again, if not make sure you\'re using the github release and not the automatically generated tarball!')
    endif
endif

xi_vendor_target_dir = join_paths(meson.current_build_dir(), 'vendor', 'xi-editor', 'rust')

if xi_opt == 'builtin'
  xi_build_path = join_paths(meson.current_source_dir(), 'vendor', 'xi-editor', 'rust')
  custom_target('cargo-build-xi-core',
                          output: ['xi-core'],
                          install: true,
                          install_dir: tau_libexecdir,
                          console: true,
                          command: [cargo_script,
                                    xi_build_path,
                                    join_paths(meson.build_root(),'@OUTDIR@'),
                                    '@OUTPUT@',
                                    xi_vendor_target_dir,
                                    'xi-core',
                                    '',
                                    tau_plugin_dir,
                                    tau_localedir,
                                    meson.project_version(),
                                   ])
endif

syntect_build_path = join_paths(meson.current_source_dir(), 'vendor', 'xi-editor', 'rust', 'syntect-plugin')
syntect_dir = join_paths(tau_plugin_dir, 'syntect')
syntect_bindir = join_paths(syntect_dir, 'bin')

syntect_plugin_files = files(
  'vendor/xi-editor/rust/syntect-plugin/src/main.rs',
  'vendor/xi-editor/rust/syntect-plugin/src/stackmap.rs',
  'vendor/xi-editor/rust/syntect-plugin/Cargo.toml',
  'vendor/xi-editor/rust/Cargo.lock',
)

custom_target('cargo-build-syntect',
                        input: syntect_plugin_files,
                        output: ['xi-syntect-plugin'],
                        install: true,
                        install_dir: syntect_bindir,
                        console: true,
                        command: [cargo_script,
                                  syntect_build_path,
                                  join_paths(meson.build_root(),'@OUTDIR@'),
                                  '@OUTPUT@',
                                  xi_vendor_target_dir,
                                  'xi-syntect-plugin',
                                  '',
                                  tau_plugin_dir,
                                  tau_localedir,
                                  meson.project_version(),
                                 ])

synctect_assets = ['assets/Makefile.toml', 'assets/YAML.toml', 'manifest.toml']

foreach a: synctect_assets
    file = join_paths(syntect_build_path, a)
    install_data(
        file,
        install_dir: syntect_dir,
    )
endforeach

meson.add_install_script('meson_post_install.sh')

run_target(
  'release',
  command: ['scripts/release.sh', meson.project_name(), meson.project_version()]
)

subdir('data')
subdir('po')

test(
  'cargo test --all',
  cargo,
  args: 'test',
  timeout: 3000,
)
