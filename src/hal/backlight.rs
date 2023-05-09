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

// Based on https://github.com/jeremija/backlight/blob/master/src/lib.rs

use anyhow::Result;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::io;
use std::io::Write;
use std::path::{PathBuf};
use glob::glob;

pub struct Backlight {
    path: PathBuf,
    max_brightness: i32,
}

impl std::default::Default for Backlight {
    fn default() -> Backlight {
        return Backlight {
            path: glob("/sys/class/backlight/*").unwrap().next().unwrap().unwrap(),
            max_brightness: 0,
        }
    }
}

impl Backlight {
    fn get(&self, filename: &str) -> Result<i32, io::Error> {
        let path = self.path.as_path();
        let mut file = File::open(path)?;

        let mut content = String::new();
        file.read_to_string(&mut content)?;

        match content.trim().parse::<i32>() {
            Ok(value) => Ok(value),
            Err(_) => {
                Ok(-1)
            }
        }
    }

    pub fn set_brightness(&self, mut value: i32) -> Result<bool, io::Error> {
        let max = self.get_max_brightness()?;
        if value > max {
            value = max;
        } else if value < 0 {
            value = 0;
        }

        let path = self.path.as_path();

        let mut file = OpenOptions::new().write(true).open(path)?;

        match file.write_all(value.to_string().as_bytes()) {
            Ok(_) => Ok(true),
            Err(err) => Err(err)
        }
    }

    pub fn get_max_brightness(&self) -> Result<i32, io::Error> {
        if self.max_brightness > 0 {
            return Ok(self.max_brightness);
        }
        return self.get("max_brightness");
    }

    pub fn get_brightness(&self) -> Result<i32, io::Error> {
        return self.get("brightness");
    }

    pub fn get_percent(&self) -> Result<i32, io::Error> {
        let value = self.get_brightness()? as f32;
        let max = self.get_max_brightness()? as f32;
        let result = (100 as f32) * (value + 0.5) / max;
        return Ok(result as i32);
    }

    pub fn set_percent(&self, value: i32) -> Result<bool, io::Error> {
        let max = self.get_max_brightness()?;
        let value = (value as f32) / (100_f32) * (max as f32) + 0.5_f32;
        let value = value as i32;
        return self.set_brightness(value as i32);
    }
}
