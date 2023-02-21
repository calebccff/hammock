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

// Heavily inspired by https://github.com/ActivityWatch/aw-watcher-window-wayland/blob/master/src/current_window.rs

use anyhow::{bail, Result};
use calloop::{EventLoop, LoopHandle};
use log::{debug, info, trace, warn};
use wayland_client::backend::ObjectId;
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use std::{process, time::Duration};
use wayland_client::event_created_child;
use wayland_client::{
    globals::{self, registry_queue_init, Global, GlobalList, GlobalListContents},
    protocol::wl_display::WlDisplay,
    protocol::wl_registry::{Event, WlRegistry},
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WaylandSource,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{
        self as zwlr_foreign_toplevel_handle, Event as TopLevelHandleEvent,
        ZwlrForeignToplevelHandleV1 as TopLevelHandle,
    },
    zwlr_foreign_toplevel_manager_v1::{
        self as zwlr_foreign_toplevel_manager, Event as TopLevelManagerEvent,
        ZwlrForeignToplevelManagerV1 as TopLevelManager, EVT_TOPLEVEL_OPCODE,
    },
};

pub struct HammockWl {
    toplevels: Arc<Mutex<Vec<HTopLevel>>>,
}

impl HammockWl {
    fn handle_event(&mut self) {
        //debug!("Handling event");
    }

    pub fn wayland_init(xdg_runtime_dir: &str, wayland_display: &str) -> Result<HammockWl> {
        ::std::env::set_var("WAYLAND_DEBUG", "1");
        ::std::env::set_var("WAYLAND_DISPLAY", wayland_display);
        ::std::env::set_var("XDG_RUNTIME_DIR", xdg_runtime_dir);
        debug!(
            "Connecting to display '{}', XDG_RUNTIME_DIR=\"{}\"",
            wayland_display, xdg_runtime_dir
        );

        let conn = Connection::connect_to_env()?;
        let display = conn.display();
        let mut event_loop: EventLoop<HammockWl> = EventLoop::try_new()?;
        let (globals, event_queue) = registry_queue_init::<HammockWl>(&conn).unwrap();

        // Tell the server to get us the TopLevelManager
        let ftlm: TopLevelManager = globals.bind(&event_queue.handle(), 1..=1, ())?;

        WaylandSource::new(event_queue)
            .unwrap()
            .insert(event_loop.handle())
            .unwrap();

        let mut hwl = HammockWl {
            toplevels: Arc::new(Mutex::new(Vec::new())),
        };

        event_loop.run(Duration::from_millis(200), &mut hwl, |hwl| {
            hwl.handle_event()
        });

        Ok(hwl)
    }
}

impl wayland_client::Dispatch<WlRegistry, GlobalListContents> for HammockWl {
    fn event(
        state: &mut Self,
        proxy: &WlRegistry,
        event: Event,
        // This mutex contains an up-to-date list of the currently known globals
        // including the one that was just added or destroyed
        data: &GlobalListContents,
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        if let Event::Global {
            name,
            interface,
            version,
        } = event
        {
            trace!("NEW global: [{}] {} (v{})", name, interface, version);
            if (interface == "zwlr_foreign_toplevel_manager_v1") && (version >= 3) {
                //state.ftlm = Some(proxy.bind(name, version, qhandle, ()));
            }
        }
    }
}

impl Dispatch<TopLevelManager, ()> for HammockWl {
    fn event(
        state: &mut Self,
        proxy: &TopLevelManager,
        event: <TopLevelManager as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        if let TopLevelManagerEvent::Toplevel { toplevel } = event {
            print!(
                "Got ZwlrForeignToplevelManagerV1 event {}, data",
                toplevel.id()
            );
        }
    }

    event_created_child!(HammockWl, TopLevelManager, [
        // Toplevel created
        EVT_TOPLEVEL_OPCODE => (TopLevelHandle, HTopLevel::new()),
    ]);
}

impl Dispatch<TopLevelHandle, HTopLevel> for HammockWl {
    fn event(
        state: &mut Self,
        proxy: &TopLevelHandle,
        event: <TopLevelHandle as Proxy>::Event,
        data: &HTopLevel,
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        data.event(proxy, event);
    }
}

struct HTopLevelInner {
    title: String,
    app_id: String,
    state: Vec<u8>,
}

#[derive(Clone)]
pub struct HTopLevel {
    inner: Arc<Mutex<HTopLevelInner>>,
    id: ObjectId,
}

impl HTopLevel {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HTopLevelInner {
                title: "Unknown".into(),
                app_id: "unknown".into(),
                state: Vec::new(),
            })),
            id: ObjectId::null(),
        }
    }

    /// I would like to be able to keep the mutex locked
    /// until the ::Done event, the wayland protocol
    /// offers atomicity this way. This will require
    /// some funky stuff to do properly though i expect
    /// e.g. some thread will have to hold the lock
    /// and block until the Done event is received.
    fn event(&self, proxy: &TopLevelHandle, event: TopLevelHandleEvent) {
        // if self.id.is_null() {
        //     self.id = proxy.id();
        // } else if self.id != proxy.id() {
        //     /// This _should_ be a developer error
        //     panic!("Mismatched window handle!");
        // }
        match event {
            TopLevelHandleEvent::Title { title } => {
                self.inner.lock().unwrap().title = title.clone();
            }
            TopLevelHandleEvent::AppId { app_id } => {
                self.inner.lock().unwrap().app_id = app_id.clone();
            }
            TopLevelHandleEvent::State { state } => {
                self.inner.lock().unwrap().state = state;
            }
            TopLevelHandleEvent::Done => {
                debug!("Done updating window {}", self.inner.lock().unwrap().title);
            }
            _ => {}
        }
    }
}
