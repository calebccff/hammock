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

use std::net::{UdpSocket, SocketAddr};
use std::os::unix::thread;
use std::sync::mpsc::channel;
use std::time::Duration;

use crate::app_track::{TopLevelState, AppTrack};
use crate::application::{App, AppFilter};
use crate::cgroups::CGHandler;
use crate::config::Rule;
use crate::events::{HammockEvent, HammockEventSource};
use crate::match_rules::MatchRules;
use crate::hal::{Hal, Backlight, Wakeup, WakeupType};
use anyhow::Result;
use parking_lot::Mutex;

pub struct Hammock {
    pub rules: MatchRules,
    pub handler: CGHandler,
    pub hal: Hal,
    apps: Mutex<Vec<App>>,
}

impl Hammock {
    pub fn new(rules: MatchRules, handler: CGHandler) -> Self {
        Self {
            rules,
            handler,
            hal: Hal::new(),
            apps: Mutex::new(Vec::new()),
        }
    }

    /// The main event loop, called every 200ms
    /// or when a new event is received
    fn handle_event(&self, app_track: &AppTrack, event: HammockEvent, dbg: &mut DbgSock) -> Result<()> {
        // FIXME: should just send the event via dbus to the root daemon
        match event {
            // App was launched NOT with dbus activation
            // We need to create a new cgroup for it ASAP and hope
            // we don't get screwed by PID race conditions (ie a fork)
            HammockEvent::NewApplication(app_info) => {
                self.apps.lock().push(
                    App::new(app_info.app_id(), app_info.pid(), &self.handler)?
                );
                dbg.send_app(&app_info.app_id().to_string(), true, 1);
                Ok(())
            }
            HammockEvent::NewTopLevel(top_level) => {
                let filt = AppFilter::AppId(&top_level.app_id);
                let app_id = top_level.app_id.clone().to_string();
                // Option 1: Toplevel appeared for an app we're already tracking
                if self.has_app(&filt) {
                    if top_level.pid > 0 {
                        let filt = AppFilter::Pid(top_level.pid);
                        if self.has_app(&filt) {
                            return Ok(());
                        } else {
                            // Should this be an error or is it ok?
                            // Could be multiple instances of one app?
                            // multiple windows?
                            warn!("FIXME: Toplevel matched existing AppId but not PID. {}", &top_level.app_id);
                        }
                    } else {
                        return Ok(());
                    }
                }

                // Option 2: We have a toplevel with no app, this means the app
                // was launched with dbus activation and my dbus patches
                // created a cgroup for it, load the cgroup and track the app
                // FIXME: The PID may not be the one used to launch the cgroup
                // Should instead look it up via /proc/$pid/cgroup
                let cgroup = self.handler.load_cgroup(&format!("{}-{}", top_level.app_id, top_level.pid))?;
                self.apps.lock().push(
                    App::new_with_cgroup(top_level.app_id, top_level.pid, cgroup)
                );

                dbg.send_app(&app_id, true, 1);
                Ok(())
            }
            HammockEvent::TopLevelChanged(top_level) => {
                for app in self.apps.lock().iter() {
                    let guard = app.info.read();
                    if guard.app_id == top_level.app_id {
                        let rule = match top_level.state {
                            Some(TopLevelState::Activated) => self.rules.get(Rule::Foreground),
                            _ => self.rules.get(Rule::Background),
                        }.expect(&format!("CONFIG ERROR: No rule for toplevel {:?}", &top_level.state));
                        debug!("{}:{} applying rule {}", guard.app_id, app.pid, rule.name);

                        // HACK: For PoC just freeze/unfreeze app unconditionally
                        match top_level.state {
                            Some(TopLevelState::Activated) => {
                                app.thaw()?;
                                dbg.send_app(&guard.app_id.to_string(), true, 1);
                            },
                            _ => {
                                app.freeze()?;
                                dbg.send_app(&guard.app_id.to_string(), true, 0);
                            },
                        }

                        return Ok(());
                    }
                }

                if top_level.pid > 0 {
                    debug!("TopLevelChanged: Assuming new toplevel?");
                    let cgroup = self.handler.load_cgroup(&format!("{}-{}", top_level.app_id, top_level.pid))?;
                    self.apps.lock().push(
                        App::new_with_cgroup(top_level.app_id, top_level.pid, cgroup)
                    );
                }
                //trace!("FIXME! Can't map existing TopLevel to PID!!!");
                Ok(())
            }
            HammockEvent::TopLevelClosed(toplevel) => {
                let mut apps = self.apps.lock();
                let mut i = 0;
                for app in apps.iter() {
                    if app.info.read().app_id == toplevel.app_id {
                        dbg.send_app(&app.info.read().app_id.to_string(), false, 2);
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
            HammockEvent::SystemSuspend(active) => {
                match active {
                    true => {
                        dbg.send_wakeup("About to suspend!");
                        std::thread::sleep(Duration::from_millis(400)); // HACK: Give the shell some time to turn the panel off etc...
                        self.handler.freeze_all(true)?; // Freeze all of userspace so pesky GSD doesn't touch the display when we're coming back from suspend
                        app_track.handle_suspend(true)?;
                    },
                    false => if let Ok(cause) = self.hal.wakeup().get_cause() {
                        debug!("Woke up with cause: {}", cause);
                        dbg.send_wakeup(&cause.to_string());
                        match cause {
                            WakeupType::Modem => {}, // Async wait 5+ seconds for incoming call to show up, or go back to sleep
                            WakeupType::Button => {
                                self.handler.freeze_all(false)?;
                                app_track.handle_suspend(false)?;
                            },
                            WakeupType::Motion => { // FIXME: Should wait 3-5 seconds for user interaction and then go back to sleep
                                self.handler.freeze_all(false)?;
                                app_track.handle_suspend(false)?;
                            }
                            _ => {
                                self.handler.freeze_all(false)?;
                                app_track.handle_suspend(false)?;
                            },
                        }
                    }
                }
                Ok(())
            }
        }
    }

    /// Find the app matched by filt and call cb with it
    /// returns Ok(()) if the callback was called
    // fn with_app<F>(&self, filt: &AppFilter, cb: F) -> Result<()>
    //     where F: FnOnce(&App)
    // {
    //     match self.apps.lock().iter().find(|app: &&App| { app.matches(filt) }) {
    //         Some(app) => {
    //             cb(app);
    //             Ok(())
    //         },
    //         None => Err(anyhow!("Couldn't find app that matches filter {}", filt))
    //     }
    // }

    fn has_app(&self, filt: &AppFilter) -> bool {
        self.apps.lock().iter().any(|app: &App| { app.matches(filt) })
    }
}

struct DbgSock {
    sock: UdpSocket,
}

impl DbgSock {
    pub fn send_app(&mut self, app: &str, new: bool, state: u32) {
        // App name // State (0: frozen, 1: active, 2: closed)
        let msg = format!("app:{}:{}:{}", app, match new {
            true => "new",
            false => "change",
        }, state);
        let buf = msg.as_bytes();
        let _ = self.sock.send(buf);
    }

    pub fn send_wakeup(&mut self, wakeup: &str) {
        let msg = format!("wakeup:{}", wakeup);
        let buf = msg.as_bytes();
        let _ = self.sock.send(buf);
    }

    fn connect_poll(&mut self) {
        if self.sock.peer_addr().is_err() {
            let r = self.sock.connect("172.16.42.2:4480");
            //trace!("Connecting to debug socket: {:?}", r);
        }
    }
}

pub fn event_loop(hammock: Hammock, xdg_runtime_dir: &str, wl_display: &str) -> Result<()> {
    let (tx, rx) = channel::<HammockEvent>();
    let mut app_track = AppTrack::new(xdg_runtime_dir, wl_display, &tx)?;
    let mut debug_sock = DbgSock {
        sock: UdpSocket::bind("172.16.42.1:4480")?,
    };

    //debug_sock.sock.set_write_timeout(Some(Duration::from_millis(200)))?;

    loop {
        let start = std::time::Instant::now();
        app_track.process_pending()?;
        while let Ok(event) = rx.try_recv() {
            trace!("Received event: {}", event);
            hammock.handle_event(&app_track, event, &mut debug_sock)?;
        }
        //debug_sock.connect_poll();
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
