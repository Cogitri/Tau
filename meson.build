project(
  'gxi', 'rust',
  version: '0.6.2',
  license: 'MIT',
)

dependency('glib-2.0', version: '>= 2.38')
dependency('gio-2.0', version: '>= 2.38')
dependency('cairo', version: '>= 1')
dependency('pango', version: '>=1.38')
dependency('gtk+-3.0', version: '>= 3.20')
# This is only used to decide what transition code to use for the search/replace Popover
opt_gtk3_22_dep = dependency('gtk+-3.0', version: '>= 3.22', required: false)

gxi_prefix = get_option('prefix')
gxi_bindir = join_paths(gxi_prefix, get_option('bindir'))
gxi_datadir = get_option('datadir')
gxi_localedir = join_paths(gxi_prefix, get_option('localedir'))
gxi_plugin_dir = join_paths(gxi_prefix, get_option('libdir'), 'gxi', 'plugins')
gxi_podir = join_paths(meson.current_source_dir(), 'po')

i18n = import('i18n')

cargo = find_program('cargo', required: false)
gresource = find_program('glib-compile-resources', required: false)
cargo_vendor = find_program('cargo-vendor', required: false)
cargo_script = find_program('scripts/cargo.sh')
grabber = find_program('scripts/grabber.sh')

c = run_command(grabber)
sources = c.stdout().strip().split('\n')

if meson.is_cross_build()
  cross_build='true'
else
  cross_build='false'
endif

if opt_gtk3_22_dep.found()
  gtk3_22='true'
else
  gtk3_22='false'
endif

if get_option('profile') == 'development'
  profile = 'devel'
  name_suffix = ' (Development)'
  vcs_tag = run_command('git', 'rev-parse', '--short', 'HEAD').stdout().strip()
  if vcs_tag == ''
    version_suffix = '-devel'
  else
    version_suffix = '-@0@'.format (vcs_tag)
  endif
else
  profile = ''
  name_suffix = ''
  version_suffix = ''
endif

application_id = 'com.github.Cogitri.gxi'

if profile != ''
  application_id = application_id + '.@0@'.format(profile)
endif

custom_target('cargo-build-gxi',
                        build_by_default: true,
                        build_always_stale: true,
                        input: sources,
                        output: ['gxi'],
                        install: true,
                        install_dir: gxi_bindir,
                        console: true,
                        command: [cargo_script,
                                  '@CURRENT_SOURCE_DIR@',
                                  join_paths(meson.build_root(),'@OUTDIR@'),
                                  '@OUTPUT@',
                                  meson.current_build_dir(),
                                  'gxi',
                                  cross_build,
                                  gtk3_22,
                                  gxi_plugin_dir,
                                  gxi_localedir,
                                  meson.project_version() + version_suffix,
                                  application_id,
                                  get_option('profile'),
                                  'gxi' + name_suffix,
                                 ])

syntect_build_path = join_paths(meson.current_source_dir(), 'vendor', 'xi-editor', 'rust', 'syntect-plugin')
syntect_dir = join_paths(gxi_plugin_dir, 'syntect')
syntect_bindir = join_paths(syntect_dir, 'bin')
syntect_releasebin_path = join_paths(meson.current_build_dir(), 'vendor', 'xi-editor', 'rust')

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

custom_target('cargo-build-syntect',
                        build_by_default: true,
                        build_always_stale: true,
                        input: sources,
                        output: ['xi-syntect-plugin'],
                        install: true,
                        install_dir: syntect_bindir,
                        console: true,
                        command: [cargo_script,
                                  syntect_build_path,
                                  join_paths(meson.build_root(),'@OUTDIR@'),
                                  '@OUTPUT@',
                                  syntect_releasebin_path,
                                  'xi-syntect-plugin',
                                  cross_build,
                                  '',
                                  gxi_plugin_dir,
                                  gxi_localedir,
                                  meson.project_version() + version_suffix,
                                  application_id,
                                  get_option('profile'),
                                  'gxi' + name_suffix,
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
  'cargo test',
  cargo,
  env: ['DONT_INSTALL_HUSKY_HOOKS=1'],
  args: 'test',
  timeout: 3000,
)
