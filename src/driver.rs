use std::{
    env, fs, io,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    thread,
    time::{Duration, Instant},
};

use windows_sys::Win32::{
    Devices::DeviceAndDriverInstallation::{
        DiUninstallDriverW, SetupCopyOEMInfW, UpdateDriverForPlugAndPlayDevicesW,
        INSTALLFLAG_FORCE, INSTALLFLAG_NONINTERACTIVE, SP_COPY_NOOVERWRITE, SPOST_PATH,
    },
    Foundation::{GetLastError, ERROR_FILE_EXISTS, ERROR_FILE_NOT_FOUND},
};

use crate::device::{Device, DEVICE_PATH};

const INF_NAME: &str = "IntelXeGraphicI2cDeviceDriver.inf";
const CAT_NAME: &str = "IntelXeGraphicI2cDeviceDriver.cat";
const SYS_NAME: &str = "IntelXeGraphicI2cDeviceDriver.sys";
const ERROR_NO_SUCH_DEVINST: u32 = 0xE000_020B;
const HW_IDS: &[&str] = &[
    r"{5CDD8C40-2EC8-4363-9629-C43D92197B68}\NF_I2C_Compat",
    r"{5CDD8C40-2EC8-4363-9629-C43D92197B68}\ADR_0X0010",
    r"{5CDD8C40-2EC8-4363-9629-C43D92197B68}\ADR_0X0011",
    r"{5CDD8C40-2EC8-4363-9629-C43D92197B68}\ADR_0X0012",
    r"{5CDD8C40-2EC8-4363-9629-C43D92197B68}\ADR_0X0013",
];

static INF_BYTES: &[u8] = include_bytes!("../driver/IntelXeGraphicI2cDeviceDriver.inf");
static CAT_BYTES: &[u8] = include_bytes!("../driver/IntelXeGraphicI2cDeviceDriver.cat");
static SYS_BYTES: &[u8] = include_bytes!("../driver/IntelXeGraphicI2cDeviceDriver.sys");

fn wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    s.as_ref().encode_wide().chain([0]).collect()
}

fn last_error() -> io::Error {
    io::Error::from_raw_os_error(unsafe { GetLastError() } as i32)
}

struct TempDriver {
    dir: PathBuf,
    inf: PathBuf,
}

