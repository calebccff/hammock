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

use anyhow::{bail, Result};
use cgroups_rs::cpuset::CpuSetController;
use crate::config::{Rule, Conditional, RuleEnterTime};
use cgroups_rs::Cgroup;
use std::{fmt, ops};
use std::string::ToString;

pub struct MatchConditions {
    only_from: Option<Conditional>,
    never_from: Option<Conditional>,
    enter_time: RuleEnterTime,
}

impl MatchConditions {
    pub fn new(only_from: Option<Conditional>, never_from: Option<Conditional>, enter_time: RuleEnterTime) -> Self {
        Self { only_from, never_from, enter_time }
    }
}

pub struct MatchRule {
    pub name: Rule,
    pub conditions: MatchConditions,
    cpuset: String,
    cgroup: Cgroup,
}

impl MatchRule {
    pub fn new(name: Rule, conditions: MatchConditions, cpuset: String, cgroup: Cgroup) -> Self {
        Self { name, conditions, cpuset, cgroup }
    }
}

// Annoying stuff to make it easy to display stuff
pub struct MatchRules(pub Vec<MatchRule>);

impl fmt::Display for MatchRule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name.to_string())
    }
}

impl fmt::Display for MatchRules {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.iter().fold(Ok(()), |result, rule| {
            result.and_then(|_| writeln!(f, "\t{}: [cpus {}]", rule, &rule.cpuset))
        })
    }
}

impl ops::Deref for MatchRules {
    type Target = Vec<MatchRule>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}