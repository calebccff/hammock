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
use clap::Parser;
use hammock::args::Args;
use hammock::events::HammockEventLoop;
use hammock::match_rules::MatchRules;
use hammock::user;
use hammock::{cgroups::CGHandler, config::Config};
use log::info;
use hammock::hammock::Hammock;
use std::ffi::OsStr;
use std::io::Write;
use std::path::PathBuf;

// fn system_init(args: Args) -> Result<()> {
    

//     HammockEventLoop::run_root(hammock)
// }

fn main() -> Result<()> {
    setup_logging();

    info!("Hello World!!!");
    return Ok(());

    let are_root = nix::unistd::getuid() == nix::unistd::Uid::from_raw(0);

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

    let hammock = Hammock::new(rules, Some(handler));

    info!(
        "Hammock daemon started! Loaded {} rules.\n{}",
        hammock.rules.len(),
        &hammock.rules
    );

    user::run(hammock, &args.xdg_runtime_dir, &args.wayland_display)?;

    // let _hammock = match are_root {
    //     true => {
    //         info!("Starting Hammock system daemon...");
    //         system_init(args)?
    //     }
    //     false => {
    //         info!("Starting Hammock user daemon...");
    //         user::run(&args.xdg_runtime_dir, &args.wayland_display)?
    //     }
    // };

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
                "{} {:<10} [{}] {}",
                chrono::Local::now().format("%F %T%.3f"),
                record.module_path().unwrap_or("").split("::").last().unwrap_or(""),
                style.value(record.level()),
                record.args()
            )
        })
        .init();
}
