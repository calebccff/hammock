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

use std::path::PathBuf;

use crate::config::CgroupConfig;
use anyhow::Result;
use cgroups_rs::hierarchies::{V2, custom_v2};
use cgroups_rs::{Cgroup, Hierarchy};
use cgroups_rs::cgroup_builder::CgroupBuilder;
use cgroups_rs::freezer::FreezerController;

pub struct CGHandler {
    heirachy: Box<V2>,
    root: Cgroup,
}

impl CGHandler {
    pub fn new() -> Self {
        Self {
            heirachy: custom_v2("/sys/fs/cgroup/unified/tinydm"),
            root: CgroupBuilder::new(&"tinydm")
                .set_specified_controllers(vec!["cpuset".into(), "pids".into(), "freezer".into()])
                .build(custom_v2("/sys/fs/cgroup/unified")).unwrap(),
        }
    }

    //#[cfg(not(target_arch = "x86_64"))]
    pub fn new_cgroup(
        &self,
        name: &str,
        _config: Option<&CgroupConfig>,
    ) -> Result<Cgroup, cgroups_rs::error::Error> {

        info!("Creating cgroup '{}'", name);
        match CgroupBuilder::new(name)
            //.set_specified_controllers(vec!["cpuset".into(), "freezer".into(), "pids".into()])
            // .cpu()
            // .shares(config.cpushares.unwrap_or(1024))
            // .cpus(config.cpuset.clone())
            // .done()
            // .pid().done()
            .build(self.heirachy.clone()) {
            Ok(cgroup) => {
                //cgroup.set_cgroup_type("threaded")?;
                Ok(cgroup)
            }
            Err(e) => Err(e),
            }
    }

    /// Validate that a cgroup path exists and then create a cgroup handle
    /// for it.
    pub fn load_cgroup(&self, name: &str) -> Result<Cgroup> {
        // let mut path: PathBuf = self.heirachy.root().to_path_buf();
        // path.push(name);
        // info!("Loading cgroup from path: {}", path.to_str().unwrap());
        // if !path.exists() {
        //     bail!("Invalid path");
        // }

        match self.new_cgroup(name, None) {
            Ok(cgroup) => Ok(cgroup),
            Err(e) => bail!("Failed to load cgroup: {}", e),
        }
    }

    pub fn freeze_all(&self, active: bool) -> Result<()> {
        info!("Freezing all user processes");
        let freezer: &FreezerController = match self.root.controller_of() {
            Some(freezer) => freezer,
            None => bail!("Failed to get root freezer controller"),
        };

        if active {
            freezer.freeze()?;
        } else {
            freezer.thaw()?;
        }

        Ok(())
    }

    // #[cfg(target_arch = "x86_64")]
    // pub fn new_cgroup(
    //     &self,
    //     name: &str,
    //     config: &CgroupConfig,
    // ) -> Result<Cgroup, cgroups_rs::error::Error> {
    //     info!("STUB! Creating cgroup {} with config: {:?}", name, config);
    //     Ok(Cgroup::default())
    // }
}

impl Default for CGHandler {
    fn default() -> Self {
        Self::new()
    }
}
