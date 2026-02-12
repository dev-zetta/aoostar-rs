// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2025 Markus Zehnder
// SPDX-FileCopyrightText: Copyright (c) 2026 Gabriel Max

#![forbid(non_ascii_idents)]
#![deny(unsafe_code)]

use asterctl::cfg::{MonitorConfig, Sensor, load_custom_panel};
use asterctl::render::PanelRenderer;
use asterctl::sensors::start_sensor_poller;
use asterctl::{cfg, img};
use asterctl_lcd::{AooScreen, AooScreenBuilder, DISPLAY_SIZE};

use anyhow::anyhow;
use chrono::Timelike;
use clap::Parser;
use env_logger::Env;
use log::{debug, error, info, warn};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::{Duration, Instant};

/// AOOSTAR WTR MAX and GEM12+ PRO screen control.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Serial device, for example, "/dev/cu.usbserial-AB0KOHLS". Takes priority over --usb option.
    #[arg(short, long)]
    device: Option<String>,

    /// USB serial UART "vid:pid" in hex notation (lsusb output). Default: 416:90A1
    #[arg(short, long)]
    usb: Option<String>,

    /// Switch display on and exit. This will show the last displayed image.
    #[arg(long)]
    on: bool,

    /// Switch display off and exit.
    #[arg(long)]
    off: bool,

    /// Image to display, other sizes than 960x376 will be scaled.
    #[arg(short, long)]
    image: Option<String>,

    /// AOOSTAR-X json configuration file to parse.
    ///
    /// The configuration file will be loaded from the `config_dir` directory if no full path is
    /// specified.
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Include one or more additional custom panels into the base configuration.
    ///
    /// Specify the path to the panel directory containing panel.json and fonts / img subdirectories.
    #[arg(short, long)]
    panels: Option<Vec<PathBuf>>,

    /// Configuration directory containing configuration files and background images
    /// specified in the `config` file.
    #[arg(long, default_value_t = String::from("cfg"))]
    config_dir: String, // default_value_t requires Display trait which PathBuf does not implement

    /// Font directory for fonts specified in the `config` file.
    #[arg(long, default_value_t = String::from("fonts"))]
    font_dir: String,

    /// Switch off display n seconds after loading image or running demo.
    #[arg(short, long)]
    off_after: Option<u32>,

    /// Test mode: only write to the display without checking response.
    #[arg(short, long)]
    write_only: bool,

    /// Test mode: save changed images in ./out folder.
    #[arg(short, long)]
    save: bool,

    /// Simulate serial port for testing and development, `--device` and `--usb` options are ignored.
    #[arg(long)]
    simulate: bool,
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    // initialize display with given UART port parameter
    let mut builder = AooScreenBuilder::new();
    builder.no_init_check(args.write_only);
    let mut screen = if args.simulate {
        builder.simulate()?
    } else if let Some(device) = args.device {
        builder.open_device(&device)?
    } else if let Some(usb) = args.usb {
        builder.open_usb_id(&usb)?
    } else {
        builder.open_default()?
    };

    // process simple commands
    if args.off {
        screen.off()?;
        return Ok(());
    } else if args.on {
        screen.on()?;
        return Ok(());
    }

    // switch on screen for remaining commands
    screen.init()?;

    if let Some(config) = args.config {
        info!("Starting sensor panel mode");
        let img_save_path = if args.save {
            let img_save_path = PathBuf::from("out");
            fs::create_dir_all(&img_save_path)?;
            Some(img_save_path)
        } else {
            None
        };

        let cfg_dir = PathBuf::from(args.config_dir);
        let font_dir = PathBuf::from(args.font_dir);
        let cfg = load_configuration(&config, &cfg_dir, args.panels)?;
        run_sensor_panel(
            &mut screen,
            cfg,
            cfg_dir,
            font_dir,
            img_save_path,
        )?;
        return Ok(());
    }

    if let Some(image) = args.image {
        info!("Loading and displaying background image {image}...");
        let rgb_img = img::load_image(&image, Some(DISPLAY_SIZE))?.to_rgb8();
        let timestamp = Instant::now();
        screen.send_image(&rgb_img)?;
        debug!("Image sent in {}ms", timestamp.elapsed().as_millis());
    }

    if let Some(off) = args.off_after {
        info!("Switching off display in {off}s");
        sleep(Duration::from_secs(off as u64));
        screen.off()?;
    }

    info!("Bye bye!");

    Ok(())
}

fn load_configuration<P: AsRef<Path>>(
    config: P,
    config_dir: P,
    panels: Option<Vec<PathBuf>>,
) -> anyhow::Result<MonitorConfig> {
    let config = config.as_ref();
    let config_dir = config_dir.as_ref();

    let mut cfg = if config.is_absolute() {
        cfg::load_cfg(config)?
    } else {
        cfg::load_cfg(config_dir.join(config))?
    };

    if let Some(panels) = panels {
        for panel in panels {
            cfg.include_custom_panel(load_custom_panel(panel)?);
        }
    }

    // Compile sensor filter regexes from inline config
    if cfg.compile_sensor_filters() {
        info!("Using sensor filter from config");
    }

    Ok(cfg)
}

