#![cfg_attr(not(windows), allow(dead_code, unused_imports))]

#[cfg(not(windows))]
compile_error!("maxsun-b580-rgb only supports Windows x64.");

#[cfg(windows)]
mod device;
#[cfg(windows)]
mod driver;
#[cfg(windows)]
mod protocol;

#[cfg(windows)]
mod app {
    use std::env;

    use crate::{
        device::Device,
        driver::DriverSession,
        protocol::{
            gradient, parse_frame, parse_rgb, probe_is_valid, solid, static_rainbow,
        },
    };

    fn usage() {
        eprintln!(
            "MAXSUN Intel Arc B580 iCraft RGB\n\n\
             Usage:\n  \
             maxsun-b580-rgb probe\n  \
             maxsun-b580-rgb set <RRGGBB>\n  \
             maxsun-b580-rgb off\n  \
             maxsun-b580-rgb gradient <RRGGBB> <RRGGBB>\n  \
             maxsun-b580-rgb rainbow-static\n  \
             maxsun-b580-rgb frame <30 x RRGGBB>\n\n\
             All RGB commands write one persistent framebuffer and exit.\n\
             No command keeps refreshing LEDs in the background."
        );
    }

    fn verified_probe(device: &Device) -> Result<[u8; 16], String> {
        let response = device.probe().map_err(|e| format!("probe: {e}"))?;
        if !probe_is_valid(&response) {
            return Err("unexpected MCU signature; write refused".into());
        }
        Ok(response)
    }

    fn write(device: &Device, frame: &[u8]) -> Result<(), String> {
        device
            .write_frame(frame)
            .map_err(|e| format!("RGB write: {e}"))
    }

    fn execute(device: &Device, args: &[String]) -> Result<(), String> {
        let response = verified_probe(device)?;

        match args.get(1).map(String::as_str) {
            Some("probe") => {
                print!("MCU:");
                for byte in response {
                    print!(" {byte:02X}");
                }
                println!("\nMAXSUN B580 RGB MCU detected");
                Ok(())
            }
            Some("set") => {
                if args.len() != 3 {
                    return Err("usage: set <RRGGBB>".into());
                }
                let color = parse_rgb(&args[2])?;
                write(device, &solid(color))?;
                println!("set #{:02X}{:02X}{:02X}", color.0, color.1, color.2);
                Ok(())
            }
            Some("off") => {
                if args.len() != 2 {
                    return Err("usage: off".into());
                }
                write(device, &solid((0, 0, 0)))?;
                println!("off");
                Ok(())
            }
            Some("gradient") => {
                if args.len() != 4 {
                    return Err("usage: gradient <RRGGBB> <RRGGBB>".into());
                }
                let start = parse_rgb(&args[2])?;
                let end = parse_rgb(&args[3])?;
                write(device, &gradient(start, end))?;
                println!("gradient #{} -> #{}", args[2], args[3]);
                Ok(())
            }
            Some("rainbow-static") => {
                if args.len() != 2 {
                    return Err("usage: rainbow-static".into());
                }
                write(device, &static_rainbow())?;
                println!("static rainbow");
                Ok(())
            }
            Some("frame") => {
                if args.len() != 2 + crate::protocol::LED_COUNT {
                    return Err(format!(
                        "usage: frame <{} x RRGGBB>",
                        crate::protocol::LED_COUNT
                    ));
                }
                let frame = parse_frame(&args[2..])?;
                write(device, &frame)?;
                println!("custom 30-LED frame set");
                Ok(())
            }
            _ => {
                usage();
                Err("invalid command".into())
            }
        }
    }

    pub fn run() -> i32 {
        let args: Vec<String> = env::args().collect();
        if args.len() < 2 {
            usage();
            return 2;
        }

        let (mut driver, device) = match DriverSession::acquire() {
            Ok(value) => value,
            Err(error) => {
                eprintln!("driver: {error}\nRun from an elevated Administrator terminal.");
                return 1;
            }
        };

        let result = execute(&device, &args);

        // The device handle must be closed before removing the PnP devnode and
        // deleting the driver package from Driver Store.
        drop(device);

        let cleanup = driver.cleanup();

        if let Err(error) = cleanup {
            eprintln!("driver cleanup: {error}");
            return 4;
        }

        match result {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("error: {error}");
                1
            }
        }
    }
}

#[cfg(windows)]
fn main() {
    std::process::exit(app::run());
}
