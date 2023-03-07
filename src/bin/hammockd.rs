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

use clap::Parser;
use hammock::events::HammockEventLoop;
use log::{info};
use anyhow::{bail, Result};
use hammock::{cgroups::CGHandler, config::Config};
use hammock::match_rules::{MatchRules};
use hammock::args::Args;
use hammock::user;
use nix::libc::system;
use parking_lot::Mutex;
use hammock::hammock::Hammock;
use env_logger;
use std::io::Write;
use chrono;
use nix;


fn system_init(args: Args) -> Result<()> {
    let config = match Config::load(args.config_path) {
        Ok(c) => c,
        Err(e) => bail!("Failed to load config: {}", e),
    };

    let handler = CGHandler::new();
    let rules = match config.parse_rules(&handler) {
        Ok(r) => MatchRules(r),
        Err(e) => bail!("Failed to parse rules: {}", e),
    };

    let hammock = Hammock::new(rules, Some(handler));

    info!("Hammock system daemon started! Loaded {} rules.\n{}",
        hammock.rules.len(), hammock.rules);

    HammockEventLoop::run_root(hammock)
}

fn main() -> Result<()> {
    setup_logging();

    let are_root = nix::unistd::getuid() == nix::unistd::Uid::from_raw(0);

    let args = Args::parse();
    
    let hammock = match are_root {
        true => {
            info!("Starting Hammock system daemon...");
            system_init(args)?
        }
        false => {
            info!("Starting Hammock user daemon...");
            user::run(&args.xdg_runtime_dir, &args.wayland_display)?
        }
    };

    Ok(())
}

fn setup_logging() {
    #[cfg(debug_assertions)]
    ::std::env::set_var("RUST_LOG", "trace");
    #[cfg(not(debug_assertions))]
    ::std::env::set_var("RUST_LOG", "info");

    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            let style = buf.default_level_style(record.level());

            writeln!(
                buf,
                "{} [{}] {}",
                chrono::Local::now().format("%F %T%.3f"),
                style.value(record.level()),
                record.args()
            )
        })
        .init();
}
