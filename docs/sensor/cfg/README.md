# Sensor Configuration

Specify configuration file to use:
```shell
asterctl --config monitor.json
```

- The configuration file is loaded from the configuration directory if not an absolute path is specified.
- The default configuration directory is `./cfg` and can be changed with the `--config-dir` command line option.

Example configuration file: [cfg/monitor.json](https://github.com/zehnm/aoostar-rs/blob/main/cfg/monitor.json).

## Setup

The `setup` object configures global display behavior:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `refresh` | float | `1` | Panel redraw interval in seconds. |
| `sensorPageTime` | float | `10` | Time in seconds to display each sensor page before cycling to the next. |
| `timePageTime` | float | *sensorPageTime* | Time in seconds to display the clock page. Defaults to `sensorPageTime` if not set. |
| `timePage` | string | *(none)* | Date/time format label for a dedicated clock page (e.g., `"DATE_h_m_s_1"`). If empty or not set, no clock page is shown. |
| `timePageFontSize` | float | `64` | Font size for the clock page. |
| `displayOnHour` | int | *(none)* | Hour (0–23) when the display turns on. |
| `displayOffHour` | int | *(none)* | Hour (0–23) when the display turns off. |
| `sensorPageLabel` | object | *(none)* | Configuration for the sensor name label shown above the value. See below. |

### Sensor Page Label

The optional `sensorPageLabel` object controls the sensor name text displayed above the sensor value:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fontFamily` | string | system default | Font family name. |
| `fontSize` | float | `28` | Font size in points. |
| `fontColor` | string | `#b4b4b4` | Font color in `#RRGGBB` notation. |
| `x` | int | *(centered)* | Horizontal position. Centered if not set. |
| `y` | int | `40` | Vertical position. |

### Display Schedule

If `displayOnHour` and/or `displayOffHour` are set, the LCD is automatically turned on/off:
- Both set: display is active during `[onHour, offHour)`. Supports wrap-around (e.g., on=22, off=6).
- Only `displayOnHour`: display is active from that hour onwards.
- Only `displayOffHour`: display is active until that hour.
- Neither set: display is always on.

## Sensor Filter

The optional `sensorFilter` array contains regex patterns. Sensor keys matching any pattern are excluded:

```json
"sensorFilter": [
  "^temperature_.*#unit"
]
```

## Sensor Templates

Sensor entries in `diy[].sensor[]` act as display templates. Each template uses a regex `match` pattern
to select which sensor keys it applies to. At runtime, all discovered sensor keys are matched against
templates to dynamically build display pages.

### Template Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `match` | string | *(required)* | Regex pattern to match sensor keys. Capture groups can be used. |
| `name` | string | `"Sensor"` | Display name. Capture groups from `match` can be referenced as `{1}`, `{2}`, etc. |
| `mode` | int | | Sensor display mode: `1` = text, `2` = circular progress, `3` = progress bar, `4` = pointer. |
| `x` | int | | X-position of the sensor value. |
| `y` | int | | Y-position of the sensor value. |
| `fontFamily` | string | | Font name matching a font filename (without extension) in the font directory. |
| `fontSize` | float | | Font size. |
| `fontColor` | string/int | `#ffffff` | Font color in `#RRGGBB` notation, or `-1` for white. |
| `textAlign` | string | `"left"` | Text alignment: `left`, `right`, `center`. |
| `decimalDigits` | int | | Number of decimal places for the sensor value. |
| `integerDigits` | int | | Number of integer places (0-prefixed). |
| `unit` | string | | Unit label appended after the sensor value (e.g., `" °C"`, `" %"`). |

Additional fields for fan (2), progress (3) and pointer (4) modes:
- `min_value` and `max_value`
- `width` and `height`
- `direction`
- `pic`: progress image filename
- `min_angle` and `max_angle`
- `xz_x` and `xz_y`

### Example

```json
{
  "mode": 1,
  "match": "^temperature_nvme_Composite_(.+)$",
  "name": "NVMe {1}",
  "x": 480, "y": 200,
  "fontFamily": "HarmonyOS_Sans_SC_Bold",
  "fontSize": 80,
  "fontColor": -1,
  "textAlign": "center",
  "decimalDigits": 0,
  "unit": " °C"
}
```

This template matches all NVMe composite temperature sensors and displays each one with the drive
name extracted from the sensor key (e.g., `"NVMe KINGSTON_OM8PGP41024Q-A0"`).
