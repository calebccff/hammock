/*
* Hammock user daemon
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
/// This module contains the user daemon, which is responsible for
/// tracking applications and notifying the system daemon of changes.

use anyhow::Result;
use crate::app_track::AppTrack;
use crate::events::HammockEvent;
use crate::hammock::Hammock;
use crate::events::HammockEventSource;
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn run(hammock: Hammock, xdg_runtime_dir: &str, wl_display: &str) -> Result<()> {
    let (tx, rx) = channel::<HammockEvent>();
    let mut app_track = AppTrack::new(xdg_runtime_dir, wl_display, &tx)?;

    loop {
        let start = std::time::Instant::now();
        app_track.process_pending()?;
        while let Ok(event) = rx.try_recv() {
            trace!("Received event: {}", event);
            hammock.handle_event(event)?;
        }
        let elapsed = start.elapsed();
        //trace!("Event loop took {}ms", elapsed.as_millis());
        let sleep_time = if elapsed > Duration::from_millis(200) {
            trace!("Event loop took {}ms!!!", elapsed.as_millis());
            Duration::from_millis(0)
        } else {
            Duration::from_millis(200) - elapsed
        };
        std::thread::sleep(sleep_time);
    }
}
