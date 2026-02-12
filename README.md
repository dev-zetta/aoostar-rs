# AOOSTAR WTR MAX / GEM12+ PRO Screen Control

Reverse engineering the [AOOSTAR WTR MAX](https://aoostar.com/products/aoostar-wtr-max-amd-r7-pro-8845hs-11-bays-mini-pc)
display protocol, with a proof-of-concept application written in Rust.  
It has only been tested on the WTR MAX, but should also support the GEM12+ PRO device.

Check out the **[User Guide](https://zehnm.github.io/aoostar-rs)** for a list of features and installation and usage information.

## Features

- Control the AOOSTAR WTR MAX and GEM12+ PRO second screen from Linux.
- Switch the display on or off.
    - Also possible with standard [Linux shell commands](docs/shell_commands.md).
    - [Linux systemd Service](docs/linux/README.md) to automatically switch off the LCD at boot up.
    - Configurable display schedule to automatically turn the LCD on/off at specific hours.
- Display images (with automatic scaling and partial update support).
- Render dynamic sensor pages with template-based configuration.
    - Direct system sensor reading via the integrated `aster-sysinfo` library (no external scripts or files needed).
    - Auto-discovery of sensors with regex-based template matching.
    - Rotate through sensor pages and an optional time page at a configurable interval.
- USB device/serial port selection.

## Disclaimer

> I take no responsibility for the use of this software.  
> There is no official documentation available;
> all display control commands have been reverse engineered from the original AOOSTAR-X software.

Even though this software works fine **for me**, I cannot guarantee that it is risk-free:

- It may or may not work.
- It could crash the display firmware, requiring a power cycle.
- It could even brick the display firmware.
- You have been warned!

The risk remains until the manufacturer provides official documentation, and the protocol can be reviewed.
Note: Multiple attempts to contact the manufacturer for documentation have received no response.

With that out of the way, on to the fun stuff!

- Browse the source code or read the [User Guide](https://zehnm.github.io/aoostar-rs)
- See [releases](https://github.com/zehnm/aoostar-rs/releases) for binary Linux x64 releases. A Debian package for easy installation is planned for the future!

## Running in a Proxmox LXC Container

The AOOSTAR WTR MAX / GEM12+ PRO LCD can be controlled from inside an LXC container running on Proxmox VE.
This requires passing through the USB serial device to the container.

### 1. Identify the USB device on the Proxmox host

```shell
lsusb | grep 0416
# Example output: Bus 001 Device 002: ID 0416:90a1
ls -l /dev/ttyACM0
# crw-rw---- 1 root dialout 166, 0 ...
```

### 2. Pass the device to the LXC container

Add the following to the container configuration on the Proxmox host
(`/etc/pve/lxc/<CTID>.conf`):

```
lxc.cgroup2.devices.allow: c 166:* rwm
lxc.mount.entry: /dev/ttyACM0 dev/ttyACM0 none bind,optional,create=file
```

- `166` is the major device number for `/dev/ttyACM0` (USB ACM devices).
- Use `ls -l /dev/ttyACM0` to verify the major number on your host.

### 3. Set permissions inside the container

After starting the container, ensure the device is accessible:

```shell
# Verify the device is visible
ls -l /dev/ttyACM0

# Add your user to the dialout group (if not running as root)
usermod -aG dialout <username>
```

### 4. Install and run asterctl

Copy or build the `asterctl` binary inside the container, then run it normally:

```shell
asterctl --config monitor.json
```

> **Note**: If the USB device is not present at container start (e.g., after a host reboot where the
> device enumerates late), you may need to restart the container or re-bind the device.

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please note that this software is currently in its initial development and will have major changes until the mentioned
goals above are reached!

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
