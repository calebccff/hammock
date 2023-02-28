/*
* Hammock
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

use clap::Parser;
use log::{debug, error, info, trace, warn};
use anyhow::{bail, Result};
use crate::{cgroups::CGHandler, config::Config};
use crate::match_rules::{MatchRule, MatchRules};
use crate::args::Args;
use crate::wayland::HammockWl;
use env_logger;
use std::io::Write;
use chrono;

pub struct Hammock {
    pub rules: MatchRules,
    pub handler: CGHandler,
}

impl Hammock {
    /// The main event loop, called every 200ms
    /// or when a new event is received
    pub fn event_loop(&self) {

    }
}