scalable_dir = join_paths('hicolor', 'scalable', 'apps')

install_data(
  join_paths(scalable_dir, 'org.gnome.Tau.svg'),
  install_dir: join_paths(tau_datadir, 'icons', scalable_dir),
  rename: '@0@.svg'.format(application_id)
)

symbolic_dir = join_paths('hicolor', 'symbolic', 'apps')

install_data(
  join_paths(symbolic_dir, 'org.gnome.Tau-symbolic.svg'),
  install_dir: join_paths(tau_datadir, 'icons', symbolic_dir),
  rename: '@0@-symbolic.svg'.format(application_id)
)
