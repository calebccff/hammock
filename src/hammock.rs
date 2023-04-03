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

use crate::app_track::{TopLevelState, AppId};
use crate::application::App;
use crate::cgroups::CGHandler;
use crate::events::HammockEvent;
use crate::match_rules::MatchRules;
use anyhow::Result;
use parking_lot::Mutex;

pub struct Hammock {
    pub rules: MatchRules,
    pub handler: CGHandler,
    apps: Mutex<Vec<App>>,
}

impl Hammock {
    pub fn new(rules: MatchRules, handler: CGHandler) -> Self {
        Self {
            rules,
            handler,
            apps: Mutex::new(Vec::new()),
        }
    }

    /// The main event loop, called every 200ms
    /// or when a new event is received
    pub fn handle_event(&self, event: HammockEvent) -> Result<()> {
        // FIXME: should just send the event via dbus to the root daemon
        match event {
            HammockEvent::NewApplication(app_info) => {
                self.apps.lock().push(
                    App::new(app_info.app_id(), app_info.pid(), Some(&self.handler))?
                );
                Ok(())
            }
            HammockEvent::NewTopLevel(top_level) => {
                let filt = AppFilter::AppId(top_level.app_id());
                for app in self.apps.lock().iter() {
                    if app.app_id == top_level.app_id() {
                        let cg = match top_level.state() {
                            Ok(TopLevelState::Activated) => self.rules.foreground(),
                            _ => self.rules.background(),
                        };
                        return cg.add_app(app.pid);
                    }
                }
                let pid = top_level.pid();
                if pid > 0 {
                    // 
                }
                trace!("FIXME! Can't map existing TopLevel to PID!!!");
                Ok(())
            }
            HammockEvent::TopLevelChanged(top_level) => {
                for app in self.apps.lock().iter() {
                    if app.app_id == top_level.app_id() {
                        let cg = match top_level.state() {
                            Ok(TopLevelState::Activated) => self.rules.foreground(),
                            _ => self.rules.background(),
                        };
                        debug!("Moving app {}:{} to {}", app.app_id, app.pid, cg.name);
                        return cg.add_app(app.pid);
                    }
                }
                trace!("FIXME! Can't map existing TopLevel to PID!!!");
                Ok(())
                //Err(anyhow!("Could not find app for toplevel"))
            }
            HammockEvent::TopLevelClosed(toplevel) => {
                let mut apps = self.apps.lock();
                let mut i = 0;
                for app in apps.iter() {
                    if app.app_id == toplevel.app_id() {
                        break;
                    }
                    i += 1;
                }
                if i < apps.len() {
                    apps.remove(i);
                    Ok(())
                } else {
                    trace!("FIXME! Can't map existing TopLevel to PID!!!");
                    Ok(())
                    //Err(anyhow!("Could not find app for toplevel!"))
                }
            }
        }
    }

    /// Find the app matched by filt and call cb with it
    /// returns Ok(()) if the callback was called
    fn with_app(filt: &AppFilter, cb: F) -> Result<()>
        where F: FnOnce(&App)
    {
        match self.apps.lock().iter().find(|app: &App| { app.matches(filt) }) {
            Some(_) => Ok(()),
            None => Err(anyhow!("Couldn't find app that matches filter {}", filt))
        }
    }
}

pub fn event_loop(hammock: Hammock, xdg_runtime_dir: &str, wl_display: &str) -> Result<()> {
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
        // FIXME: need to poll()
        let sleep_time = if elapsed > Duration::from_millis(200) {
            trace!("Event loop took {}ms!!!", elapsed.as_millis());
            Duration::from_millis(0)
        } else {
            Duration::from_millis(200) - elapsed
        };
        std::thread::sleep(sleep_time);
    }
}
