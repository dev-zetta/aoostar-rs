# Sensor Panels

The `asterctl` tool is started in sensor panel mode if the `--config` command line option is specified.

Sensor panels are dynamic screens showing various sensor values. Sensor values are read directly from
the system using the integrated [aster-sysinfo](provider/sysinfo.md) library — no external scripts
or intermediate files are needed.

Sensor entries in the configuration act as templates using regex `match` patterns. At runtime, all
discovered sensor keys are matched against these templates to dynamically build display pages.

Example panels from the AOOSTAR-X software, rendered with `asterctl` using dummy sensor values:

<img src="../img/sensor_panel-01.png" alt="Sensor panel 1">

<img src="../img/sensor_panel-02.png" alt="Sensor panel 1">

See [custom sensor panels](custom_panel.md) for including custom panels.

## Supported Features

- Template-based sensor display with regex pattern matching and auto-discovery.
- Sensor pages rotate at a configurable interval (`setup.sensorPageTime`).
- Optional dedicated clock/time page with a separate display duration (`setup.timePageTime`).
- Display schedule to automatically turn the LCD on/off at specific hours.
- Each sensor template can be customized with an individual font, size, color and text alignment.
  - Text sensor value fields (`sensor.mode: 1`) are fully supported.
  - Fan (2), progress (3) and pointer (4) sensor modes are being worked on and not all configuration options are working yet.
- Panels are redrawn at a configurable interval (`setup.refresh`).
  - Only the updated areas of the image are sent to the display for faster updates.

See [Sensor Configuration](cfg/) for more information.
