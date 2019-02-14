
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
