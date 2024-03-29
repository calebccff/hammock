/*
* Hammock system daemon
* Copyright (C) 2022 Caleb Connolly <caleb@connolly.tech>
*
* This program is free software; you can redistribute it and/or modify
* it under the terms of the GNU General Public License as published by
* the Free Software Foundation; either version 2 of the License, or
* (at your option) any later version.
*
* This program is distributed in the hope that it will be useful,
* but WITHOUT ANY WARRANTY; without even the implied warranty of
* MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
* GNU General Public License for more details.
*
* You should have received a copy of the GNU General Public License along
* with this program; if not, write to the Free Software Foundation, Inc.,
* 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
*/

use super::AppId;
use crate::events::{HammockEvent, HammockEventSource};
use anyhow::anyhow;
use anyhow::{bail, Result};
use dbus::blocking::{Proxy, Connection};
use dbus::channel::MatchingReceiver;
use dbus::message::{MatchRule, Message};
use dbus::arg::OwnedFd;
use serde::de::Visitor;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::sync::mpsc::Sender;
use std::time::Duration;
use std::os::raw::c_int;
use parking_lot::Mutex;
use std::sync::Arc;
use std::thread;

struct InhibitHandler {
    fd: Option<OwnedFd>,
}

pub(super) struct HammockDbus {
    connection: Connection,
    sys_conn: Connection, // System bus
    inhib: Arc<Mutex<InhibitHandler>>,
}

impl HammockDbus {
    pub(super) fn new(tx: Sender<HammockEvent>) -> Result<Self> {
        // Requires DBUS_SESSION_BUS_ADDRESS to be set
        let _address = std::env::var("DBUS_SESSION_BUS_ADDRESS")
            .map_err(|_| anyhow!("DBUS_SESSION_BUS_ADDRESS not set"))?;
        debug!("Connecting to session bus");
        let conn = match Connection::new_session() {
            Ok(c) => c,
            Err(e) => {
                bail!("Failed to connect to DBUS session bus, is DBUS_SESSION_BUS_ADDRESS_SET? (you need to fetch it from the user session): {}", e);
            }
        };

        let mut gio_launched_rule = MatchRule::new();
        // We want to know about app launches
        gio_launched_rule.interface = Some("org.gtk.gio.DesktopAppInfo".into());
        gio_launched_rule.member = Some("Launched".into());
        gio_launched_rule.eavesdrop = true;

        let proxy = conn.with_proxy(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            Duration::from_millis(100),
        );

        let _: Result<(), dbus::Error> = proxy.method_call(
            "org.freedesktop.DBus.Monitoring",
            "BecomeMonitor",
            (vec![gio_launched_rule.match_str()], 0u32),
        );

        let txc = tx.clone();

        conn.start_receive(
            gio_launched_rule,
            Box::new(move |msg, _| {
                Self::handle_launched(&tx, &msg);
                true
            }),
        );

        debug!("Connecting to system bus");
        let sys_conn = match Connection::new_system() {
            Ok(c) => c,
            Err(e) => {
                bail!("Failed to connect to DBUS system bus: {}", e);
            }
        };

        let mut inhibit_rule = MatchRule::new();
        inhibit_rule.interface = Some("org.freedesktop.login1.Manager".into());
        inhibit_rule.member = Some("PrepareForSleep".into());
        inhibit_rule.eavesdrop = true;

        let inhib = InhibitHandler::new(&sys_conn)?;

        let proxy = sys_conn.with_proxy(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            Duration::from_millis(100),
        );

        let _: Result<(), dbus::Error> = proxy.method_call(
            "org.freedesktop.DBus.Monitoring",
            "BecomeMonitor",
            (vec![inhibit_rule.match_str()], 0u32),
        );

        sys_conn.start_receive(inhibit_rule,
            Box::new(move |msg, _| {
                let active = match msg.get1::<bool>() {
                    Some(active) => active,
                    None => {
                        warn!("Failed to parse DBUS message");
                        return true;
                    }
                };
                if let Err(e) = txc.send(HammockEvent::SystemSuspend(active)) {
                    error!("Failed to send event: {}", e);
                }
                true
            }),
        );

        debug!("Connected to DBUS");
        Ok(Self { connection: conn, sys_conn, inhib: Arc::new(Mutex::new(inhib)) })
    }

