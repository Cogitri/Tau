## Changes in v0.8.1

### Feature changes

 - (gxi): set gxi's icon as window icon
 - (gxi): add option to launch new instance

### Bugfixes

 - (gxi): set gettext domain for glade files
 - (i18n): update lang (Dutch)

## Changes in v0.8.0

### Feature changes

 - (gxi-config-storage): add try_* functions for panic-safe access to GSettings
 - (editview): only draw number in linecount on actual line
 - (editview): add line&column to statusbar
 - (editview): redirect EventScroll from linecount to ev_scroll_window
 - (editview): use ScrolledWindow for overlay scrollbars
 - (po): Add nl
 - (gxi|config_storage): save window state
 - (edit_view): use symbolic close button for tabs
 - (macros): add setup_gtk_panic macro

### Bugfixes

 - (editview): remove uneccesary event masks
 - (gxi): set default size instead of setting a size request on window state restoring
 - (editview): avoid rounding errors in cursor positioning
 - (editview): don't scroll down in scroll_to if vadj's page_size is 1
 - (i18n): update lang (Dutch)
 - (i18n): update lang (Norwegian Bokmål)
 - (editview): grab focus of ev_scrolled_window instead of edit_area
 - (edit_view): set Layout size instead of setting Scrollbar's upper
 - (i18n): update PO files
 - (gxi): log with timestamp when RUST_LOg sets a custom loglevel, makes for nicer debugging
 - (gxi): log for all crates in our workspace
 - (edit_view): don't warn on window-{height,maximized,width} GSettings key change
 - (gxi): allow PanicHandler::new to not return Self
 - (gxi-linecache|gxi-peer): derive Default if a new function is present
 - (data): set gettext domain in gschema
 - (main_win): don't include GLADE_SRC file twice
 - (i18n): update lang (German)
 - (i18n): update lang (German)

## Changes in v0.7.0

### Feature changes

 - (edit_view): keybind Shift+Tab to outdent
 - (edit_view): keybind Escape to stopping the current search
 - (edit_view): keybind Ctrl+Backspace to delete_word_backward
 - (main_win): open in existing tab if there's an empty tab
 - (po): Add zh_Hans and zh-Hant to LINGUAS file
 - (po): add zh_Hans to LINGUAS
 - (po): add zh-Hant to LINGUAS
 - (edit_view|prefs_win): support setting a custom tab size

### Bugfixes

 - (ui): remove startup_id property of ApplicationWindow
 - (edit_view): measure FontMetrics in en-US locale
 - (edit_view): use IMContextSimple to fix inserting dead/non latin characters
 - (i18n): update lang (Chinese (Traditional))
 - (i18n): update lang (Chinese (Traditional))
 - (i18n): update lang (Portuguese (Brazil))
 - (i18n): update lang (Norwegian Bokmål)

## Changes in v0.6.2

### Feature changes

 - No new features

### Bugfixes

 - (main): trace log app_id

## Changes in v0.6.1

### Feature changes

 - No new features

### Bugfixes

 - No bugfixes

## Changes in v0.6.0

### Feature changes

 - (rpc): display an ErrorDialog if Xi send 'error'
 - (edit_view): use Popover to make the FindReplace Dialog smaller
 - (edit_view): add regex search option
 - (edit_view): highlight current line
 - (main): if RUST_LOG isn't set, set default log level to Warn
 - (main_win): set Ctrl+W as shortcut for closing the current tab
 - (*): better trace logging

### Bugfixes

 - (edit_view): show close button of the search_bar
 - (edit_view): don't set upper limits during draw/scroll but during update
 - (edit_view): add padding to hadj when scrolling to the right
 - (edit_view): fixup scrolling horizontally when the cursor moves out of the window
 - (ui): don't make FindReplace popover modal
 - (main_win): get_current_edit_view _can_ return no EditView
 - (errors): partially revert 91b8da70556bfab2ccd1b3e4c3d295b5e0fb50e7
 - (edit_view): don't request a specific size for the linecount height
 - (appdata): spelling: on thesave -> in it, It
 - (i18n): update lang (Norwegian Bokmål)
 - (main): only enable Warn logging for gxi itself by default
 - (i18n): update translations
 - (main_win): make actions easier to translate
 - (about_win): update website, fix translator_credits
 - (xi_thread): log errors in sending messages to Xi
 - (edit_view|rpc): use glib::MainContext::channel for cut&copy operations
 - (errors): also display ErrorMsgs on console

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