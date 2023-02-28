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
use log::{debug, error, info, trace, warn};
use anyhow::{bail, Result};
use hammock::{cgroups::CGHandler, config::Config};
use hammock::match_rules::{MatchRule, MatchRules};
use hammock::args::Args;
use hammock::wayland::HammockWl;
use hammock::hammock::Hammock;
use env_logger;
use std::io::Write;
use chrono;


fn main() -> Result<()> {
    setup_logging();

    let args = Args::parse();
    let config = match Config::load(args.config_path) {
        Ok(c) => c,
        Err(e) => bail!("Failed to load config: {}", e),
    };

    let handler = CGHandler::new();
    let rules = match config.parse_rules(&handler) {
        Ok(r) => MatchRules(r),
        Err(e) => bail!("Failed to parse rules: {}", e),
    };

    let hammock = Hammock {
        rules,
        handler,
    };

    info!("Hammock started! Loaded {} rules.\n{}", hammock.rules.len(), hammock.rules);

    HammockEventLoop::run(hammock, &args.xdg_runtime_dir, &args.wayland_display);

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