fn run_sensor_panel<B: Into<PathBuf>>(
    screen: &mut AooScreen,
    cfg: MonitorConfig,
    config_dir: B,
    font_dir: B,
    img_save_path: Option<B>,
) -> anyhow::Result<()> {
    let font_dir = font_dir.into();
    let config_dir = config_dir.into();
    let img_save_path = img_save_path.map(|p| p.into());

    let mut renderer = PanelRenderer::new(DISPLAY_SIZE, &font_dir, &config_dir);
    if let Some(img_save_path) = &img_save_path {
        renderer.set_img_save_path(img_save_path);
        renderer.set_save_render_img(true);
        // renderer.set_save_processed_pic(true);
        // renderer.set_save_progress_layer(true);
    }

    let sensor_values: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));

    let poller_refresh = Duration::from_millis((cfg.setup.refresh * 1000f32) as u64);
    start_sensor_poller(
        sensor_values.clone(),
        poller_refresh,
        cfg.sensor_filter.clone(),
    )?;

    let refresh = Duration::from_millis((cfg.setup.refresh * 1000f32) as u64);
    let sensor_page_time =
        Duration::from_secs_f32(cfg.setup.sensor_page_time.unwrap_or(10.0));
    let time_page_time = Duration::from_secs_f32(
        cfg.setup.time_page_time.unwrap_or(cfg.setup.sensor_page_time.unwrap_or(10.0)),
    );

    // Compile sensor template patterns from active panels
    let templates = compile_sensor_templates(&cfg);
    info!("Compiled {} sensor templates", templates.len());

    // Wait for initial sensor data to be available
    sleep(Duration::from_millis(1500));

    // Log all discovered sensor keys
    {
        let values = sensor_values.read().expect("RwLock is poisoned");
        let mut keys: Vec<&String> = values.keys().collect();
        keys.sort();
        info!("Discovered {} sensor keys:", keys.len());
        for key in &keys {
            info!("  {}: {}", key, values.get(*key).map(|v| v.as_str()).unwrap_or("N/A"));
        }
    }

    // Build initial page list from discovered sensors
    let mut pages = build_pages(&templates, &sensor_values, &cfg);
    if pages.is_empty() {
        return Err(anyhow!("No pages to display (no sensors matched any template)"));
    }

    info!(
        "Sensor page mode: {} pages, sensor={:.1}s, time={:.1}s",
        pages.len(),
        sensor_page_time.as_secs_f32(),
        time_page_time.as_secs_f32()
    );

    let time_font_size = cfg.setup.time_page_font_size;
    let mut display_off = false;

    if cfg.setup.display_on_hour.is_some() || cfg.setup.display_off_hour.is_some() {
        info!(
            "Display schedule: on={}, off={}",
            cfg.setup.display_on_hour.map_or("always".to_string(), |h| format!("{h}:00")),
            cfg.setup.display_off_hour.map_or("never".to_string(), |h| format!("{h}:00")),
        );
    }

    // page cycling loop
    let mut page_idx = 0;
    loop {
        // Rebuild pages periodically to pick up new sensors
        if page_idx == 0 {
            let new_pages = build_pages(&templates, &sensor_values, &cfg);
            if !new_pages.is_empty() {
                pages = new_pages;
            }
        }

        if page_idx >= pages.len() {
            page_idx = 0;
        }

        let page = &pages[page_idx];

        match page {
            PageKind::Sensor(sp) => {
                let value = sensor_values
                    .read()
                    .expect("RwLock is poisoned")
                    .get(&sp.sensor_key)
                    .cloned()
                    .unwrap_or_else(|| "N/A".to_string());
                info!(
                    "Page {}/{}: '{}' [{}] = {}",
                    page_idx + 1,
                    pages.len(),
                    sp.display_name,
                    sp.sensor_key,
                    value
                );
            }
            PageKind::Time(label) => {
                info!("Page {}/{}: time ({})", page_idx + 1, pages.len(), label);
            }
        }

        let page_start = Instant::now();
        let mut refresh_count = 1;

        // refresh loop for current page
        loop {
            let upd_start_time = Instant::now();

            if img_save_path.is_some() {
                renderer.set_img_suffix(format!("-{refresh_count:02}"));
            }

            // Check display schedule: turn display on/off based on hour range
            let display_on = is_display_active(&cfg);
            if !display_on {
                if !display_off {
                    info!("Display schedule: turning off");
                    screen.off()?;
                    display_off = true;
                }
                let page_duration = match page {
                    PageKind::Sensor(_) => sensor_page_time,
                    PageKind::Time(_) => time_page_time,
                };
                sleep(Duration::from_secs(30));
                if page_start.elapsed() >= page_duration {
                    break;
                }
                continue;
            } else if display_off {
                info!("Display schedule: turning on");
                screen.on()?;
                display_off = false;
            }

            let rendered = match page {
                PageKind::Sensor(sp) => {
                    let values = sensor_values.read().expect("RwLock is poisoned");
                    renderer.render_sensor_page_from_template(
                        &sp.template,
                        &sp.sensor_key,
                        &sp.display_name,
                        &values,
                        cfg.setup.sensor_page_label.as_ref(),
                    )
                }
                PageKind::Time(label) => {
                    renderer.render_time_page(label, time_font_size)
                }
            };

            match rendered {
                Ok(image) => {
                    screen.send_image(&image)?;
                }
                Err(e) => error!("Error rendering page: {e:?}"),
            }

            let elapsed = upd_start_time.elapsed();
            if refresh > elapsed {
                sleep(refresh - elapsed);
            }

            let page_duration = match page {
                PageKind::Sensor(_) => sensor_page_time,
                PageKind::Time(_) => time_page_time,
            };
            if page_start.elapsed() >= page_duration {
                break;
            }

            refresh_count += 1;
        }

        page_idx = (page_idx + 1) % pages.len();
    }
}

