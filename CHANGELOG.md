# AOOSTAR WTR MAX Screen Control Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Changed
- **Direct sensor reading**: `asterctl` now reads system sensors directly via the `aster-sysinfo` library crate,
  eliminating the need for external scripts, intermediate text files, and the file watcher.
  The `aster-sysinfo` crate is now both a standalone CLI tool and a library used by `asterctl`.
- **Template-based sensor display**: Sensor entries in `monitor.json` now use regex `match` patterns
  instead of fixed labels. One template can match multiple sensors (e.g., all NVMe temperatures,
  all SSD usage values). Capture groups can be referenced in display names with `{1}`, `{2}`, etc.
- **Auto-discovery**: Pages are built dynamically at runtime by matching discovered sensor keys
  against templates. No more hardcoded sensor mappings or duplicate entries.
- Sensor label font size increased for better readability on the 960x376 display.
- Time page font size is now configurable via `timePageFontSize` in `monitor.json`.

### Added
- **Display schedule**: New `displayOnHour` / `displayOffHour` settings in `monitor.json` to
  automatically turn the LCD on/off at specific hours (e.g., on at 8:00, off at 23:00).
- **Separate time page duration**: New `timePageTime` setting in `monitor.json` to control how long
  the clock page is displayed, independent of `sensorPageTime`. Defaults to `sensorPageTime` if not set.
- `render_sensor_page_from_template` rendering method for template-based sensor pages.

### Removed
- **Sensor mapping**: The `sensorMapping` section in `monitor.json` and the `--sensor-mapping` CLI
  argument have been removed. Sensor templates with `match` patterns replace the old mapping approach.
- **File-based sensor reading**: The file slurper, file watcher (`notify` dependency), and all related
  code (`start_file_slurper`, `read_key_value_file`, `read_filter_file`, `read_path`) have been removed.
- External sensor mapping and filter configuration files are no longer used.
  Sensor filters are now defined inline in `monitor.json` via the `sensorFilter` array.
- Unused `switchTime` configuration field removed from `Setup`.

## v0.2.0 - 2025-08-31
### Fixed
- Misplaced text sensors in custom panels ([#11](https://github.com/zehnm/aoostar-rs/issues/11)).
- Wrong start position for circular progress (fan) sensor using a counter-clockwise direction ([#12](https://github.com/zehnm/aoostar-rs/issues/12)).
- aster-sysinfo tool: make sensor file world-readable, create all parent directories.

### Added
- Simple sensor panel with a file-based data source ([#6](https://github.com/zehnm/aoostar-rs/issues/6)). 
- Initial support for fan-, progress-, & pointer-sensors ([#8](https://github.com/zehnm/aoostar-rs/pull/8)).
- Use [mdBook](https://rust-lang.github.io/mdBook/) for documentation and publish user guide to GitHub pages ([#10](https://github.com/zehnm/aoostar-rs/pull/10)).
- Initial `aster-sysinfo` tool for providing sensor values in a text file for `asterctl`.

### Changed
- Project structure using a Cargo workspace.

---

## v0.1.0 - 2025-08-02
### Added
- Initial `asterctl` tool release for controlling the LCD: on, off, display an image.
- systemd service file to switch off LCD on system start.
- Demo mode.
