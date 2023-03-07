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

use crate::config::CgroupConfig;
use anyhow::Result;
use cgroups_rs::hierarchies::V2;
use cgroups_rs::Cgroup;
use log::info;

pub struct CGHandler {
    heirachy: Box<V2>,
}

pub struct HCGroup {
    cgroup: Cgroup,
}

impl CGHandler {
    pub fn new() -> Self {
        Self {
            heirachy: cgroups_rs::hierarchies::custom_v2("/sys/fs/cgroup/unified"),
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub fn new_cgroup(
        &self,
        name: &str,
        config: &CgroupConfig,
    ) -> Result<Cgroup, cgroups_rs::error::Error> {
        use cgroups_rs::cgroup_builder::CgroupBuilder;

        info!("Creating cgroup {} with config: {:?}", name, config);
        CgroupBuilder::new(name)
            .cpu()
            .shares(config.cpushares.unwrap_or(1024))
            .cpus(config.cpuset.clone())
            .done()
            .build(self.heirachy.clone())
    }

    #[cfg(target_arch = "x86_64")]
    pub fn new_cgroup(
        &self,
        name: &str,
        config: &CgroupConfig,
    ) -> Result<Cgroup, cgroups_rs::error::Error> {
        info!("STUB! Creating cgroup {} with config: {:?}", name, config);
        Ok(Cgroup::default())
    }
}
