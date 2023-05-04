# Hammock: A power optimisation framework for Linux Mobile devices

* Manage app lifetime
* Suspend background applications
* Permissions integration to stay awake, run in background etc
* Push notifications

Hammock currently only supports Phosh on postmarketOS, with the following
patches being required:

* https://gitlab.freedesktop.org/calebccff/dbus/-/commit/9c229750e5da68f379b987fbea86022d59e21124
* https://gitlab.gnome.org/calebccff/phoc/-/commit/2a5068cadf0667fae8a87378ebeb6c96187741a6

It will configure a cgroup per application and use app state tracking via
Wayland to freeze apps that aren't in focus. See the example configuration `docs/config.default.yaml` for more information.
