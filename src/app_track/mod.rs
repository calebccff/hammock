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

// The app tracking code is only used by the user daemon.

// Heavily inspired by https://github.com/ActivityWatch/aw-watcher-window-wayland/blob/master/src/current_window.rs

use anyhow::{Result};
use calloop::{EventLoop};
use log::{debug, trace, warn, info, error};
use wayland_client::backend::ObjectId;
use strum_macros::Display;
use std::fmt;
use zbus::blocking::{Connection as ZbusConnection, fdo::MonitoringProxy};
use zbus::{Result as ZbusResult, dbus_proxy};
use dbus::blocking::Connection as DbusConnection;
use std::sync::{Arc};
use parking_lot::Mutex;
use std::sync::mpsc::{channel, Sender, SyncSender, Receiver, sync_channel};
use std::thread::spawn;
use std::{time::Duration};
use wayland_client::event_created_child;
use crate::events::{HammockEvent, HammockEventSource};
use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::wl_registry::{Event, WlRegistry},
    Connection, Dispatch, Proxy, EventQueue, QueueHandle, WaylandSource,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{
        Event as TopLevelHandleEvent,
        ZwlrForeignToplevelHandleV1 as TopLevelHandle,
    },
    zwlr_foreign_toplevel_manager_v1::{
        Event as TopLevelManagerEvent,
        ZwlrForeignToplevelManagerV1 as TopLevelManager, EVT_TOPLEVEL_OPCODE,
    },
};

use hdbus::HammockDbus;
use wayland::HammockWl;

mod hdbus;
mod wayland;

/// Exports from child modules
pub use wayland::{TopLevel, TopLevelState};
pub use hdbus::DesktopAppInfo;

pub struct AppTrack {
    hwl: HammockWl,
    hdbus: HammockDbus,
}

impl HammockEventSource for AppTrack {
    fn process_pending(&mut self) -> Result<()> {
        self.hdbus.process_pending()?;
        // The wayland event loop is actually it's own thread
        self.hwl.process_pending()
    }
}

impl AppTrack {
    pub fn new(xdg_runtime_dir: &str, wayland_display: &str, tx: &Sender<HammockEvent>) -> Result<Self> {
        Ok(Self {
            hwl: HammockWl::new(xdg_runtime_dir, wayland_display, tx.clone())?,
            hdbus: HammockDbus::new(tx.clone())?,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppId {
    app_id: Option<String>,
}

impl From<String> for AppId {
    fn from(app_id: String) -> Self {
        Self {
            // TODO: validation!
            app_id: Some(app_id),
        }
    }
}

impl fmt::Display for AppId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.app_id {
            Some(ref app_id) => write!(f, "{}", &app_id),
            None => write!(f, "unknown"),
        }
    }
}

