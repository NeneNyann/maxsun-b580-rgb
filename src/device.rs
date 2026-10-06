use std::{io, mem::zeroed, ptr::null_mut, thread, time::Duration};

use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_IO_PENDING, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{
        CreateFileW, ReadFile, WriteFile, FILE_FLAG_OVERLAPPED, OPEN_EXISTING,
    },
    System::{
        IO::{DeviceIoControl, GetOverlappedResult, OVERLAPPED},
        Threading::CreateEventW,
    },
};

use crate::protocol::PROBE_SIZE;

pub const DEVICE_PATH: &str = r"\\.\NF_I2C_BUS_00_0X0043";
const IOCTL_OPEN_PERIPHERAL: u32 = 0x0400_1C00;
const IOCTL_CLOSE_PERIPHERAL: u32 = 0x0400_1C04;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

fn last_error() -> io::Error {
    io::Error::from_raw_os_error(unsafe { GetLastError() } as i32)
}

pub struct Device(HANDLE);

impl Device {
    pub fn open() -> io::Result<Self> {
        let path = wide(DEVICE_PATH);
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            Err(last_error())
        } else {
            Ok(Self(handle))
        }
    }

    pub fn probe(&self) -> io::Result<[u8; PROBE_SIZE]> {
        let mut session = Peripheral::open(self)?;
        session.write(&[0x00])?;
        thread::sleep(Duration::from_millis(100));
        let mut response = [0u8; PROBE_SIZE];
        session.read(&mut response)?;
        session.close()?;
        Ok(response)
    }

    pub fn write_frame(&self, frame: &[u8]) -> io::Result<()> {
        let mut session = Peripheral::open(self)?;
        session.write(frame)?;
        session.close()
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

struct Peripheral<'a> {
    device: &'a Device,
    event: HANDLE,
    overlapped: OVERLAPPED,
    closed: bool,
}

impl<'a> Peripheral<'a> {
    fn open(device: &'a Device) -> io::Result<Self> {
        let event = unsafe { CreateEventW(null_mut(), 1, 0, null_mut()) };
        if event.is_null() {
            return Err(last_error());
        }

        let mut overlapped: OVERLAPPED = unsafe { zeroed() };
        overlapped.hEvent = event;
        let mut session = Self { device, event, overlapped, closed: false };
        if let Err(error) = session.ioctl(IOCTL_OPEN_PERIPHERAL) {
            unsafe { CloseHandle(event) };
            session.closed = true;
            return Err(error);
        }
        Ok(session)
    }

    fn await_io(&mut self, started: i32, immediate_error: u32) -> io::Result<u32> {
        if started == 0 && immediate_error != ERROR_IO_PENDING {
            return Err(io::Error::from_raw_os_error(immediate_error as i32));
        }
        let mut transferred = 0u32;
        let ok = unsafe {
            GetOverlappedResult(self.device.0, &mut self.overlapped, &mut transferred, 1)
        };
        if ok == 0 { Err(last_error()) } else { Ok(transferred) }
    }

    fn ioctl(&mut self, code: u32) -> io::Result<()> {
        let mut immediate = 0u32;
        let started = unsafe {
            DeviceIoControl(
                self.device.0,
                code,
                null_mut(),
                0,
                null_mut(),
                0,
                &mut immediate,
                &mut self.overlapped,
            )
        };
        let error = if started == 0 { unsafe { GetLastError() } } else { 0 };
        self.await_io(started, error).map(|_| ())
    }

    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        let mut immediate = 0u32;
        let started = unsafe {
            WriteFile(
                self.device.0,
                data.as_ptr().cast(),
                data.len() as u32,
                &mut immediate,
                &mut self.overlapped,
            )
        };
        let error = if started == 0 { unsafe { GetLastError() } } else { 0 };

        // Match MAXSUN's AacHal_x64.dll transport timing exactly: the HAL
        // waits 50 ms after issuing WriteFile before waiting for completion.
        // This gives the Xe SPB/I2C bridge and RGB MCU time to consume a
        // complete frame before ClosePeripheralDevice is issued.
        thread::sleep(Duration::from_millis(50));

        let written = self.await_io(started, error)?;
        if written == data.len() as u32 {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::WriteZero, format!("short write: {written}/{}", data.len())))
        }
    }

    fn read(&mut self, data: &mut [u8]) -> io::Result<()> {
        let mut immediate = 0u32;
        let started = unsafe {
            ReadFile(
                self.device.0,
                data.as_mut_ptr().cast(),
                data.len() as u32,
                &mut immediate,
                &mut self.overlapped,
            )
        };
        let error = if started == 0 { unsafe { GetLastError() } } else { 0 };
        let read = self.await_io(started, error)?;
        if read == data.len() as u32 {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::UnexpectedEof, format!("short read: {read}/{}", data.len())))
        }
    }

    fn close(&mut self) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }
        let result = self.ioctl(IOCTL_CLOSE_PERIPHERAL);
        unsafe { CloseHandle(self.event) };
        self.closed = true;
        result
    }
}

impl Drop for Peripheral<'_> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
