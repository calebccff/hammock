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

use std::sync::Arc;
use cgroups_rs::freezer::FreezerController;
use cgroups_rs::{Cgroup, CgroupPid};
use anyhow::Result;
use parking_lot::RwLock;
use strum_macros::Display;
use crate::app_track::AppId;
use crate::config::{Rule, Tag};
use crate::cgroups::CGHandler;

pub struct AppMatchInfo {
    pub app_id: AppId,
    pub cgroup: Cgroup,
    pub tags: Vec<Tag>,
    pub match_rule: Rule,
}

// FIXME: doesn't belong here...
pub struct App {
    pub info: Arc<RwLock<AppMatchInfo>>,
    pub pid: u64, // The first PID, used as unique ID for an instance, may not be valid.
}

#[derive(Display)]
pub enum AppFilter<'a> {
    AppId(&'a AppId),
    Pid(u64),
    Rule(Rule),
}

impl App {
    pub fn new(app_id: AppId, pid: u64, cgh: &CGHandler) -> Result<Self> {
        let cgroup = cgh.new_cgroup(&format!("{}-{}", app_id, pid), None)?;
        match cgroup.add_task_by_tgid(CgroupPid{ pid: pid }) {
            Ok(_) => {},
            Err(e) => {
                warn!("Lost the PID race for {}-{}: {}", app_id, pid, e);
            }
        }

        Ok(Self::new_with_cgroup(app_id, pid, cgroup))
    }

    pub fn new_with_cgroup(app_id: AppId, pid: u64, cgroup: Cgroup) -> Self {
        App {
            info: Arc::new(RwLock::new(AppMatchInfo {
                app_id,
                tags: Vec::new(),
                match_rule: Rule::Foreground,
                cgroup,
            })),
            pid
        }
    }

    pub fn matches(&self, cmp: &AppFilter) -> bool {
        match cmp {
            AppFilter::AppId(app_id) => self.info.read().app_id == **app_id,
            AppFilter::Pid(pid) => self.pids().contains(&CgroupPid { pid: *pid }),
            AppFilter::Rule(rule) => self.info.read().match_rule == *rule,
        }
    }

    pub fn pids(&self) -> Vec<CgroupPid> {
        self.info.read().cgroup.tasks().into()
    }

    pub fn get_info(&self) -> Arc<RwLock<AppMatchInfo>> {
        self.info.clone()
    }

    pub fn freeze(&self) -> Result<()> {
        self.info.write().cgroup.controller_of::<FreezerController>().unwrap().freeze()?;
        Ok(())
    }

    pub fn thaw(&self) -> Result<()> {
        self.info.write().cgroup.controller_of::<FreezerController>().unwrap().thaw()?;
        Ok(())
    }
}
