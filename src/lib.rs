/*
* Hammock library
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

// Avoid having to manually import these in
// all modules
#[macro_use] extern crate anyhow;
#[macro_use] extern crate log;

pub mod app_track;
pub mod application;
pub mod args;
pub mod cgroups;
pub mod config;
pub mod events;
pub mod hammock;
pub mod match_rules;
pub mod user;
pub mod dbus;
