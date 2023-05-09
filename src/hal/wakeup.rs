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

// Based on https://github.com/jeremija/backlight/blob/master/src/lib.rs

use std::{path::{PathBuf, Path}, cell::RefCell, sync::Arc};
use glob::glob;

use anyhow::Result;
use parking_lot::Mutex;
use strum_macros::Display;
use std::fs::File;
use std::io::Read;
use std::io;

// FIXME: Move to config and deserialise
#[derive(Debug, Clone, Copy, Display)]
pub enum WakeupType {
    Button,
    Motion,
    Charger,
    //ChargerAttach,
    //ChargerDetach,
    Modem,
    Notification,
}

pub struct WakeupSource {
    name: String,
    wakeup_type: WakeupType,
    device: String,
    wakeup_path: PathBuf,
    count: u32,
}

impl WakeupSource {
    pub fn new(name: &str, wakeup_type: WakeupType, path: PathBuf) -> Result<WakeupSource> {
        let dev_str = path.to_string_lossy();
        let wakeup_path = match glob(format!("{}/*/wakeup[0-9]*", dev_str).as_str())?.next() {
            Some(r) => r?,
            None => return Err(anyhow!("Wakeup device does not exist for {}", path.display())),
        };
        if !wakeup_path.exists() {
            return Err(anyhow!("Wakeup device does not exist for {}", path.display()));
        }

        let count = Self::get_count(&wakeup_path)?;

        Ok(WakeupSource {
            name: name.to_string(),
            wakeup_type,
            device: path.file_name().and_then(|s| s.to_str()).unwrap().to_string(),
            wakeup_path,
            count,
        })
    }

    fn get_count(path: &PathBuf) -> Result<u32> {
        let mut path = path.clone();
        path.push("wakeup_count");

        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        match content.trim().parse::<u32>() {
            Ok(value) => Ok(value),
            Err(e) => {
                Err(anyhow!("Failed to parse wakeup count: {}", e))
            }
        }
    }

    fn did_cause_wakeup(&mut self) -> Result<bool> {
        let count = Self::get_count(&self.wakeup_path)?;
        let v = count > self.count;
        self.count = count;
        Ok(v)
    }
}

struct WakeupState {
    sources: Vec<WakeupSource>,
    cause: u32, // Index
}

pub struct Wakeup {
    state: Arc<Mutex<WakeupState>>,
}

impl Wakeup {
    pub fn new(/*Config*/) -> Self {
        let sources = Self::temp_get_axolotl_sources();
        
        Wakeup {
            state: Arc::new(Mutex::new(WakeupState {
                sources,
                cause: 0,
            })),
        }
    }

    fn temp_get_axolotl_sources() -> Vec<WakeupSource> {
        let mut sources = Vec::new();

        match WakeupSource::new("Power Button", WakeupType::Button, PathBuf::from("/sys/devices/platform/soc@0/c440000.spmi/spmi-0/0-00/c440000.spmi:pmic@0:pon@800/c440000.spmi:pmic@0:pon@800:pwrkey")) {
            Ok(s) => sources.push(s),
            Err(e) => warn!("Failed to add wakeup source: {}", e),
        }

        match WakeupSource::new("Charger", WakeupType::Charger, PathBuf::from("/sys/devices/platform/soc@0/c440000.spmi/spmi-0/0-02/c440000.spmi:pmic@2:charger@1000/power_supply/pmi8998-charger")) {
            Ok(s) => sources.push(s),
            Err(e) => warn!("Failed to add wakeup source: {}", e),
        }

        match WakeupSource::new("SLPI", WakeupType::Motion, PathBuf::from("/sys//sys/devices/platform/soc@0/5c00000.remoteproc/remoteproc/remoteproc2/5c00000.remoteproc:glink-edge/5c00000.remoteproc:glink-edge.IPCRTR.-1.-1")) {
            Ok(s) => sources.push(s),
            Err(e) => warn!("Failed to add wakeup source: {}", e),
        }

        match WakeupSource::new("Modem", WakeupType::Modem, PathBuf::from("/sys/devices/platform/soc@0/4080000.remoteproc/remoteproc/remoteproc3/4080000.remoteproc:glink-edge/4080000.remoteproc:glink-edge.IPCRTR.-1.-1")) {
            Ok(s) => sources.push(s),
            Err(e) => warn!("Failed to add wakeup source: {}", e),
        }

        sources
    }

    pub fn get_cause(&self) -> Result<WakeupType> {
        let mut state = self.state.lock();
        //let mut cause = 0;
        for (_i, source) in state.sources.iter_mut().enumerate() {
            if source.did_cause_wakeup()? {
                //cause = i as u32;
                return Ok(source.wakeup_type.clone());
            }
        }
        //state.cause = cause;
        Ok(state.sources[state.cause as usize].wakeup_type.clone())
    }
}
