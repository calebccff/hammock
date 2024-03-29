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

use anyhow::Result;
use log::{debug, info, trace, warn};
use serde::{Serialize, Deserialize};
use wayland_client::backend::ObjectId;
use parking_lot::Mutex;
use strum_macros::Display as StrumDisplay;
use std::sync::mpsc::{sync_channel, Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::thread::{spawn, JoinHandle};
use super::AppId;
use crate::events::{HammockEvent, HammockEventSource};
use wayland_client::event_created_child;
use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::wl_registry::{Event, WlRegistry},
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{
        Event as TopLevelHandleEvent, ZwlrForeignToplevelHandleV1 as TopLevelHandle,
    },
    zwlr_foreign_toplevel_manager_v1::{
        Event as TopLevelManagerEvent, ZwlrForeignToplevelManagerV1 as TopLevelManager,
        EVT_TOPLEVEL_OPCODE,
    },
};

#[derive(Clone)]
struct HammockWlInner {
    exit: Arc<Mutex<bool>>,
    tx: Sender<HammockEvent>,
}

pub(super) struct HammockWl {
    exit: Arc<Mutex<bool>>,
    handle: JoinHandle<Result<()>>,
}

impl HammockWl {
    pub(super) fn new(
        xdg_runtime_dir: &str,
        wayland_display: &str,
        tx: Sender<HammockEvent>,
    ) -> Result<HammockWl> {
        //::std::env::set_var("WAYLAND_DEBUG", "1");
        ::std::env::set_var("WAYLAND_DISPLAY", wayland_display);
        ::std::env::set_var("XDG_RUNTIME_DIR", xdg_runtime_dir);
        debug!(
            "Connecting to display '{}', XDG_RUNTIME_DIR=\"{}\"",
            wayland_display, xdg_runtime_dir
        );

        let conn = Connection::connect_to_env()?;
        let (globals, mut event_queue) = registry_queue_init::<HammockWlInner>(&conn).unwrap();

        // Tell the server to get us the TopLevelManager
        globals.bind::<TopLevelManager, _, _>(&event_queue.handle(), 1..=1, ())?;

        let exit = Arc::new(Mutex::new(false));

        let mut inner = HammockWlInner {
            exit: exit.clone(),
            tx,
        };

        let wl_handle = std::thread::spawn(move || {
            loop {
                match event_queue.blocking_dispatch(&mut inner) {
                    Ok(_) => {}
                    Err(err) => {
                        bail!("Error while dispatching pending events: {}", err);
                    }
                };
                if *inner.exit.lock() {
                    info!("Exiting");
                    break;
                }
            }
            Ok(())
        });

        Ok(HammockWl {
            exit,
            handle: wl_handle,
        })
    }

    pub fn exit(&self) {
        *self.exit.lock() = true;
    }
}

impl HammockEventSource for HammockWl {
    fn process_pending(&mut self) -> Result<()> {
        // TODO: May want to check / restart the thread ?
        Ok(())
    }
}

impl wayland_client::Dispatch<WlRegistry, GlobalListContents> for HammockWlInner {
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
            name: _,
            interface: _,
            version: _,
        } = event
        {
            //trace!("NEW global: [{}] {} (v{})", name, interface, version);
        }
    }
}

impl Dispatch<TopLevelManager, ()> for HammockWlInner {
    fn event(
        _state: &mut Self,
        _proxy: &TopLevelManager,
        event: <TopLevelManager as Proxy>::Event,
        // This mutex contains an up-to-date list of the currently known globals
        // including the one that was just added or destroyed
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let TopLevelManagerEvent::Toplevel { .. } = event {
            //trace!("Got TopLevelManager!");
        }
    }

    event_created_child!(HammockWlInner, TopLevelManager, [
        // Toplevel created
        EVT_TOPLEVEL_OPCODE => (TopLevelHandle, TopLevel::new()),
    ]);
}

impl Dispatch<TopLevelHandle, TopLevel> for HammockWlInner {
    fn event(
        state: &mut Self,
        _: &TopLevelHandle,
        event: <TopLevelHandle as Proxy>::Event,
        data: &TopLevel,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match data.event(event) {
            TopLevelProp::Done => {
                let ev = match data.is_new() {
                    true => HammockEvent::TopLevelChanged(data.clone_inner()), // FIXME: is_new() borked?
                    false => HammockEvent::TopLevelChanged(data.clone_inner()),
                };
                trace!("TopLevel update done!");
                state
                .tx
                .send(ev)
                .unwrap();
            },
            TopLevelProp::Closed => {
                trace!("TopLevel closed!");
                state
                .tx
                .send(HammockEvent::TopLevelClosed(data.clone_inner()))
                .unwrap();
            },
            _ => {}
        }
    }
}

#[derive(StrumDisplay, Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum TopLevelState {
    Background = 0,
    Minimised = (1 << 1),
    Activated = (1 << 2),
    Fullscreen = (1 << 3),
}

#[derive(Debug, PartialEq, Clone)]
enum TopLevelProp {
    Title(String),
    AppId(String),
    State(TopLevelState),
    Credentials(u64),
    Done,
    Closed,
}

#[derive(Debug, Clone)]
pub struct TopLevelInner {
    new: u32,
    pub title: String,
    pub app_id: AppId,
    pub state: Option<TopLevelState>,
    pub pid: u64,
    pub id: ObjectId,
}

#[derive(Debug, Clone)]
pub struct TopLevel {
    inner: Arc<Mutex<TopLevelInner>>,
    tx: Arc<Mutex<Option<SyncSender<TopLevelProp>>>>,
}

