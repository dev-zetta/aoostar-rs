// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2025 Markus Zehnder
// SPDX-FileCopyrightText: Copyright (c) 2026 Gabriel Max

#![forbid(non_ascii_idents)]
#![deny(unsafe_code)]

use aster_sysinfo::{SysinfoSource, update_linux_storage_sensors};
use clap::Parser;
use env_logger::Env;
use itertools::Itertools;
use log::{debug, info};
use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tempfile::Builder;

/// Proof of concept sensor value collection for the asterctl screen control tool.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Output sensor file.
    #[arg(short, long)]
    out: Option<PathBuf>,

    /// Temporary directory for preparing the output sensor file.
    ///
    /// The system temp directory is used if not specified.
    /// The temp directory must be on the same file system for atomic rename operation!
    #[arg(short, long)]
    temp_dir: Option<PathBuf>,

    /// Print values in console
    #[arg(long)]
    console: bool,

    /// System sensor refresh interval in seconds
    #[arg(short, long)]
    refresh: Option<u16>,

    /// Enable individual disk refresh logic as used in AOOSTAR-X. Refresh interval in seconds.
    #[arg(long)]
    disk_refresh: Option<u16>,

    /// Retrieve drive temperature if `disk-update` option is enabled.
    ///
    /// Requires smartctl and password-less sudo!
    #[cfg(target_os = "linux")]
    #[arg(long)]
    smartctl: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    #[cfg(target_os = "linux")]
    let use_smartctl = args.smartctl;
    #[cfg(not(target_os = "linux"))]
    let use_smartctl = false;

    if let Some(out_file) = &args.out
        && let Some(parent) = out_file.parent()
    {
        fs::create_dir_all(parent)?;
    }
    let mut sensors = HashMap::with_capacity(64);
    let mut sysinfo_source = SysinfoSource::new();

    let refresh = Duration::from_secs(args.refresh.unwrap_or_default() as u64);

    let disk_refresh = Duration::from_secs(args.disk_refresh.unwrap_or_default() as u64);
    let mut disk_refresh_time = Instant::now();
    if !disk_refresh.is_zero() {
        update_linux_storage_sensors(&mut sensors, use_smartctl)?;
    }

    if !refresh.is_zero() {
        info!(
            "Starting aster-sysinfo with refresh={}ms",
            refresh.as_millis()
        );
    }

    loop {
        let upd_start_time = Instant::now();

        sysinfo_source.refresh();
        sysinfo_source.update_sensors(&mut sensors)?;

        if !disk_refresh.is_zero() && disk_refresh_time.elapsed() > disk_refresh {
            debug!("Refreshing individual disks");
            update_linux_storage_sensors(&mut sensors, use_smartctl)?;
            disk_refresh_time = Instant::now();
        }

        if let Some(out_file) = &args.out {
            write_sensor_file(out_file, args.temp_dir.as_deref(), &sensors)?;
        }

        if args.console {
            // pretty print console output with sorted keys
            for (label, value) in sensors.iter().sorted() {
                println!("{}: {}", label, value);
            }
            println!();
        }

        if refresh.is_zero() {
            break;
        }

        let elapsed = upd_start_time.elapsed();
        if refresh > elapsed {
            sleep(refresh - elapsed);
        }
    }

    Ok(())
}

fn write_sensor_file(
    out_file: &Path,
    temp_dir: Option<&Path>,
    sensors: &HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if out_file.is_dir() {
        log::error!("Output cannot be a directory: {}", out_file.display());
        exit(1);
    }

    // make sure our sensor file can be read by everyone
    let all_read_perm = fs::Permissions::from_mode(0o664);
    let tmp_file = if let Some(temp_path) = temp_dir {
        fs::create_dir_all(temp_path)?;

        debug!("Creating a new named temp file in {temp_path:?}");
        Builder::new()
            .permissions(all_read_perm)
            .tempfile_in(temp_path)?
    } else {
        debug!("Creating a new named temp file");
        Builder::new().permissions(all_read_perm).tempfile()?
    };

    debug!("Writing sensor temp file...");
    let mut stream = BufWriter::new(&tmp_file);

    for (label, value) in sensors.iter() {
        writeln!(stream, "{label}: {value}")?;
    }

    stream.flush()?;
    drop(stream);
    debug!("Renaming temp file to: {out_file:?}");
    tmp_file.persist(out_file)?;

    Ok(())
}