impl TempDriver {
    fn create() -> io::Result<Self> {
        let dir = env::temp_dir().join(format!("maxsun-b580-rgb-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join(INF_NAME), INF_BYTES)?;
        fs::write(dir.join(CAT_NAME), CAT_BYTES)?;
        fs::write(dir.join(SYS_NAME), SYS_BYTES)?;
        Ok(Self {
            inf: dir.join(INF_NAME),
            dir,
        })
    }
}

impl Drop for TempDriver {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

pub struct DriverSession {
    _temp: TempDriver,
    published_inf: PathBuf,
    cleaned: bool,
}

impl DriverSession {
    pub fn acquire() -> Result<(Self, Device), String> {
        // Always own the complete lifecycle, including a package left behind by
        // a previous interrupted run. No "already present -> don't clean" path.
        let temp = TempDriver::create().map_err(|e| format!("extract embedded driver: {e}"))?;
        let published_inf = stage(&temp.inf)?;
        bind(&temp.inf)?;

        let device = wait_for_device_present(Duration::from_secs(8))
            .ok_or_else(|| format!("driver bound, but {DEVICE_PATH} did not appear"))?;

        Ok((
            Self {
                _temp: temp,
                published_inf,
                cleaned: false,
            },
            device,
        ))
    }

    pub fn cleanup(&mut self) -> Result<(), String> {
        if self.cleaned {
            return Ok(());
        }

        // Do NOT remove the Intel Xe/Resource Hub child devnode. It is a real
        // PnP child and Windows will simply re-enumerate it. Removing it also
        // creates a race where the next run cannot find a device to bind.
        //
        // DiUninstallDriverW is exactly what we want here: it detaches this
        // package from every device using it, installs another matching driver
        // (or the null driver), and then removes this package from Driver Store.
        uninstall_driver_package(&self.published_inf)?;

        // The MAXSUN-created user-visible DOS device must disappear once the
        // driver is detached. The underlying Intel child devnode may remain,
        // which is expected and required for the next run to bind immediately.
        if !wait_for_device_absent(Duration::from_secs(8)) {
            return Err(format!(
                "cleanup incomplete: {DEVICE_PATH} is still openable after uninstalling {}",
                self.published_inf.display()
            ));
        }

        self.cleaned = true;
        Ok(())
    }
}

impl Drop for DriverSession {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn stage(source_inf: &Path) -> Result<PathBuf, String> {
    let source = wide(source_inf);
    let mut destination = vec![0u16; 32768];
    let mut required = 0u32;
    let mut component = null_mut();

    let ok = unsafe {
        SetupCopyOEMInfW(
            source.as_ptr(),
            null(),
            SPOST_PATH,
            SP_COPY_NOOVERWRITE,
            destination.as_mut_ptr(),
            destination.len() as u32,
            &mut required,
            &mut component,
        )
    };

    let error = unsafe { GetLastError() };
    if ok == 0 && error != ERROR_FILE_EXISTS {
        return Err(format!(
            "SetupCopyOEMInfW: {}",
            io::Error::from_raw_os_error(error as i32)
        ));
    }

    let len = destination
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(destination.len());
    if len == 0 {
        return Err("SetupCopyOEMInfW did not return the published OEM INF path".into());
    }

    Ok(PathBuf::from(String::from_utf16_lossy(&destination[..len])))
}

fn bind(source_inf: &Path) -> Result<(), String> {
    let inf = wide(source_inf);
    let mut matches = 0usize;
    let mut reboot = 0;
    let mut last_error_seen = None;

    for hardware_id in HW_IDS {
        let hardware_id = wide(hardware_id);
        let ok = unsafe {
            UpdateDriverForPlugAndPlayDevicesW(
                null_mut(),
                hardware_id.as_ptr(),
                inf.as_ptr(),
                INSTALLFLAG_FORCE | INSTALLFLAG_NONINTERACTIVE,
                &mut reboot,
            )
        };

        if ok != 0 {
            matches += 1;
        } else {
            let code = unsafe { GetLastError() };
            if code != ERROR_NO_SUCH_DEVINST {
                last_error_seen = Some(io::Error::from_raw_os_error(code as i32));
            }
        }
    }

    if matches == 0 {
        return Err(last_error_seen
            .map(|e| format!("driver binding failed: {e}"))
            .unwrap_or_else(|| "no matching Intel Xe I2C child device found".into()));
    }
    if reboot != 0 {
        return Err("Windows requested a reboot while binding the driver".into());
    }
    Ok(())
}

fn uninstall_driver_package(published_inf: &Path) -> Result<(), String> {
    let inf = wide(published_inf);
    let mut reboot = 0;
    let ok = unsafe { DiUninstallDriverW(null_mut(), inf.as_ptr(), 0, &mut reboot) };
    if ok == 0 {
        let code = unsafe { GetLastError() };
        if code != ERROR_FILE_NOT_FOUND {
            return Err(format!(
                "DiUninstallDriverW({}): {}",
                published_inf.display(),
                io::Error::from_raw_os_error(code as i32)
            ));
        }
    }
    if reboot != 0 {
        return Err("Windows requested a reboot to finish removing the driver package".into());
    }
    Ok(())
}

fn wait_for_device_present(timeout: Duration) -> Option<Device> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(device) = Device::open() {
            return Some(device);
        }
        if Instant::now() >= deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn wait_for_device_absent(timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        match Device::open() {
            Ok(device) => drop(device),
            Err(_) => return true,
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(100));
    }
}