enum PageKind {
    Sensor(SensorPage),
    Time(String),
}

struct SensorPage {
    sensor_key: String,
    display_name: String,
    template: Sensor,
}

struct CompiledTemplate {
    regex: Regex,
    sensor: Sensor,
}

/// Compile regex patterns from sensor templates in active panels.
fn compile_sensor_templates(cfg: &MonitorConfig) -> Vec<CompiledTemplate> {
    let mut templates = Vec::new();
    for &active in &cfg.active_panels {
        if active == 0 || active > cfg.panels.len() as u32 {
            continue;
        }
        let panel = &cfg.panels[active as usize - 1];
        for sensor in &panel.sensor {
            if let Some(pattern) = &sensor.match_pattern {
                match Regex::new(pattern) {
                    Ok(re) => templates.push(CompiledTemplate {
                        regex: re,
                        sensor: sensor.clone(),
                    }),
                    Err(e) => warn!("Invalid sensor match pattern '{pattern}': {e}"),
                }
            }
        }
    }
    templates
}

/// Build pages by matching available sensor keys against compiled templates.
/// Templates are matched in order; each sensor key matches at most one template.
fn build_pages(
    templates: &[CompiledTemplate],
    sensor_values: &Arc<RwLock<HashMap<String, String>>>,
    cfg: &MonitorConfig,
) -> Vec<PageKind> {
    let values = sensor_values.read().expect("RwLock is poisoned");
    let mut sensor_keys: Vec<&String> = values.keys().collect();
    sensor_keys.sort();

    // For each template (in order), find all matching sensor keys.
    // This preserves template order as the primary sort.
    let mut matched_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut pages: Vec<PageKind> = Vec::new();

    for tmpl in templates {
        let mut matches: Vec<(&String, String)> = Vec::new();
        for key in &sensor_keys {
            if matched_keys.contains(*key) {
                continue;
            }
            if let Some(caps) = tmpl.regex.captures(key) {
                let display_name = expand_template_name(&tmpl.sensor, &caps);
                matches.push((key, display_name));
            }
        }
        for (key, display_name) in matches {
            matched_keys.insert(key.clone());
            pages.push(PageKind::Sensor(SensorPage {
                sensor_key: key.clone(),
                display_name,
                template: tmpl.sensor.clone(),
            }));
        }
    }

    // Add optional time page at the end
    if let Some(time_label) = &cfg.setup.time_page {
        pages.push(PageKind::Time(time_label.clone()));
    }

    info!("Built {} pages from {} sensor keys", pages.len(), sensor_keys.len());
    pages
}

/// Expand the template display name using regex capture groups.
/// `{1}`, `{2}`, etc. in the sensor `name` are replaced with capture group values.
fn expand_template_name(sensor: &Sensor, caps: &regex::Captures) -> String {
    let base_name = sensor
        .name
        .as_deref()
        .or(sensor.item_name.as_deref())
        .unwrap_or("Sensor");

    let mut result = base_name.to_string();
    for i in 1..=9 {
        let placeholder = format!("{{{i}}}");
        if let Some(m) = caps.get(i) {
            result = result.replace(&placeholder, m.as_str());
        }
    }
    result
}

/// Check if the display should be active based on the configured hour range.
///
/// - If both `display_on_hour` and `display_off_hour` are set, the display is active
///   when the current hour is within `[on_hour, off_hour)`.
///   Supports wrap-around (e.g., on=22, off=6 means active from 22:00 to 05:59).
/// - If only `display_on_hour` is set, the display is active from that hour onwards.
/// - If only `display_off_hour` is set, the display is active until that hour.
/// - If neither is set, the display is always active.
fn is_display_active(cfg: &MonitorConfig) -> bool {
    let (on_hour, off_hour) = match (cfg.setup.display_on_hour, cfg.setup.display_off_hour) {
        (None, None) => return true,
        (Some(on), None) => return chrono::Local::now().hour() >= on,
        (None, Some(off)) => return chrono::Local::now().hour() < off,
        (Some(on), Some(off)) => (on, off),
    };

    let hour = chrono::Local::now().hour();
    if on_hour <= off_hour {
        // e.g., on=8, off=22 → active during 08:00–21:59
        hour >= on_hour && hour < off_hour
    } else {
        // e.g., on=22, off=6 → active during 22:00–05:59
        hour >= on_hour || hour < off_hour
    }
}
