/*
* Hammock
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

use std::ops::Deref;

use crate::events::HammockEvent;
use crate::wayland::{HTopLevel, TopLevelState};
use crate::{cgroups::CGHandler};
use crate::match_rules::{MatchRules};
use crate::application::App;
use parking_lot::Mutex;
use log::{debug};

pub struct Hammock {
    pub rules: MatchRules,
    pub handler: CGHandler,
    pub apps: Mutex<Vec<App>>,
}

impl Hammock {
    /// The main event loop, called every 200ms
    /// or when a new event is received
    pub fn handle_event(&self, event: HammockEvent) {
        match event {
            HammockEvent::NewApplication(app_info) => {
                self.apps.lock().push(App::new(app_info.app_id, app_info.pid));
            },
            HammockEvent::NewTopLevel(top_level) => {
                for app in self.apps.lock().iter() {
                    if app.app_id == top_level.get_app_id() {
                        let cg = match top_level.get_state() {
                            TopLevelState::Activated => self.rules.foreground(),
                            _ => self.rules.background(),
                        };
                        cg.add_app(app.pid);
                    }
                }
            },
            HammockEvent::TopLevelChanged(top_level) => {
                for app in self.apps.lock().iter() {
                    if app.app_id == top_level.get_app_id() {
                        let cg = match top_level.get_state() {
                            TopLevelState::Activated => self.rules.foreground(),
                            _ => self.rules.background(),
                        };
                        debug!("Moving app {}:{} to {}", app.app_id, app.pid, cg.name);
                        cg.add_app(app.pid);
                    }
                }
            },
            HammockEvent::ApplicationClosed(app_id) => {
                let mut apps = self.apps.lock();
                let mut i = 0;
                for app in apps.iter() {
                    if app.app_id == app_id {
                        break;
                    }
                    i += 1;
                }
                apps.remove(i);
            },
        }
    }
}
