## Changes in v0.11.0

### Feature changes

 - (tau): add settings for terminal to preferences window
 - (tau): add terminals to tabs
 - (tau): Allow setting a custom shell
 - (tau): add terminal
 - (tau): use libhandy widgets for syntax configuration
 - (tau): make tab&space drawing preferences more friendly on mobile
 - (tau): use HdyActionRow instead of HdyPrefenrecesRow where appropriate
 - (tau): use GtkSwitch instead of GtkCheckButton
 - (tau): squash preferences into two pages

### Bugfixes

 - (tau): fix numerous typos
 - (editview): fix crashing on negative end_index
 - (editview): redraw upon changing syntax or tab size
 - (tau): only display the syntect warning if syntect is not available
 - (tau): fix appearance of syntax configuration in PrefsWin
 - (tau): fix setting margin_spinbutton's sensitivity upon flipping margin_switch
 - (tau): use consistent capitalisation in titles
 - (tau): Always use actually activatable widget for HdyActionRow
 - (tau): fix hiding warning about syntect missing  when syntect is available
 - (tau): fix alignment on syntect warnings in PrefsWin
 - (tau): disable GtkNotebook's border
 - (tau): sync titlebar properly on tab switch

## Changes in v0.10.2

### Feature changes

 - (tau): add missing keybinds to GtkShortcutWindow
 - (edit_view): Full title as tooltip and in titlebar

### Bugfixes

 - (editview): fix redo keybind
 - (tau): don't start drawing the background at negative X's
 - (tau): don't use the now removed org.gnome.TauDevel gschema
 - (tau): if we already have a filename, set this as default for save_as
 - (tau): ask for confirmation if user attempts to overwrite existing file in save_as

## Changes in v0.10.1

### Feature changes

 - No new features

### Bugfixes

 - (tau): remove typo in libhandy ui and must_use warnings

## Changes in v0.10.0

### Feature changes

 - (tau): Option to restore session. Keep track of open files. Add option to restore session on startup instead of opening empty file. Remove error message when changing autosave option. fixes #59
 - (tau): Add `save as` accelerator Show `save as` window on `Ctrl+Shift+S`
 - (tau): Add fullscreen mode
 - (tau): Add find-all option
 - (tau): Add tab history and shortcuts to cycle through it

### Bugfixes

 - (tau): default to current EditView's parent path when opening files fixes #380
 - (editview): Fix invisibles position and thickness Make space radius relative to line height, to keep them visible for narrow spaces. Drawing of cursor and invisibles adjusts with horizontal scroll
 - (tau): better handle bad GSettings configuration
 - (editview): make invisibles more transparent, let spaces scale with line height
 - (tau): instead of blocking on the ViewId in 'new_view' spawn a future
 - (tau): use same channel for all XiEvents (including new_view events)
 - (tau): add missing `class` attribute to prefs_win_handy, fixing crash when opening it
 - (tau): add `context` attributes to translatable strings in the shortcut window
 - (editview): fix AtkObject::accessible-role enum names
 - (tau): Close all search dialogs when switching tabs
 - (editview): do not misplace invisibles drawn on selection
 - (i18n): assorted translatable string improvements
 - (i18n): update lang (Portuguese (Portugal))
 - (i18n): don't translate strings which aren't user facing
 - (i18n): add translator comments to the .desktop file
 - (i18n): update POTFILES.in
 - (i18n): conform .po file names to glibc locales

## Changes in v0.9.3

### Feature changes

 - (tau): add font increase/decrease shortcuts
 - (tau): auto-save when window loses focus
 - (tau): display title of current document in GtkHeaderBar
 - (tau): only display tabs if more than 1 documents are opened
 - (editview): show invisibles on selection
 - (tau): new icon design, by Tobias Bernard (@bertob)
 - (i18n): create es translation
 - (tau): use HdyPreferencesWindow for PrefsWin for mobile usability

### Bugfixes

 - (edit_view): redraw linecount when we scroll up/down the ScrolledWindow
 - (i18n): add missing source files to POTFILES.in
 - (tau): introduce maximal tab length
 - (tau): set title to "Tau" instead of last opened document when closing all tabs
 - (tau): open new tab without closing empty ones
 - (i18n): add missing langs to LINGUAS
 - (i18n): update lang (Spanish)
 - (i18n): update lang (Chinese (Simplified))
 - (editview): Apply pange style attributes properly
 - (editview): use `window-close-symbolic` icon for closing editview tabs
 - (i18n): update lang (French)

## Changes in v0.9.2

### Feature changes

 - (tau): make tabs reorderable via mouse drag

