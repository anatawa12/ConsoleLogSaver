project('c-rewrite', 'c',
        version : '1.0.0',
        default_options : ['warning_level=3', 'c_std=c11'])

libui_proj = subproject('libui')
libui_dep = libui_proj.get_variable('libui_dep')

c_rewrite = executable(
        'c_rewrite',
        'main.c',
        dependencies : libui_dep,
        install : true
)

test('test', c_rewrite)