    fn handle_launched(tx: &Sender<HammockEvent>, msg: &Message) {
        //trace!("Received DBUS message: {:?}", msg);
        let (path, pid) = match msg.get3::<Vec<u8>, String, i64>() {
            (Some(path), _, Some(pid)) => (
                {
                    let s = path.iter().map(|&c| c as char).collect::<String>();
                    s[..s.len() - 1].to_string()
                },
                pid.try_into().unwrap(),
            ),
            _ => {
                warn!("Failed to parse DBUS message");
                return;
            }
        };

        let app_id = match path.split('/').last() {
            Some(app_id) => {
                let off = app_id.find(".desktop").unwrap_or(app_id.len());
                app_id[..off].to_string()
            }
            None => {
                error!("Failed to parse DBUS message: {:?}", msg);
                return;
            }
        };

        debug!(
            "New application launched: {} (pid: {}, path: {})",
            &app_id, pid, path
        );
        match tx.send(HammockEvent::NewApplication(DesktopAppInfo::new(
            app_id, pid, path,
        ))) {
            Ok(_) => true,
            Err(e) => {
                warn!("Failed to send DBUS message to event loop: {}", e);
                false
            }
        };
    }

    pub(super) fn handle_suspend(&self, active: bool) -> Result<()> {
        let mut inhib = self.inhib.lock();
        if active {
            inhib.onSuspend();
            Ok(())
        } else {
            inhib.onResume(&self.connection)
        }
    }

    // pub(super) fn start(&self) {
    //     std::thread::spawn(|| {
    //         loop {
    //             self.connection
    //                 .process(Duration::from_millis(100));
    //             self.sys_conn
    //                 .process(Duration::from_millis(100))
    //                 .unwrap();
    //         }
    //         Ok(())
    //     });
    // }
}

impl InhibitHandler {
    fn new(conn: &Connection) -> Result<Self> {
        let proxy = conn.with_proxy(
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            Duration::from_millis(1000),
        );

        let (fd,) = proxy.method_call("org.freedesktop.login1.Manager", "Inhibit", ("sleep", "Hammock", "Freeze gnome-session", "delay"))?;

        Ok(Self {
            fd: Some(fd),
        })
    }
    
    fn onSuspend(&mut self) {
        self.fd.take();
    }

    fn onResume(&mut self, conn: &Connection) -> Result<()> {
        let proxy = conn.with_proxy(
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            Duration::from_millis(1000),
        );

        let (fd,) = proxy.method_call("org.freedesktop.login1.Manager", "Inhibit", ("sleep", "Hammock", "Freeze gnome-session", "delay"))?;

        self.fd.replace(fd);

        Ok(())
    }
}

impl HammockEventSource for HammockDbus {
    fn process_pending(&mut self) -> Result<()> {
        match self.connection.process(Duration::from_millis(0)) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!("Failed to process DBUS messages: {}", e)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopAppInfo {
    app_id: AppId,
    pid: u64,
    /// desktop file path ?
    path: String,
}

impl DesktopAppInfo {
    fn new(app_id: String, pid: u64, path: String) -> Self {
        Self {
            app_id: app_id.into(),
            pid,
            path,
        }
    }

    pub fn app_id(&self) -> AppId {
        self.app_id.clone()
    }

    pub fn pid(&self) -> u64 {
        self.pid
    }
}

// impl Serialize for DesktopAppInfo {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let mut state = serializer.serialize_struct("DesktopAppInfo", 3)?;
//         state.serialize_field("app_id", &self.app_id)?;
//         state.serialize_field("pid", &self.pid)?;
//         state.serialize_field("path", &self.path)?;
//         state.end()
//     }
// }

// impl<'de> Deserialize<'de> for DesktopAppInfo {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         let fields = ["app_id", "pid", "path"];
//         let mut deserializer = deserializer.deserialize_struct("DesktopAppInfo", &fields, Visitor)?;
//         let app_id = deserializer.deserialize_field(&mut deserializer, "app_id")?;
//         let pid = deserializer.deserialize_field(&mut deserializer, "pid")?;
//         let path = deserializer.deserialize_field(&mut deserializer, "path")?;
//     }
// }
