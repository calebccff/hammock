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

use anyhow::{Result};
use calloop::{EventLoop};
use log::{debug, trace, warn};
use wayland_client::backend::ObjectId;
use strum_macros::Display;

use std::sync::{Arc};
use parking_lot::Mutex;
use std::sync::mpsc::{channel, Sender, SyncSender, Receiver, sync_channel};
use std::thread::spawn;
use std::{time::Duration};
use wayland_client::event_created_child;
use crate::events::HammockEvent;
use crate::events::AppId;
use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::wl_registry::{Event, WlRegistry},
    Connection, Dispatch, Proxy, QueueHandle, WaylandSource,
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

#[derive(Clone)]
pub struct HammockWl {
    tx: Sender<HammockEvent>,
}

impl HammockWl {
    /// Does not return...
    pub fn run<F>(xdg_runtime_dir: &str, wayland_display: &str, tx: Sender<HammockEvent>, cb: F) -> Result<()>
        where F: Fn() -> () {
        //::std::env::set_var("WAYLAND_DEBUG", "1");
        ::std::env::set_var("WAYLAND_DISPLAY", wayland_display);
        ::std::env::set_var("XDG_RUNTIME_DIR", xdg_runtime_dir);
        debug!(
            "(wl) Connecting to display '{}', XDG_RUNTIME_DIR=\"{}\"",
            wayland_display, xdg_runtime_dir
        );

        let conn = Connection::connect_to_env()?;
        let mut event_loop: EventLoop<HammockWl> = EventLoop::try_new()?;
        let (globals, event_queue) = registry_queue_init::<HammockWl>(&conn).unwrap();

        // Tell the server to get us the TopLevelManager
        let _: TopLevelManager = globals.bind(&event_queue.handle(), 1..=1, ())?;

        WaylandSource::new(event_queue)
            .unwrap()
            .insert(event_loop.handle())
            .unwrap();

        let mut hwl = HammockWl {
            tx,
        };

        match event_loop.run(Duration::from_millis(200), &mut hwl, |_hwl| {
            cb();
        }) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
}

impl wayland_client::Dispatch<WlRegistry, GlobalListContents> for HammockWl {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        event: Event,
        // This mutex contains an up-to-date list of the currently known globals
        // including the one that was just added or destroyed
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let Event::Global {
            name,
            interface,
            version,
        } = event
        {
            trace!("(wl) NEW global: [{}] {} (v{})", name, interface, version);
            if (interface == "zwlr_foreign_toplevel_manager_v1") && (version >= 3) {
                //state.ftlm = Some(proxy.bind(name, version, qhandle, ()));
            }
        }
    }
}

impl Dispatch<TopLevelManager, ()> for HammockWl {
    fn event(
        _state: &mut Self,
        _proxy: &TopLevelManager,
        event: <TopLevelManager as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // if let TopLevelManagerEvent::Toplevel { toplevel } = event {
        //     trace!(
        //         "(wl) Got ZwlrForeignToplevelManagerV1 event {}",
        //         toplevel.id()
        //     );
        // }
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
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if data.event(proxy, event) {
            state.tx.send(HammockEvent::TopLevelChanged(data.clone())).unwrap();
        }
    }
}

#[derive(Display, Debug, PartialEq, Clone, Copy)]
pub enum TopLevelState {
    Background = 0,
    Minimised = (1 << 1),
    Activated = (1 << 2),
    Fullscreen = (1 << 3),
}

#[derive(Debug, PartialEq)]
enum HTopLevelProp {
    Title(String),
    AppId(String),
    State(TopLevelState),
    Done,
}

#[derive(Debug)]
struct HTopLevelInner {
    title: Option<String>,
    app_id: Option<AppId>,
    state: Option<TopLevelState>,
    id: ObjectId,
}

#[derive(Debug, Clone)]
pub struct HTopLevel {
    inner: Arc<Mutex<HTopLevelInner>>,
    tx: Arc<Mutex<Option<SyncSender<HTopLevelProp>>>>,
}

impl HTopLevel {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HTopLevelInner {
                title: None,
                app_id: None,
                state: None,
                id: ObjectId::null(),
            })),
            tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Lock the inner mutex and process events until
    /// the Done event is received.
    fn process_events(self, rx: Receiver<HTopLevelProp>) {
        let mut inner = self.inner.lock();
        loop {
            // FIXME: Shouldn't unwrap here....
            let prop = rx.recv().unwrap();
            match prop {
                HTopLevelProp::Title(title) => {
                    trace!("(wl) {} Title: {}", inner.app_id.clone().unwrap_or_default(), title);
                    inner.title = Some(title)
                },
                HTopLevelProp::AppId(app_id) => {
                    trace!("(wl) AppId: {}", app_id);
                    inner.app_id = Some(app_id)
                },
                HTopLevelProp::State(state) => {
                    trace!("(wl) {} State: {}", inner.app_id.clone().unwrap_or_default(), state);
                    inner.state = Some(state)
                },
                HTopLevelProp::Done => break,
            }
        }

        *self.tx.lock() = None;
    }

    fn parse_state(arr: Vec<u8>) -> TopLevelState {
        let mut state = TopLevelState::Background;
        for s in arr.chunks_exact(4).map(|c| u32::from_ne_bytes(c.try_into().unwrap())) {
            match s {
                1 => state = TopLevelState::Minimised,
                2 => state = TopLevelState::Activated,
                3 => state = TopLevelState::Fullscreen,
                _ => (),
            }
        }
        state
    }

    /// I would like to be able to keep the mutex locked
    /// until the ::Done event, the wayland protocol
    /// offers atomicity this way. This will require
    /// some funky stuff to do properly though i expect
    /// e.g. some thread will have to hold the lock
    /// and block until the Done event is received.
    fn event(&self, _proxy: &TopLevelHandle, event: TopLevelHandleEvent) -> bool {
        // if self.inner.id.is_null() {
        //     self.inner.id = proxy.id();
        // } else if self.inner.id != proxy.id() {
        //     /// This _should_ be a developer error
        //     panic!("Mismatched window handle!");
        // }
        let prop = match event {
            TopLevelHandleEvent::Title { title } => HTopLevelProp::Title(title),
            TopLevelHandleEvent::AppId { app_id } => HTopLevelProp::AppId(app_id),
            TopLevelHandleEvent::State { state } => HTopLevelProp::State(Self::parse_state(state)),
            TopLevelHandleEvent::Closed => HTopLevelProp::Done,
            TopLevelHandleEvent::Done => HTopLevelProp::Done,
            _ => {
                warn!("Unhandled event: {:?}", event);
                HTopLevelProp::Done
            }
        };

        let done = prop == HTopLevelProp::Done;

        let mut guard = self.tx.lock();

        let tx = match guard.take() {
            Some(tx) => {
                tx
            }
            // Create a channel and spawn a thread to keep the inner mutex locked
            // until the Done event is received.
            // This makes wayland updates atomic across multiple events.
            None => {
                // Process events in tight lockstep for now...
                let (tx, rx) = sync_channel::<HTopLevelProp>(0);
                let self_clone = self.clone();
                spawn(|| {
                    self_clone.process_events(rx);
                });
                tx
            },
        };

        tx.send(prop).unwrap();

        *guard = Some(tx);
        done
    }

    pub fn get_state(&self) -> TopLevelState {
        self.inner.lock().state.clone().unwrap_or(TopLevelState::Background)
    }

    pub fn get_app_id(&self) -> AppId {
        self.inner.lock().app_id.clone().unwrap_or_default()
    }
}