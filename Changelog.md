## Changes in v0.5.5

### Feature changes

 - (main_win): switch to respective EditView upon asking if it should be saved
 - (main_win): use different Dialog style for ask_save_dialog
 - (theme): use darker version of ThemeSettings::background for right hand margin
 - (edit_view|pref_{storage,view}): implement right hand margin

### Bugfixes

 - (i18n): update translations
 - (po): add nb_NO to LINGUAS
 - (main_win): fix pressing the cancel button on the ask_save_dialog
 - (prefs_win): redraw EditView's edit_area when changing right hand margin
 - (edit_view): center linecount's font
 - (edit_view): instead of a different color for linecount add more padding
 - (edit_view): only queue linecount draw if actually necessary
 - (pref_storage): return config_dir as String in Config::new()

## Changes in v0.5.4

### Feature changes

 - (i18n): create nb_NO translation
 - (edit_view): open/close find&replace dialog upon triggering action again
 - (about_win): display icon and translator credit
 - (edit_view): make linecount more consistent

### Bugfixes

 - (appdata): fix typo in 0.5.0 changelog
 - (*): use ellipsis (…) instead of three dots (...)
 - (i18n): update lang Norwegian Bokmål
 - (i18n): update lang Portuguese (Brazil)
 - (edit_view): linecount is its own widget; don't include it in x/y -> cell calculations
 - (edit_view): don't try to add Searchbar to two boxes
 - (i18n): update translations
 - (edit_view): fix drawing spaces after tabs
 - (edit_view): fix horizotal scrollbar width on empty document
 - (edit_view): fix linecount placement upon opening search/replace
 - (edit_view): intialize EditView with font set by Config
 - (edit_view): fix font size calculation on font changed
 - (prefs_win): Remove unsupported properties
 - (data): resize icons to 128x128
 - (build): remove rust-target option
 - (edit_view): redraw upon font changes
 - (edit_view): fix width of linecount for numbers >=100

## Changes in v0.5.3

### Feature changes

 - (main_win): ask user if unsaved changes should be save upon closing
 - (main): use human_panic for better panic output

### Bugfixes

 - (main_win): prefix params with an underscore in plugin_started
 - (build): set plugin_dir to '/usr/local/lib/gxi/plugins' by default
 - (main_win): set title for ask_save_dialog
 - (edit_view): fix line numbers upon scrolling

## Changes in v0.5.2

### Feature changes

 - (data): new icon
 - (main_win): notify the user if a plugin isn't available or has crashed

### Bugfixes

 - (data): add drop shadow to icon
 - (data): less grey, more white in icon
 - (edit_view): fix line numbers upon deleting lines
 - (i18n): update translations
 - (linecache): don't assume we always have at least one line in the linecache
 - (i18n): update translations

## Changes in v0.5.1

### Feature changes

 - No new features

### Bugfixes

 - (meson): fix build with appstream-utils

## Changes in v0.5.0

### Feature changes

 - (shared_queue): more verbose trace logging
 - (main_win): use a thread to handle CoreMsgs instead of add_idle
 - (main): display error window if xi-editor crashes
 - (edit_view): add newline to end of the file if it doesn't terminate with one
 - (edit_view): only draw trailing spaces
 - (shared_queue): also use for sending stuff to xi
 - (gettext): build against system gettext

### Bugfixes

 - (prefs_win): use pango::SCALE instead of hardcoding 1024
 - (edit_view): we don't ship the Inconsolata font anymore
 - (main): set config dir correctly
 - (prefs_win): fix choosing font size
 - (edit_view): fix scrollbar adjustment
 - (main_win): better CoreMsg trace msgs
 - (pref_storage): don't implement Clone for Config<T>
 - (main): fix loading config
 - (main): don't load config twice
 - (linecache): fix linecount for wrapped lines
 - (ui): re-enable word wrapping, it's pretty complete now
 - (pref_storage): DO NOT clone Config to make sure it's consistent across different windows
 - (edit_view): send 'resize' to xi upon allocating a new size
 - (rpc): correctly handle measure_width
 - (meson): only validate appstream if appstream-util is recent enough