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

use anyhow::{anyhow, Result};
use crate::config::{Conditional, Rule, RuleEnterTime, CgroupConfig};
use cgroups_rs::{Cgroup, CgroupPid};
use std::string::ToString;
use std::{fmt, ops};

pub mod logic;

pub struct MatchConditions {
    only_from: Option<Conditional>,
    never_from: Option<Conditional>,
    enter_time: RuleEnterTime,
}

impl MatchConditions {
    pub fn new(
        only_from: Option<Conditional>,
        never_from: Option<Conditional>,
        enter_time: RuleEnterTime,
    ) -> Self {
        Self {
            only_from,
            never_from,
            enter_time,
        }
    }
}

pub struct MatchRule {
    pub name: Rule,
    conditions: MatchConditions,
    cgroup: CgroupConfig,
}

impl MatchRule {
    pub fn new(name: Rule, conditions: MatchConditions, cgroup: CgroupConfig) -> Self {
        Self {
            name,
            conditions,
            cgroup,
        }
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
            result.and_then(|_| writeln!(f, "\t{}: [cpus {}]", rule, &rule.cgroup.cpuset))
        })
    }
}

impl ops::Deref for MatchRules {
    type Target = Vec<MatchRule>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MatchRules {
    pub fn get(&self, rule: Rule) -> Result<&MatchRule> {
        self.iter()
            .find(|r| r.name == rule)
            .ok_or_else(|| anyhow!("No rule named {}", rule))
    }
}