### Bugfixes

 - (editview): honour style's fore-&background alpha
 - (tau): actually enable gtk_v3_22 for editview if enabled for tau
 - (tau): copy selected text into primary clipboard
 - (editview): fix cursor position if last char of line is more than one byte long
 - (editview): fix line indicator in statusbar if document contains broken lines
 - (i18n): update lang (French)
 - (tau): replace arrow labels in status bar with symbolic variants
 - (tau): show user ErrorDialog when opening file fails
 - (editview): don't display trailing space if following line is soft broken
 - (tau): GtkNotebook shouldn't be possible to focus
 - (tau): do not reserve space for subtitle in headerbar
 - (editview): grab focus upon creation
 - (tau): do not send click on drag even start
 - (editview): remove the frame from EditView's ScrolledWindow
 - (tau): change page to the to be saved EditView in MainWin::save_as
 - (tau): fix 'Save All' button
 - (tau): don't offer Quit/Close (All) in primary menu
 - (tau): don't block in MainWin::req_new_view
 - (tau): use standard names for Keyboard Shortcuts and About <application> in app menu

## Changes in v0.9.1

### Feature changes

 - No new features

### Bugfixes

 - (editview): Fix dragging for some users
 - (meson): use full path to xi-core when using system xi-core

## Changes in v0.9.0

### Feature changes

 - (tau): Set default_tab_size for EditView if the syntax defines it
 - (editview): add UI to set tab_size,auto_indent and insert_spaces per EditView
 - (i18n): create sv translation
 - (tau): add per-syntax configuration for tab-size and insert-spaces
 - (tau): use Ctrl+G/Ctrl+Shift+G shortcut for find_next/find_prev
 - (tau): add shortcuts win
 - (i18n): create bn translation
 - (editview): add a context menu when doing a right click
 - (editview): support toggling cursor visibility
 - (editview|tau): close tabs upon middle-clicking on it
 - (po): add French
 - (gxi): add custom css to make scrollbar smaller
 - (editview|gxi): draw tabs/spaces with cairo instead of just replacing their symbols
 - (editview|gxi): reintroduce capability to draw trailing tabs
 - (editview): update LineCache in another thread

### Bugfixes

 - (editview): Inhibit in connect_motion_notify_event, fixing mouse movement after dragging
 - (tau): don't block on getting view_id in connect_open, fixes opening multiple files
 - (editview): use Line's 'line_num' field for determing the first linenumber in linecount
 - (editview): call EditView::update_visible_scroll_region in EditView::update
 - (tau): fix setting the values of insert_spaces/tab_size buttons for the initial syntax
 - (tau): Make it possible to unset syntax configs or only partially set them
 - (tau): call WindowExt::close() instead of Window::destroy() in "app.quit"
 - (tau): don't use the same Adjustment for general and syntax tab_size spinbutton
 - (i18n): update lang (Chinese (Simplified))
 - (i18n): update lang (Chinese (Traditional))
 - (i18n): update lang (Chinese (Simplified))
 - (i18n): update lang (Chinese (Simplified))
 - (tau): fix BorrowMut crash in MainWin::handle_save_button
 - (tau): reset GSettings keys if they contain bad values
 - (tau): set ranges on window-{width,height}, tab-size&column-right-margin in GSchema
 - (tau): reset theme name in GSettings too when previous theme isn't available
 - (editview): only send find_prev/find_next msg to xi-core if we're in search mode
 - (tau): print a better error msg if we can't find the xi-core binary
 - (tau): allow not highlighting spaces&tabs
 - (i18n): update lang (Norwegian Bokmål)
 - (editview): set syntax_label's text when creating an EditView too
 - (tau): add padding to syntax selection to center it
 - (i18n): update lang (Chinese (Simplified))
 - (i18n): update lang (Chinese (Traditional))
 - (i18n): update lang (Dutch)
 - (i18n): update lang (Norwegian Bokmål)
 - (i18n): update lang (Dutch)
 - (editview): Pango uses byte-index for characters, not char index!
 - (editview): pay attention to newlines on trailing tabs/spaces
 - (editview): scroll to the entire length of the EditView
 - (editview): call `EditView::update_visible_scroll_region` when vadj's value changes
 - (i18n): update lang (French)
 - (i18n): update lang (German)
 - (i18n): update lang (Dutch)
 - (i18n): update lang (German)
 - (i18n): update lang (Chinese (Traditional))
 - (i18n): update lang (Chinese (Simplified))
 - (i18n): update lang (German)
 - (po): apply rename to Tau and regen
 - (gxi): upon exiting gxi also shutdown the tokio runtime
 - (editview): don't block on copy/cut
 - (editview): do modify selection on find_{next,pref}
 - (gxi): don't register resource twice
 - (gxi): fix rebuilding when env variables change
 - (editview): fix last_line detection in update_visibile_scroll_region
 - (editview): avoid looping between languaged_changed and set_language
 - (gxi|editview): don't assume avail_languages is set in stone after creation of EditView
 - (editview): disable lang selection if there are no langs available
 - (prefs_win): disable word-wrap for now
 - (gxi): make the channel for creating EditViews high priority

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