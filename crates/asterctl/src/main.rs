// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2025 Markus Zehnder

#![forbid(non_ascii_idents)]
#![deny(unsafe_code)]

use asterctl::cfg::{MonitorConfig, load_custom_panel};
use asterctl::render::PanelRenderer;
use asterctl::sensors::{read_filter_file, read_key_value_file, start_file_slurper};
use asterctl::{cfg, img};
use asterctl_lcd::{AooScreen, AooScreenBuilder, DISPLAY_SIZE};

use anyhow::anyhow;
use clap::Parser;
use env_logger::Env;
use log::{debug, error, info};
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

    /// Single sensor value input file or directory for multiple sensor input files.
    #[arg(long, default_value_t = String::from("cfg/sensors"))]
    sensor_path: String,

    /// Sensor identifier mapping file. Ignored if the file does not exist.
    ///
    /// The configuration file will be loaded from the `config_dir` directory if no full path is
    /// specified.
    #[arg(long, default_value_t = String::from("sensor-mapping.cfg"))]
    sensor_mapping: String,

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
        let sensor_path = PathBuf::from(args.sensor_path);
        let mapping_cfg = PathBuf::from(args.sensor_mapping);
        let cfg = load_configuration(&config, &cfg_dir, args.panels, &mapping_cfg)?;
        run_sensor_panel(
            &mut screen,
            cfg,
            cfg_dir,
            font_dir,
            sensor_path,
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
    sensor_mapping: P,
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

    let sensor_mapping = sensor_mapping.as_ref();
    let mapping_cfg = if sensor_mapping.is_absolute() {
        sensor_mapping.to_path_buf()
    } else {
        config_dir.join(sensor_mapping)
    };
    if mapping_cfg.is_file() {
        let mut mapping = HashMap::new();
        read_key_value_file(&mapping_cfg, &mut mapping, None)?;
        cfg.set_sensor_mapping(mapping);
    } else {
        info!("Sensor mapping file {mapping_cfg:?} not found");
    }

    cfg.sensor_filter = load_sensor_filter(&mapping_cfg)?;

    Ok(cfg)
}

fn load_sensor_filter(mapping_cfg: &Path) -> anyhow::Result<Option<Vec<Regex>>> {
    if let Some(parent) = mapping_cfg.parent()
        && let Some(file_stem) = mapping_cfg.file_stem()
        && let Some(extension) = mapping_cfg.extension()
    {
        let filter_file = parent
            .join(format!("{}-filter", file_stem.to_string_lossy()))
            .with_extension(extension);

        if filter_file.is_file() {
            info!("Loading sensor filter file {filter_file:?}");
            return read_filter_file(filter_file);
        } else {
            info!("No sensor filter file {filter_file:?} available");
        }
    }

    Ok(None)
}

fn run_sensor_panel<B: Into<PathBuf>>(
    screen: &mut AooScreen,
    cfg: MonitorConfig,
    config_dir: B,
    font_dir: B,
    sensor_path: B,
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

    start_file_slurper(
        sensor_path,
        sensor_values.clone(),
        cfg.sensor_filter.clone(),
    )?;

    let refresh = Duration::from_millis((cfg.setup.refresh * 1000f32) as u64);
    let sensor_page_time =
        Duration::from_secs_f32(cfg.setup.sensor_page_time.unwrap_or(10.0));

    // Collect all pages: sensor pages from active panels + optional time page
    let mut pages: Vec<PageKind> = Vec::new();
    for &active in &cfg.active_panels {
        if active == 0 || active > cfg.panels.len() as u32 {
            continue;
        }
        let panel_idx = active as usize - 1;
        let panel = &cfg.panels[panel_idx];
        for sensor_idx in 0..panel.sensor.len() {
            pages.push(PageKind::Sensor(panel_idx, sensor_idx));
        }
    }

    if let Some(time_label) = &cfg.setup.time_page {
        info!("Time page enabled: {time_label}");
        pages.push(PageKind::Time(time_label.clone()));
    }

    if pages.is_empty() {
        return Err(anyhow!("No pages to display"));
    }

    info!(
        "Sensor page mode: {} pages, cycling every {:.1}s",
        pages.len(),
        sensor_page_time.as_secs_f32()
    );

    // page cycling loop
    let mut page_idx = 0;
    loop {
        let page = &pages[page_idx];

        match page {
            PageKind::Sensor(panel_idx, sensor_idx) => {
                let panel = &cfg.panels[*panel_idx];
                info!(
                    "Page {}/{}: panel '{}', sensor '{}'",
                    page_idx + 1,
                    pages.len(),
                    panel.friendly_name(),
                    panel.sensor[*sensor_idx].label
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

            match page {
                PageKind::Sensor(panel_idx, sensor_idx) => {
                    let panel = &cfg.panels[*panel_idx];
                    let values = sensor_values.read().expect("RwLock is poisoned");
                    match renderer.render_sensor_page(panel, *sensor_idx, &values) {
                        Ok(image) => screen.send_image(&image)?,
                        Err(e) => error!(
                            "Error rendering sensor page '{}'/[{}]: {e:?}",
                            panel.friendly_name(),
                            sensor_idx
                        ),
                    }
                }
                PageKind::Time(label) => {
                    match renderer.render_time_page(label) {
                        Ok(image) => screen.send_image(&image)?,
                        Err(e) => error!("Error rendering time page: {e:?}"),
                    }
                }
            }

            let elapsed = upd_start_time.elapsed();
            if refresh > elapsed {
                sleep(refresh - elapsed);
            }

            if page_start.elapsed() >= sensor_page_time {
                break;
            }

            refresh_count += 1;
        }

        page_idx = (page_idx + 1) % pages.len();
    }
}

enum PageKind {
    Sensor(usize, usize),
    Time(String),
}
