# smc

[![crates.io](https://img.shields.io/crates/v/smc.svg)](https://crates.io/crates/smc)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Read and write Apple System Management Controller (SMC) values on macOS.

The SMC is a hardware controller on Apple systems responsible for temperature
sensors, fan control, power management, port status, and more. This library
provides a safe Rust interface to the AppleSMC kernel driver via IOKit.

Supports **x86_64** and **aarch64** (Apple Silicon) architectures.

## Requirements

- macOS only
- Rust 1.57+

## Installation

```toml
[dependencies]
smc = "1.0"
```

## Quick Start

```rust
use four_char_code::four_char_code;
use smc::SMC;

let smc = SMC::new().unwrap();

// Read CPU temperature (TC0P)
let temp: f32 = smc.read_key(four_char_code!("TC0P")).unwrap();
println!("CPU temp: {} °C", temp);

// Check if optical disk drive is full
if smc.is_optical_disk_drive_full().unwrap() {
    println!("Optical disk drive is full (has media)");
}

// Enumerate fans
for fan in smc.fans().unwrap() {
    let fan = fan.unwrap();
    println!("Fan {}: {} rpm", fan.id(), fan.rpm(&smc).unwrap());
}
```

## Feature Flags

| Flag   | Description                                        |
|--------|----------------------------------------------------|
| (none) | Default: `no_std` compatible                       |
| `std`  | Enables `std::error::Error` impl for `SMCError` and `GreaterThan10`/`MoreDigits` |

## API Overview

### Reading SMC Keys

Read any SMC key by its FourCharCode. The library handles type conversion
automatically:

```rust
use four_char_code::four_char_code;
use smc::SMC;

let smc = SMC::new().unwrap();

// Supported target types:
let temp: f32 = smc.read_key(four_char_code!("TC0P"))?;    // f32 (FPE2, SP78, FLT)
let flag: bool = smc.read_key(four_char_code!("MSDI"))?;   // bool (FLAG)
let count: u32 = smc.read_key(four_char_code!("FNum"))?;   // u32 (U8, U16, U32)
let value: i64 = smc.read_key(four_char_code!("...

// Raw SMCVal for custom parsing
let raw: smc::SMCVal = smc.read_key(four_char_code!("...

// Constrained types
let fan_id: smc::OneDigit = smc.read_key(four_char_code!("F0ID"))?;
let fan_count: smc::UMax10 = smc.read_key(four_char_code!("FNum"))?;
```

**Supported conversions (`FromSMC`):**
- `bool` ← `flag`
- `u8`, `i8` ← `ui8 `, `si8 `
- `u16` ← `ui8 `, `ui16`
- `i16` ← `ui8 `, `si8 `, `si16`
- `u32` ← `ui8 `, `ui16`, `ui32`
- `i32` ← `ui8 `, `si8 `, `ui16`, `si16`, `si32`
- `i64` ← all unsigned and signed integer types
- `f32` ← `fpe2` (÷4), `sp78` (÷256), `flt ` (native LE)
- `UMax10` ← `0..=10` (fan count/index bounded)
- `OneDigit` ← `0..=9` (fan ID bounded)

### Writing SMC Keys

Writing requires `unsafe` — modifying the SMC can affect system stability
(e.g. overheating, unexpected shutdowns).

```rust
use four_char_code::four_char_code;
use smc::SMC;

let mut smc = SMC::new().unwrap();

// Unsafe: write a value to an SMC key
unsafe {
    smc.write_key(four_char_code!("F0Tg"), 2500.0f32)?;
}
```

### Fan Control

High-level API for monitoring and controlling fans. **Fan control requires
root privileges.**

```rust
use smc::SMC;

let smc = SMC::new().unwrap();

// Iterate all fans with speed percentage
for fan in smc.fans()? {
    let fan = fan?;
    println!(
        "{}) {} {}%",
        fan.id(),
        std::str::from_utf8(fan.name()).unwrap(),
        fan.percent(&smc)?
    );
}

// Get snapshot with cached values
let info = smc.get_fan_info(0.try_into().unwrap())?;
println!("Fan 0: {:.0} / {:.0} RPM ({:.1}%)",
    info.current_speed(),
    info.max_speed(),
    info.percent());

// Set fan speed (requires root)
let fan = smc.get_fan(0.try_into().unwrap())?;
fan.set_managed(&mut smc, false)?;  // switch to manual
fan.set_current_speed(&mut smc, 3000.0)?;
```

**`SMC` methods for fans:**

| Method                          | Returns            |
|---------------------------------|--------------------|
| `smc.fans_len()`                | `usize`            |
| `smc.get_fan(id)`               | `Fan`              |
| `smc.get_fan_min_speed(id)`     | `f32`              |
| `smc.get_fan_max_speed(id)`     | `f32`              |
| `smc.get_fan_current_speed(id)` | `f32`              |
| `smc.get_fan_info(id)`          | `FanInfo`          |
| `smc.fans()`                    | `Fans` iterator    |
| `smc.fan_infos()`               | `FanInfos` iterator|
| `smc.managed_fans()`            | `u16` bitmask      |
| `smc.fan_set_managed(id, bool)` | `()`               |

### Enumerating All SMC Keys

```rust
use smc::{FromSMC, SMC, SMCVal};

let smc = SMC::new()?;
for key in smc.keys()? {
    let key = key?;
    let val: SMCVal = smc.read_key(key)?;
    println!("{} [{}] {:02x?}", key, val.r#type, val.data());
}
```

### Utility Functions

The `util` module provides serialization helpers for custom `IntoSMC`
implementations:

- `write_f32(n, val)` — FPE2, SP78, FLT
- `write_u32(n, val)` — U32
- `write_i32(n, val)` — I32
- `write_u16(n, val)` — U16, widened to I32/U32
- `write_i16(n, val)` — I16, widened to I32
- `write_u8(n, val)` — U8, widened to I16/U16
- `write_i8(n, val)` — I8, widened to I16
- `write_bool(n, val)` — FLAG

### Error Handling

```rust
use smc::SMCError;

match smc.read_key::<f32>(four_char_code!("BAD!")) {
    Err(SMCError::DriverNotFound) => eprintln!("AppleSMC driver unavailable"),
    Err(SMCError::KeyNotFound(key)) => eprintln!("Key {} not found", key),
    Err(SMCError::NotPrivileged) => eprintln!("Root required"),
    Err(SMCError::TryFrom(val)) => eprintln!("Cannot parse {:?}", val),
    Err(e) => eprintln!("SMC error: {}", e),
    Ok(v) => println!("Value: {}", v),
}
```

| Error                | Cause                                |
|----------------------|--------------------------------------|
| `DriverNotFound`     | AppleSMC driver not available         |
| `Open`              | Failed to open IOKit connection       |
| `InvalidKey`        | Invalid `FourCharCode` string         |
| `KeyNotFound`       | Key does not exist on this hardware   |
| `NotPrivileged`     | Operation requires root               |
| `TryFrom`           | Cannot convert raw value to type      |
| `TryInto`           | Cannot convert value for writing      |
| `Unknown(i32, u8)`  | IOKit/SMC error codes                 |

## Examples

```sh
# List fan speeds
cargo run --example fans

# Dump all SMC keys with values
cargo run --example list
```

## Common SMC Keys

| Key   | Type     | Description                    |
|-------|----------|--------------------------------|
| `TC0P`| `sp78`  | CPU proximity temperature      |
| `TC0c`| `sp78`  | CPU core temperature           |
| `TM0P`| `sp78`  | Memory proximity temperature   |
| `TN0P`| `sp78`  | Northbridge proximity temp     |
| `FNum`| `ui8 `  | Number of fans                 |
| `F0ID`| `{fds`  | Fan 0 name                     |
| `F0Ac`| `fpe2`  | Fan 0 current speed (RPM)      |
| `F0Mn`| `fpe2`  | Fan 0 minimum speed            |
| `F0Mx`| `fpe2`  | Fan 0 maximum speed            |
| `FS! `| `ui16`  | Fan manual/auto control mask   |
| `MSDI`| `flag`  | Optical disk media present     |

For a full dump of keys on your machine: `cargo run --example list`

## License

MIT
