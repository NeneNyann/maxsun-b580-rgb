# MAXSUN B580 RGB

English | [简体中文](README.zh-CN.md)  

A personal Windows CLI written in Rust for the MAXSUN Intel Arc B580 iCraft 12G.

I wanted to turn off the GPU's RGB without running the official lighting software or leaving its driver installed. The CLI temporarily loads the official I2C driver, performs the operation, removes the driver package and exits. No background process is required.

## Build

Requires Rust/Cargo (`x86_64-pc-windows-msvc`), MSVC and the Windows SDK. Install the "Desktop development with C++" workload in Visual Studio Build Tools.

No prebuilt releases are provided to avoid redistributing the official driver embedded in the executable. The build script downloads the driver from the official source and builds locally.

```powershell
.\build.ps1
```

## Usage

Run in an Administrator PowerShell:

```powershell
# Probe the controller
.\maxsun-b580-rgb.exe probe

# Turn off RGB
.\maxsun-b580-rgb.exe off

# Solid red
.\maxsun-b580-rgb.exe set FF0000

# Static red-to-blue gradient
.\maxsun-b580-rgb.exe gradient FF0000 0000FF

# Static rainbow
.\maxsun-b580-rgb.exe rainbow-static

# Custom frame: 30 RRGGBB values, here 10 red, 10 green and 10 blue
.\maxsun-b580-rgb.exe frame `
    FF0000 FF0000 FF0000 FF0000 FF0000 FF0000 FF0000 FF0000 FF0000 FF0000 `
    00FF00 00FF00 00FF00 00FF00 00FF00 00FF00 00FF00 00FF00 00FF00 00FF00 `
    0000FF 0000FF 0000FF 0000FF 0000FF 0000FF 0000FF 0000FF 0000FF 0000FF
```

Static effects need no background updates and are not guaranteed to survive a power cycle.

Normal cleanup removes the MAXSUN driver package while keeping the underlying Intel Xe PnP child device. Interrupted runs or cleanup failures may leave the driver installed.

[Driver responsibilities, version and verification](DRIVER_AUDIT.md)