impl TopLevel {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TopLevelInner {
                new: 0,
                title: "".into(),
                app_id: AppId::default(),
                state: None,
                pid: 0,
                id: ObjectId::null(),
            })),
            tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Lock the inner mutex and process events until
    /// the Done event is received.
    fn process_events(self, rx: Receiver<TopLevelProp>) {
        let mut inner = self.inner.lock();
        loop {
            // FIXME: Shouldn't unwrap here....
            let prop = rx.recv().unwrap();
            match prop {
                TopLevelProp::Title(title) => {
                    trace!("Title: {}", title);
                    inner.title = title
                }
                TopLevelProp::AppId(app_id) => {
                    trace!("{}: got app_id: {}", &inner.title, app_id);
                    inner.app_id = app_id.into()
                }
                TopLevelProp::State(state) => {
                    trace!("{} State: {}", &inner.title, state);
                    inner.state = Some(state)
                }
                TopLevelProp::Credentials(pid) => {
                    trace!("{} Pid: {}", &inner.title, pid);
                    inner.pid = pid;
                }
                TopLevelProp::Done => break,
                TopLevelProp::Closed => {
                    trace!("{} Closed", &inner.app_id);
                    break;
                }
            }
        }

        *self.tx.lock() = None;
    }

    fn parse_state(arr: Vec<u8>) -> TopLevelState {
        let mut state = TopLevelState::Background;
        for s in arr
            .chunks_exact(4)
            .map(|c| u32::from_ne_bytes(c.try_into().unwrap()))
        {
            match s {
                1 => state = TopLevelState::Minimised,
                2 => state = TopLevelState::Activated,
                3 => state = TopLevelState::Fullscreen,
                _ => (),
            }
        }
        state
    }

    fn event(&self, event: TopLevelHandleEvent) -> TopLevelProp {
        // if self.inner.id.is_null() {
        //     self.inner.id = proxy.id();
        // } else if self.inner.id != proxy.id() {
        //     /// This _should_ be a developer error
        //     panic!("Mismatched window handle!");
        // }
        let prop = match event {
            TopLevelHandleEvent::Title { title } => TopLevelProp::Title(title),
            TopLevelHandleEvent::AppId { app_id } => TopLevelProp::AppId(app_id),
            TopLevelHandleEvent::State { state } => TopLevelProp::State(Self::parse_state(state)),
            TopLevelHandleEvent::Credentials { pid, .. } => TopLevelProp::Credentials(pid.into()),
            TopLevelHandleEvent::Closed => TopLevelProp::Closed,
            TopLevelHandleEvent::Done => TopLevelProp::Done,
            _ => {
                warn!("Unhandled event: {:?}", event);
                TopLevelProp::Done
            }
        };

        let mut guard = self.tx.lock();

        let tx = match guard.take() {
            Some(tx) => tx,
            // Create a channel and spawn a thread to keep the inner mutex locked
            // until the Done event is received.
            // This makes wayland updates atomic across multiple events.
            None => {
                // Process events in tight lockstep for now...
                let (tx, rx) = sync_channel::<TopLevelProp>(0);
                let self_clone = self.clone();
                spawn(|| {
                    self_clone.process_events(rx);
                });
                tx
            }
        };

        tx.send(prop.clone()).unwrap();

        *guard = Some(tx);
        // if prop == TopLevelProp::Done {
        //     let mut inner = self.inner.lock();
        //     (*inner).new += 1;
        // }
        // If we got a Done event then the other thread
        // will release the mutex so we can lock it and
        // update the object id.
        // if prop == TopLevelProp::Done {
        //     let mut inner = self.inner.lock();
        //     (*inner).id = proxy.id();
        // }
        prop
    }

    pub fn state(&self) -> Result<TopLevelState> {
        match self.inner.lock().state {
            Some(state) => Ok(state),
            None => Err(anyhow::anyhow!("No state set")),
        }
    }

    pub fn is_new(&self) -> bool {
        self.inner.lock().new == 1
    }

    pub fn app_id(&self) -> AppId {
        self.inner.lock().app_id.clone()
    }

    pub fn pid(&self) -> u64 {
        self.inner.lock().pid
    }

    pub fn clone_inner(&self) -> TopLevelInner {
        self.inner.lock().clone()
    }
}

// impl Serialize for TopLevel {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let inner = self.inner.lock();
//         let mut state = serializer.serialize_struct("TopLevel", 3)?;
//         state.serialize_field("title", &inner.title)?;
//         state.serialize_field("app_id", &inner.app_id)?;
//         state.serialize_field("state", &inner.state)?;
//         state.end()
//     }
// }

// impl Deserialize<'de> for TopLevel {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         let fields = ["title", "app_id", "state"];
//         let mut deserializer = deserializer.deserialize_struct("TopLevel", &fields, Visitor)?;
//         let title = deserializer.deserialize_field(&mut deserializer, "title")?;
//         let app_id = deserializer.deserialize_field(&mut deserializer, "app_id")?;
//         let state = deserializer.deserialize_field(&mut deserializer, "state")?;
//         deserializer.end()?;
//         Ok(Self {
//             inner: Arc::new(Mutex::new(TopLevelInner {
//                 title,
//                 app_id,
//                 state,
//                 id: ObjectId::null(),
//             })),
//             tx: Arc::new(Mutex::new(None)),
//         })
//     }
// }

// impl Type for TopLevel {
//     fn signature() -> Signature<'static> {
//         Signature::from_static_str("r(ssu)")
//     }
// }
