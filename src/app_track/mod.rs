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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::mpsc::Sender;
use crate::events::{HammockEvent, HammockEventSource};

use hdbus::HammockDbus;
use wayland::HammockWl;

mod hdbus;
mod wayland;

pub use hdbus::DesktopAppInfo;
/// Exports from child modules
pub use wayland::{TopLevel, TopLevelInner, TopLevelState};

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
    pub fn new(
        xdg_runtime_dir: &str,
        wayland_display: &str,
        tx: &Sender<HammockEvent>,
    ) -> Result<Self> {
        Ok(Self {
            hwl: HammockWl::new(xdg_runtime_dir, wayland_display, tx.clone())?,
            hdbus: HammockDbus::new(tx.clone())?,
        })
    }

    pub fn handle_suspend(&self, active: bool) -> Result<()> {
        self.hdbus.handle_suspend(active) // Need to freeze GSD or something :Sob:
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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
