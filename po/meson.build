i18n = import('i18n')

i18n.gettext(meson.project_name(),
             args: ['--keyword=gettext', '--keyword=ngettext', '--keyword=pgettext', '--keyword=npgettext',
                    '--keyword=ngettext:1,2', '--keyword=pgettext:1c,2', '--keyword=pngettext:1c,2,3',
                    ],
             preset: 'glib',
             install: true,
)
