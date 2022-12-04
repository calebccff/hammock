
libhammock_include = [
  include_directories('.'),
]

libhammock_headers = [
  'hammock.h',
]

libhammock_src = [
  'hammock.c',
]

hammock = shared_library('hammock', libhammock_src,
  sources: libhammock_src,
  include_directories: libhammock_include,
  dependencies: hammock_deps,
  install: true,
)

install_headers(libhammock_headers, subdir: 'hammock')
libhammock_dep = declare_dependency(
  include_directories: libhammock_include,
  link_with: hammock,
)

hammock_deps += libhammock_dep
