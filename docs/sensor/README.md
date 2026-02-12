# Sensor Panels

- [Sensor panels](panel.md)
- [Custom sensor panels](custom_panel.md)
- [Configuration](cfg/)

## Sensor Modes

Different sensor modes are supported:

- [Sensor mode 1: Text](cfg/mode1_text.md)
- [Sensor mode 2: Circular Progress](cfg/mode2_circular.md)
- [Sensor mode 3: Progress](cfg/mode3_progress.md)
- [Sensor mode 4: Pointer](cfg/mode4_pointer.md)

## Sensor Data Sources

Sensor values are read directly from the system by `asterctl` using the integrated
[aster-sysinfo](../../crates/aster-sysinfo) library. No external scripts or intermediate text files are needed.

The `aster-sysinfo` library uses the [sysinfo](https://github.com/GuillaumeGomez/sysinfo) crate to collect:
- CPU usage and temperature
- Memory usage
- Disk usage and NVMe temperatures
- Network interface addresses, upload/download speeds
- Hardware component temperatures

Additionally, internal [date time sensors](provider/internal_date_time.md) are available for displaying the current
date and time on a dedicated time page.

### Template-Based Sensor Display

Sensor entries in `monitor.json` act as display templates using regex `match` patterns.
At runtime, all discovered sensor keys are matched against these templates to dynamically build display pages.

One template can match multiple sensors. For example, a single NVMe temperature template matches all NVMe drives:

```json
{
  "mode": 1,
  "match": "^temperature_nvme_Composite_(.+)$",
  "name": "NVMe {1}",
  "fontSize": 80,
  "unit": " °C"
}
```

Regex capture groups can be referenced in the `name` field using `{1}`, `{2}`, etc.

### Sensor Filter

Sensor keys can be filtered using regular expressions defined inline in `monitor.json` via the `sensorFilter` array:

```json
"sensorFilter": [
  "^temperature_.*#unit"
]
```

This removes all sensors starting with `temperature_` and ending with `#unit`, ensuring temperature sensors are
rendered without the unit text suffix.

### Legacy Data Providers

The following data providers are no longer needed but are kept for reference:

- [Text file data source](provider/text_file.md) (legacy)
- [Linux shell scripts](provider/shell_scripts.md) (legacy)
- [aster-sysinfo CLI tool](provider/sysinfo.md) (standalone mode still available for debugging)
