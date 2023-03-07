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
use std::time::Duration;
use crate::hammock::Hammock;
use crate::app_track::{AppTrack, AppId, TopLevel, TopLevelState, DesktopAppInfo};
use log::{trace, error};

#[derive(Debug, Clone)]
pub enum HammockEvent {
    NewApplication(DesktopAppInfo),
    NewTopLevel(TopLevel),
    ApplicationClosed(AppId),
    TopLevelChanged(TopLevel),
}

pub struct HammockEventLoop;

impl HammockEventLoop {
    pub fn run_root(hammock: Hammock) -> Result<()> {
        Ok(())
    }
}

/// All event sources must implement this trait.
/// Event sources are responsible to handling any pending events
/// and propagating them to any child event sources.
/// They must return either the time it took to process, or
/// an error which will cause the loop to exit.
pub trait HammockEventSource {
    fn process_pending(&mut self) -> Result<()>;
}

