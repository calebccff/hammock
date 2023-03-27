use anyhow::Result;
use zbus::dbus_interface;
use zbus::dbus_proxy;
use zbus::blocking::{ConnectionBuilder, Connection};

use crate::app_track::DesktopAppInfo;
use crate::app_track::TopLevel;

// Client D-Bus interface implemented by the root daemon
// on the System bus
// #[dbus_proxy(
//     interface = "org.freedesktop.systemd1",
//     default_service = "org/freedesktop/systemd1",
//     default_path = "/org/freedesktop/systemd1/Manager"
// )]
// trait AppHandler {
//     /// signals an app launch
//     fn start_transient_unit(&mut self, unit_name: String, mode: String, properties: ) -> zbus::Result<()>;

// }

// enum StartTransientUnityProperty {
    
// }
