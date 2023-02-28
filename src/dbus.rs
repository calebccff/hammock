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

use crate::events::HammockEvent;
use anyhow::anyhow;
use anyhow::{bail, Result};
use calloop::{EventLoop, LoopHandle};
use dbus::blocking::Connection;
use dbus::channel::Channel;
use dbus::channel::MatchingReceiver;
use dbus::message::{Message, MatchRule};
use log::{debug, info, trace, warn};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread::spawn;
use std::{process, time::Duration};
use wayland_client::backend::ObjectId;
use wayland_client::event_created_child;

pub struct HammockDbus {
    conn: Connection,
}

impl HammockDbus {
    pub fn init(tx: Sender<HammockEvent>) -> Result<Self> {
        // Requires DBUS_SESSION_BUS_ADDRESS to be set
        let c = match Connection::new_session() {
            Ok(c) => c,
            Err(e) => {
                bail!("Failed to connect to DBUS session bus, is DBUS_SESSION_BUS_ADDRESS_SET? (you need to fetch it from the user session): {}", e);
            }
        };

        let mut rule = MatchRule::new();
        // We want to know about app launches
        rule.interface = Some("org.gtk.gio.DesktopAppInfo".into());
        rule.member = Some("Launched".into());

        let proxy = c.with_proxy(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            Duration::from_millis(2000),
        );
        proxy.method_call(
            "org.freedesktop.DBus.Monitoring",
            "BecomeMonitor",
            (vec![rule.match_str()], 0u32),
        )?;

        c.start_receive(
            rule,
            Box::new(move |msg, _| {
                trace!("Received DBUS message: {:?}", msg);
                match tx.send(HammockEvent::NewApplication(DesktopAppInfo::new(&msg))) {
                    Ok(_) => true,
                    Err(e) => {
                        warn!("Failed to send DBUS message to event loop: {}", e);
                        false
                    }
                }
            }),
        );

        info!("DBUS: registered with session bus");

        Ok(Self { conn: c })
    }

    pub fn process_pending(&self) -> Result<bool> {
        match self.conn.process(Duration::from_millis(0)) {
            Ok(true) => Ok(true),
            Ok(false) => Ok(false),
            Err(e) => Err(anyhow!("Failed to process DBUS messages: {}", e)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DesktopAppInfo {
    pub app_id: String,
    pub pid: u32,
    /// desktop file path ?
    pub path: Option<String>,
}

impl DesktopAppInfo {
    pub fn new(msg: &Message) -> Self {
        Self { app_id: "".into(), pid: 0, path: None }
    }
}
