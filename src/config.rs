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
use anyhow::{Result};
use serde::Deserialize;
use crate::{application::App, match_rules::MatchRule};

#[derive(Debug, PartialEq, Deserialize, Copy, Clone)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub enum Rule {
    Foreground,
    Recents,
    Background,
    Snooze,
    Media,
}

#[derive(Debug, PartialEq, Deserialize, Copy, Clone)]
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

#[derive(Debug, PartialEq, Deserialize, Copy, Clone)]
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

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all(deserialize = "kebab-case"))]
enum RET {
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
#[derive(Debug, Deserialize, PartialEq, Default)]
#[serde(rename_all(deserialize = "camelCase"))]
struct Conditional {
    #[serde(flatten)]
    ret: Option<RET>,

    not: Option<Box<Conditional>>,
    any_of: Option<Vec<Conditional>>,
    all_of: Option<Vec<Conditional>>,
    one_of: Option<Vec<Conditional>>,
}

// match-rules.enter-time.from array
#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
struct EnterTimeFrom {
    #[serde(flatten)]
    ret: RET,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
struct MatchRuleEnterTime {
    default: u32,
    from: Option<Vec<EnterTimeFrom>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
struct MatchRuleConfig {
    name: Rule,
    only_from: Option<Conditional>,
    never_from: Option<Conditional>,
    cgroup: CGroup,
    enter_time: Option<MatchRuleEnterTime>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
struct CGroup {
    cpuset: String,
    cpushares: Option<u32>,
}

impl Conditional {
    pub fn evaluate(&self, app: &App, event: Option<&Event>) -> bool {
        match self {
            Conditional { ret: Some(RET::Rule(r)), not: None, any_of: None, all_of: None, one_of: None } => {
                app.match_rule == *r
            },
            Conditional { ret: Some(RET::Event(e)), not: None, any_of: None, all_of: None, one_of: None } => {
                event.map(|e2| e2 == e).unwrap_or(false)
            },
            Conditional { ret: Some(RET::Tag(t)), not: None, any_of: None, all_of: None, one_of: None } => {
                app.tags.contains(t)
            },
            Conditional { not: Some(c), ret: None, any_of: None, all_of: None, one_of: None } => {
                !c.evaluate(app, event)
            },
            Conditional { any_of: Some(cs), not: None, ret: None, all_of: None, one_of: None } => {
                cs.iter().any(|c| c.evaluate(app, event))
            },
            Conditional { all_of: Some(cs), not: None, any_of: None, ret: None, one_of: None } => {
                cs.iter().all(|c| c.evaluate(app, event))
            },
            Conditional { one_of: Some(cs), not: None, any_of: None, all_of: None, ret: None } => {
                cs.iter().filter(|c| c.evaluate(app, event)).count() == 1
            },
            _ => false,
        }
    }
}

impl Config {
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let path = path.unwrap_or_else(|| PathBuf::from("/etc/hammock/config.yaml"));
        let config = std::fs::read_to_string(path).unwrap();

        Ok(serde_yaml::from_str(&config)?)
    }

    pub fn parse_rules(&self) -> Vec<MatchRule> {
        self.match_rules.iter().map(|c| -> MatchRule {
            MatchRule::new(c.name)
        }).collect()
    }
}
