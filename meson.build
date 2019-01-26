project(
  'gxi', 'rust',
  version: '0.4.4',
  license: 'MIT',
)

dependency('glib-2.0', version: '>= 2.56')
dependency('gio-2.0', version: '>= 2.56')
dependency('cairo', version: '>= 1.16')
dependency('gtk+-3.0', version: '>= 3.20')
# required for syntect-plugin: onig_sys
#find_program('cmake', required: true)

gxi_prefix = get_option('prefix')
gxi_bindir = join_paths(gxi_prefix, get_option('bindir'))
gxi_datadir = get_option('datadir')
gxi_localedir = join_paths(gxi_prefix, get_option('localedir'))
gxi_plugin_dir = join_paths(gxi_prefix, get_option('libdir'), 'gxi', 'plugins')
gxi_podir = join_paths(meson.current_source_dir(), 'po')

i18n = import('i18n')
subdir('data')
subdir('po')

cargo = find_program('cargo', required: false)
gresource = find_program('glib-compile-resources', required: false)
cargo_vendor = find_program('cargo-vendor', required: false)
cargo_script = find_program('scripts/cargo.sh')
grabber = find_program('scripts/grabber.sh')

c = run_command(grabber)
sources = c.stdout().strip().split('\n')

# Well this isn't pretty
rust_target = get_option('rust-target')

custom_target('cargo-build-gxi',
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
                                  rust_target,
                                  gxi_plugin_dir,
                                  gxi_localedir,
                                  meson.project_version(),
                                 ])

syntect_build_path = join_paths(meson.current_source_dir(), 'vendor', 'xi-editor', 'rust', 'syntect-plugin')
syntect_dir = join_paths(gxi_plugin_dir, 'syntect')
syntect_bindir = join_paths(syntect_dir, 'bin')
syntect_releasebin_path = join_paths(meson.current_build_dir(), 'vendor', 'xi-editor', 'rust')

custom_target('cargo-build-syntect',
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
                                  rust_target,
                                 ])

synctect_assets = ['assets/Makefile.toml', 'assets/YAML.toml', 'manifest.toml']

foreach a: synctect_assets
    file = join_paths(syntect_build_path, a)
    install_data(
        file,
        install_dir: syntect_dir,
    )
endforeach

meson.add_install_script('meson_post_install.py')

run_target(
  'release',
  command: ['scripts/release.sh', meson.project_name(), meson.project_version()]
)

test(
  'cargo test',
  cargo,
  args: 'test',
)