// /// dev.calebs.Hammock1 dbus client proxy

// use anyhow::Result;
// use zbus::dbus_interface;
// use zbus::dbus_proxy;
// use zbus::blocking::{ConnectionBuilder, Connection};

// use crate::app_track::DesktopAppInfo;
// use crate::app_track::TopLevel;

// /// Client D-Bus interface implemented by the root daemon
// /// on the System bus
// #[dbus_proxy(
//     interface = "dev.calebs.Hammock1.AppHandler",
//     default_service = "dev.calebs.Hammock1",
//     default_path = "/dev/calebs/Hammock1/AppHandler"
// )]
// trait AppHandler {
//     /// signals an app launch
//     fn app_launched(&mut self, app_info: &DesktopAppInfo) -> zbus::Result<()>;

//     fn new_top_level(&mut self, toplevel: &TopLevel) -> zbus::Result<()>;

//     fn top_level_changed(&mut self, toplevel: &TopLevel) -> zbus::Result<()>;

//     fn top_level_closed(&mut self, toplevel: &TopLevel) -> zbus::Result<()>;
// }
