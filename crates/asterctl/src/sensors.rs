// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2025 Markus Zehnder
// SPDX-FileCopyrightText: Copyright (c) 2026 Gabriel Max

//! Sensor value sources.
//!
//! Implementations:
//! - internal date time sensors
//! - direct system sensor polling via aster-sysinfo

use chrono::{DateTime, Datelike, Local, Timelike};
use log::{debug, info, warn};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub fn get_date_time_value(label: &str, now: &DateTime<Local>) -> Option<String> {
    if !label.starts_with("DATE_") {
        return None;
    }

    let year = now.year();
    let month = format!("{:02}", now.month());
    let day = format!("{:02}", now.day());
    let hour = format!("{:02}", now.hour());
    let minute = format!("{:02}", now.minute());
    let second = format!("{:02}", now.second());

    // same formatting logic as in AOOSTAR-X
    let value = match label {
        "DATE_year" => year.to_string(),
        "DATE_month" => month,
        "DATE_day" => day,
        "DATE_hour" => hour,
        "DATE_minute" => minute,
        "DATE_second" => second,
        "DATE_m_d_h_m_1" => format!("{month}月{day}日  {hour}:{minute}"),
        "DATE_m_d_h_m_2" => format!("{month}/{day}  {hour}:{minute}"),
        "DATE_m_d_1" => format!("{month}月{day}日"),
        "DATE_m_d_2" => format!("{month}-{day}"),
        "DATE_y_m_d_1" => format!("{year}年{month}月{day}日"),
        "DATE_y_m_d_2" => format!("{year}-{month}-{day}"),
        "DATE_y_m_d_3" => format!("{year}/{month}/{day}"),
        "DATE_y_m_d_4" => format!("{year} {month} {day}"),
        "DATE_h_m_s_1" => format!("{hour}:{minute}:{second}"),
        "DATE_h_m_s_2" => format!("{hour}时{minute}分{second}秒"),
        "DATE_h_m_s_3" => format!("{hour} {minute} {second}"),
        "DATE_h_m_1" => format!("{hour}时{minute}分"),
        "DATE_h_m_2" => format!("{hour} : {minute}"),
        "DATE_h_m_3" => format!("{hour}:{minute}"),
        _ => return None,
    };

    Some(value)
}

fn is_filtered(key: &str, filters: &[Regex]) -> bool {
    filters.iter().any(|re| re.is_match(key))
}

/// Start a direct sensor poller using SysinfoSource, eliminating the need for external scripts
/// and text files. Sensor values are read directly from the system and stored in the shared HashMap.
///
/// # Arguments
///
/// * `values`: a shared, reader-writer lock protected HashMap
/// * `refresh`: sensor refresh interval
/// * `sensor_filter`: Optional list of regex filters to filter out matching sensor keys.
///
/// returns: Result<(), Error>
pub fn start_sensor_poller(
    values: Arc<RwLock<HashMap<String, String>>>,
    refresh: std::time::Duration,
    sensor_filter: Option<Vec<Regex>>,
) -> anyhow::Result<()> {
    use aster_sysinfo::{SysinfoSource, update_linux_storage_sensors};
    use std::thread::sleep;
    use std::time::Instant;

    let mut sysinfo_source = SysinfoSource::new();

    // Initial sensor read
    {
        sysinfo_source.refresh();
        let mut raw_sensors = HashMap::with_capacity(64);
        if let Err(e) = sysinfo_source.update_sensors(&mut raw_sensors) {
            warn!("Initial sensor update failed: {e}");
        }
        if let Err(e) = update_linux_storage_sensors(&mut raw_sensors, false) {
            warn!("Initial storage sensor update failed: {e}");
        }

        let mut val = values.write().expect("Failed to lock values");
        apply_sensor_values(&mut val, &raw_sensors, sensor_filter.as_deref());
    }

    info!("Starting direct sensor poller with refresh={}ms", refresh.as_millis());

    std::thread::spawn(move || {
        let disk_refresh = std::time::Duration::from_secs(300);
        let mut disk_refresh_time = Instant::now();

        loop {
            let upd_start_time = Instant::now();

            sysinfo_source.refresh();
            let mut raw_sensors = HashMap::with_capacity(64);
            if let Err(e) = sysinfo_source.update_sensors(&mut raw_sensors) {
                warn!("Sensor update failed: {e}");
            }

            if disk_refresh_time.elapsed() > disk_refresh {
                debug!("Refreshing individual disks");
                if let Err(e) = update_linux_storage_sensors(&mut raw_sensors, false) {
                    warn!("Storage sensor update failed: {e}");
                }
                disk_refresh_time = Instant::now();
            }

            {
                let mut val = values.write().expect("Poisoned sensor RwLock");
                apply_sensor_values(&mut val, &raw_sensors, sensor_filter.as_deref());
            }

            let elapsed = upd_start_time.elapsed();
            if refresh > elapsed {
                sleep(refresh - elapsed);
            }
        }
    });

    Ok(())
}

fn apply_sensor_values(
    target: &mut HashMap<String, String>,
    source: &HashMap<String, String>,
    sensor_filter: Option<&[Regex]>,
) {
    for (key, value) in source {
        if let Some(filter) = sensor_filter
            && is_filtered(key, filter)
        {
            continue;
        }
        target.insert(key.clone(), value.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn is_filtered_does_not_filter_without_filters() {
        let key = "foobar";
        let filters = Vec::new();
        assert!(!is_filtered(key, &filters));
    }

    #[test]
    fn test_unit_extension_filter() {
        let key = "temperature_cpu#unit";
        let filters = vec![Regex::new("^temperature_.*#unit").unwrap()];
        assert!(is_filtered(key, &filters));
    }

    #[rstest]
    #[case(vec!["^foo$"])]
    #[case(vec!["^bar"])]
    #[case(vec!["other"])]
    #[case(vec!["123", "bla", "other"])]
    fn is_filtered_does_not_filter_without_a_match(#[case] filters: Vec<&str>) {
        let key = "foobar";
        let filters: Vec<Regex> = filters
            .iter()
            .map(|f| Regex::new(f).expect("Invalid regex"))
            .collect();
        assert!(
            !is_filtered(key, &filters),
            "Filter {filters:?} should not match {key}"
        );
        //
    }

    #[rstest]
    #[case(vec!["foo"])]
    #[case(vec!["bar"])]
    #[case(vec!["^.+bar"])]
    #[case(vec!["123", "foo", "other"])]
    #[case(vec!["bar", "123"])]
    #[case(vec!["^.+bar", "other"])]
    fn is_filtered_matches_filters(#[case] filters: Vec<&str>) {
        let key = "foobar";
        let filters: Vec<Regex> = filters
            .iter()
            .map(|f| Regex::new(f).expect("Invalid regex"))
            .collect();
        assert!(
            is_filtered(key, &filters),
            "Filter {filters:?} match match {key}"
        );
    }
}
