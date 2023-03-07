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

use crate::events::{HammockEvent, HammockEventSource};
use super::AppId;
use anyhow::anyhow;
use anyhow::{bail, Result};
use dbus::blocking::Connection;
use dbus::channel::{Channel, MatchingReceiver};
use dbus::message::{Message, MatchRule};
use log::{debug, info, trace, warn, error};
use std::sync::mpsc::{Sender};
use std::{time::Duration};


pub(super) struct HammockDbus {
    connection: Connection,
}

impl HammockDbus {
    pub(super) fn new(tx: Sender<HammockEvent>) -> Result<Self> {
        // Requires DBUS_SESSION_BUS_ADDRESS to be set
        let address = std::env::var("DBUS_SESSION_BUS_ADDRESS")
            .map_err(|_| anyhow!("[DBUS] DBUS_SESSION_BUS_ADDRESS not set"))?;
        debug!("[DBUS] Connecting to session bus");
        let mut conn = match Connection::new_session() {
            Ok(c) => c,
            Err(e) => {
                bail!("[DBUS] Failed to connect to DBUS session bus, is DBUS_SESSION_BUS_ADDRESS_SET? (you need to fetch it from the user session): {}", e);
            }
        };

        let mut rule = MatchRule::new();
        // We want to know about app launches
        rule.interface = Some("org.gtk.gio.DesktopAppInfo".into());
        rule.member = Some("Launched".into());

        let proxy = conn.with_proxy("org.freedesktop.DBus", "/org/freedesktop/DBus", Duration::from_millis(5000));
        let result: Result<(), dbus::Error> = proxy.method_call("org.freedesktop.DBus.Monitoring", "BecomeMonitor", (vec!(rule.match_str()), 0u32));

        if result.is_ok() {
            // Start matching using new scheme
            conn.start_receive(rule, Box::new(move |msg, _| {
                Self::handle_message(&tx.clone(), &msg);
                true
            }));
        } else {
            // Start matching using old scheme
            rule.eavesdrop = true; // this lets us eavesdrop on *all* session messages, not just ours
            conn.add_match(rule, move |_: (), _, msg| {
                Self::handle_message(&tx.clone(), &msg);
                true
            }).expect("add_match failed");
        }

        Ok(Self {
            connection: conn,
        })
    }

    fn handle_message(tx: &Sender<HammockEvent>, msg: &Message) {
        //trace!("[DBUS] Received DBUS message: {:?}", msg);
        let (path, pid) = match msg.get3::<Vec<u8>, String, i64>() {
            (Some(path), _, Some(pid)) => ({
                let s = path.iter().map(|&c| c as char).collect::<String>();
                s[..s.len() - 1].to_string()
            }, pid.try_into().unwrap()),
            _ => {
                warn!("[DBUS] Failed to parse DBUS message");
                return;
            }
        };

        let app_id = match path.split("/").last() {
            Some(app_id) => {
                let off = app_id.find(".desktop").unwrap_or(app_id.len());
                app_id[..off].to_string()
        },
            None => {
                error!("[DBUS] Failed to parse DBUS message: {:?}", msg);
                return;
            }
        };

        debug!("[DBUS] New application launched: {} (pid: {}, path: {})", &app_id, pid, path);
        match tx.send(HammockEvent::NewApplication(DesktopAppInfo::new(app_id, pid, path))) {
            Ok(_) => true,
            Err(e) => {
                warn!("[DBUS] Failed to send DBUS message to event loop: {}", e);
                false
            }
        };
    }
}

impl HammockEventSource for HammockDbus {
    fn process_pending(&mut self) -> Result<()> {
        match self.connection.process(Duration::from_millis(0)) {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(anyhow!("[DBUS] Failed to process DBUS messages: {}", e))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct DesktopAppInfo {
    app_id: AppId,
    pid: u64,
    /// desktop file path ?
    path: String,
}

impl DesktopAppInfo {
    fn new(app_id: String, pid: u64, path: String) -> Self {
        Self { app_id: app_id.into(), pid, path }
    }
}
