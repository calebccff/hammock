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

use crate::{
    application::App,
    match_rules::{MatchConditions, MatchRule}, cgroups::CGHandler,
};
use log::{debug, error, info, trace, warn};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use strum_macros::{Display, EnumString};

#[derive(Debug, PartialEq, Serialize, Deserialize, Copy, Clone, Display)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub enum Rule {
    Foreground,
    Recents,
    Background,
    Snooze,
    Media,
}

#[derive(Debug, PartialEq, Deserialize, Copy, Clone, Display)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub enum Event {
    LowBattery,
    WorkReady,
    Idle,
    Sleep,
    Wake,
    NetworkRestriction,
    Touch,
}

#[derive(Debug, PartialEq, Deserialize, Copy, Clone, Display)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub enum Tag {
    PlayingMedia,
    HammockAware,
    WorkPending,
    Busy,
    WasFocused,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
#[serde(tag = "type")]
enum EventConfig {
    LowBattery { threshold: u32 },
    WorkReady { time_period: Option<u32> },
    Idle,
    Sleep { max_time: Option<u32> },
    Wake,
    NetworkRestriction,
    Touch,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
#[serde(tag = "type")]
enum TagConfigInner {
    PlayingMedia,
    HammockAware,
    WorkPending,
    Busy { timeout: u32 },
    WasFocused,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
struct TagConfig {
    apply_latency: Option<f32>,
    remove_latency: Option<f32>,
    #[serde(flatten)]
    inner: TagConfigInner,
}

#[derive(Debug, Deserialize, PartialEq, Copy, Clone)]
#[serde(rename_all(deserialize = "kebab-case"))]
enum Atom {
    Rule(Rule),
    Event(Event),
    Tag(Tag),
}

// match-rules.{only,never}-from
// This is a tree of conditions which form an
// expression where the leaves are either the
// currently applied rule, the name of event
// that triggered this check, or a tag that is
// checked against the current application.
#[derive(Debug, Deserialize, PartialEq, Default, Clone)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Conditional {
    #[serde(flatten)]
    atom: Option<Atom>,

    not: Option<Box<Conditional>>,
    any_of: Option<Vec<Conditional>>,
    all_of: Option<Vec<Conditional>>,
    one_of: Option<Vec<Conditional>>,
}

// match-rules.enter-time.from array
#[derive(Debug, PartialEq, Deserialize, Clone)]
#[serde(rename_all(deserialize = "kebab-case"))]
struct EnterTimeFrom {
    #[serde(flatten)]
    atom: Atom,
    time: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub struct Config {
    description: String,
    cores: u32,
    memory: [u32; 2],
    match_rules: Vec<MatchRuleConfig>,
    events: Option<Vec<EventConfig>>,
    tags: Option<Vec<TagConfig>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub struct RuleEnterTime {
    default: u32,
    from: Option<Vec<EnterTimeFrom>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
struct MatchRuleConfig {
    name: Rule,
    only_from: Option<Conditional>,
    never_from: Option<Conditional>,
    cgroup: CgroupConfig,
    enter_time: RuleEnterTime,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub struct CgroupConfig {
    pub cpuset: String,
    pub cpushares: Option<u64>,
}

impl Conditional {
    pub fn evaluate(&self, app: &App, event: Option<&Event>) -> bool {
        match self {
            Conditional {
                atom: Some(Atom::Rule(r)),
                not: None,
                any_of: None,
                all_of: None,
                one_of: None,
            } => app.match_rule == *r,
            Conditional {
                atom: Some(Atom::Event(e)),
                not: None,
                any_of: None,
                all_of: None,
                one_of: None,
            } => event.map(|e2| e2 == e).unwrap_or(false),
            Conditional {
                atom: Some(Atom::Tag(t)),
                not: None,
                any_of: None,
                all_of: None,
                one_of: None,
            } => app.tags.contains(t),
            Conditional {
                not: Some(c),
                atom: None,
                any_of: None,
                all_of: None,
                one_of: None,
            } => !c.evaluate(app, event),
            Conditional {
                any_of: Some(cs),
                not: None,
                atom: None,
                all_of: None,
                one_of: None,
            } => cs.iter().any(|c| c.evaluate(app, event)),
            Conditional {
                all_of: Some(cs),
                not: None,
                any_of: None,
                atom: None,
                one_of: None,
            } => cs.iter().all(|c| c.evaluate(app, event)),
            Conditional {
                one_of: Some(cs),
                not: None,
                any_of: None,
                all_of: None,
                atom: None,
            } => cs.iter().filter(|c| c.evaluate(app, event)).count() == 1,
            _ => false,
        }
    }
}

impl Config {
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let path = path.clone().unwrap_or(PathBuf::from("docs/config.default.yaml"));
        let config = match std::fs::read_to_string(path) {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to read config: {}", e);
                return Err(e.into());
            }
        };

        match serde_yaml::from_str(&config) {
            Ok(config) => Ok(config),
            Err(e) => {
                error!("Failed to parse config: {}", e);
                Err(e.into())
            }
        }
    }

    pub fn parse_rules(self, handler: &CGHandler) -> Result<Vec<MatchRule>> {
        let mut rules: Vec<MatchRule> = vec![];

        for rule in &self.match_rules {
            let conds = MatchConditions::new(
                rule.only_from.clone(),
                rule.never_from.clone(),
                rule.enter_time.clone(),
            );

            let ruleName = rule.name.to_string().to_lowercase();

            let cgroup = match handler.new_cgroup(&ruleName, &rule.cgroup) {
                Ok(cgroup) => cgroup,
                Err(e) => {
                    error!("Failed to create cgroup for rule {}: {}", &ruleName, e);
                    return Err(e.into());
                }
            };

            rules.push(MatchRule::new(rule.name, conds, rule.cgroup.cpuset.clone(), cgroup))
        }

        Ok(rules)
    }
}
