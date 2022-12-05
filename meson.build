project('hammock', 'c', 'cpp', version: '0.0.1')
cc = meson.get_compiler('c')
fs = import('fs')

wayland_protos = dependency('wayland-protocols') # for protocols
wayland_client = dependency('wayland-client')

libcas_dep = dependency('libcas', required : false)

if not libcas_dep.found()
  libcas_proj = subproject('libcas')
  libcas_dep = libcas_proj.get_variable('libcas_dep')
endif

subdir('protocol')

hammock_deps = [
  libcas_dep,
  client_protos, # wayland-client + extra protocols
]

subdir('hammock')

hammock_include = [
  include_directories('.'),
]

hammock_headers = [
]

hammock_src = [
  'hammockd.c',
]

hammockd = executable('hammockd', hammock_src,
  sources: hammock_src,
  include_directories: hammock_include,
  dependencies: hammock_deps,
  install: true,
)
