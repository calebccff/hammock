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

use anyhow::{Result};
use std::sync::mpsc::channel;

use crate::hammock::Hammock;
use crate::wayland::{HammockWl, HTopLevel, TopLevelState};
use crate::dbus::{HammockDbus, DesktopAppInfo};
use log::{trace};

pub type AppId = String;

#[derive(Debug, Clone)]
pub enum HammockEvent {
    NewApplication(DesktopAppInfo),
    NewTopLevel(HTopLevel),
    ApplicationClosed(AppId),
    TopLevelChanged(HTopLevel),
}

pub struct HammockEventLoop;

impl HammockEventLoop {
    pub fn run(hammock: Hammock, xdg_runtime_dir: &str, wl_display: &str) -> Result<()> {
        let (tx, rx) = channel::<HammockEvent>();
        
        // Spawns a new thread
        let hdbus = HammockDbus::init(tx.clone())?;
        // Takes over the main thread (doesn't return)
        HammockWl::run(xdg_runtime_dir, wl_display, tx, move || {
            hdbus.process_pending();
            while let Ok(event) = rx.try_recv() {
                trace!("Received event: {:?}", event);
                hammock.handle_event(event);
            }
        })
    }
}
