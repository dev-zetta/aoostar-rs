# System Sensor Provider for asterctl

This crate provides system sensor reading using the [sysinfo](https://github.com/GuillaumeGomez/sysinfo) crate.

It serves two purposes:
- **Library**: Used by `asterctl` for direct, in-process sensor polling (no intermediate files needed).
- **CLI tool**: Standalone binary for debugging and inspecting available sensor values.

See [README](../../README.md) in root directory for more information.

## Library Usage

The `SysinfoSource` struct provides direct access to system sensors:

```rust
use aster_sysinfo::SysinfoSource;
use std::collections::HashMap;

let mut source = SysinfoSource::new();
source.refresh();

let mut sensors = HashMap::new();
source.update_sensors(&mut sensors).unwrap();

for (key, value) in &sensors {
    println!("{key}: {value}");
}
```

`asterctl` uses this library internally via `start_sensor_poller()` to continuously poll sensors
in a background thread.

## CLI Usage

Single test run printing all sensors to the console:
```shell
aster-sysinfo --console
```

Continuous mode writing sensor values to a file (legacy, for debugging):
```shell
aster-sysinfo --refresh 3 --out /tmp/sensors/aster-sysinfo.txt
```
