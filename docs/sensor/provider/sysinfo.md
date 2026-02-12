# aster-sysinfo

The [aster-sysinfo](https://github.com/zehnm/aoostar-rs/blob/main/crates/aster-sysinfo) crate gathers system sensor
values using the [sysinfo](https://github.com/GuillaumeGomez/sysinfo) crate.

It serves two purposes:

1. **Library** (primary): Used by `asterctl` for direct, in-process sensor polling. Sensor values are read from the
   system and stored in a shared HashMap, with no intermediate files or external processes needed.
2. **CLI tool**: Standalone binary useful for debugging and inspecting all available sensor keys on your system.

## Integrated Usage

When running `asterctl` in sensor panel mode, `aster-sysinfo` is used as a library automatically.
A background thread polls sensors at the configured refresh interval and updates the shared sensor value store.
No separate process or configuration is needed.

## Standalone CLI

The CLI tool is useful for discovering available sensor keys on your system, which you can then reference in
`monitor.json` template `match` patterns.

Print all sensors once to the console:
```shell
aster-sysinfo --console
```

Write sensor values to a file (legacy mode, for debugging):
```shell
aster-sysinfo --refresh 3 --out /tmp/sensors/aster-sysinfo.txt
```

Note: the lower the refresh rate, the more resources are used!
