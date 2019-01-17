project(
  'gxi', 'rust',
  version: '0.3.1',
  license: 'MIT',
)

dependency('glib-2.0', version: '>= 2.56')
dependency('gio-2.0', version: '>= 2.56')
dependency('cairo', version: '>= 1.16')
dependency('gtk+-3.0', version: '>= 3.20')
# required for syntect-plugin: onig_sys
find_program('cmake', required: true)

cargo = find_program('cargo', required: true)
cargo_vendor = find_program('cargo-vendor', required: false)

gxi_prefix = get_option('prefix')
gxi_bindir = join_paths(gxi_prefix, get_option('bindir'))
gxi_datadir = get_option('datadir')
gxi_plugin_dir = join_paths(gxi_prefix, get_option('libdir'), 'gxi', 'plugins')

cargo = find_program('cargo', required: false)
gresource = find_program('glib-compile-resources', required: false)
cargo_vendor = find_program('cargo-vendor', required: false)
cargo_script = find_program('scripts/cargo.sh')
grabber = find_program('scripts/grabber.sh')

c = run_command(grabber)
sources = c.stdout().strip().split('\n')

custom_target('cargo-build-gxi',
                        input: sources,
                        output: ['gxi'],
                        install: true,
                        install_dir: gxi_bindir,
                        console: true,
                        command: [cargo_script,
                                  '@CURRENT_SOURCE_DIR@',
                                  '@OUTPUT@',
                                  '@CURRENT_SOURCE_DIR@',
                                  'gxi',
                                  gxi_plugin_dir,
                                 ])

syntect_build_path = join_paths(meson.current_source_dir(), 'vendor', 'xi-editor', 'rust', 'syntect-plugin')
syntect_dir = join_paths(gxi_plugin_dir, 'syntect')
syntect_bindir = join_paths(syntect_dir, 'bin')
syntect_releasebin_path = join_paths(meson.current_source_dir(), 'vendor', 'xi-editor', 'rust')

custom_target('cargo-build-syntect',
                        input: sources,
                        output: ['xi-syntect-plugin'],
                        install: true,
                        install_dir: syntect_bindir,
                        console: true,
                        command: [cargo_script,
                                  syntect_build_path,
                                  '@OUTPUT@',
                                  syntect_releasebin_path,
                                  'xi-syntect-plugin',
                                 ])

synctect_assets = ['assets/Makefile.toml', 'assets/YAML.toml', 'manifest.toml']

foreach a: synctect_assets
    file = join_paths(syntect_build_path, a)
    install_data(
        file,
        install_dir: syntect_dir,
    )
endforeach