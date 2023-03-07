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

use crate::app_track::AppId;
use crate::config::{Rule, Tag};

// FIXME: doesn't belong here...
pub struct App {
    pub app_id: AppId,
    pub pid: u64,
    pub tags: Vec<Tag>,
    pub match_rule: Rule,
}

impl App {
    pub fn new(app_id: AppId, pid: u64) -> Self {
        App {
            app_id,
            pid,
            tags: Vec::new(),
            match_rule: Rule::Foreground,
        }
    }
}
