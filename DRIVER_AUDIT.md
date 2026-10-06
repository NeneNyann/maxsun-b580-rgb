# Driver verification notes

## Responsibilities

`IntelXeGraphicI2cDeviceDriver.sys` is the official KMDF / Resource Hub / SPB I2C bridge driver. It lets the CLI access the GPU's RGB MCU at I2C address `0x43` through `\\.\NF_I2C_BUS_00_0X0043`. The CLI generates the color frames; MAXSUN Sync and Aura Service are not required.

After each operation, the CLI removes the corresponding Driver Store package and keeps the Intel Xe PnP child device. An interrupted run may prevent cleanup.

## Version and source

- INF version: `9.33.21.740`, driver date: `2024-11-29`.
- Source: [MAXSUN's official Light Controller driver package](https://download.maxsun.com.cn:8443/vga/intel/Graphic_Light_Controller/Light_Controller_Driver.zip).
- The SYS, INF and CAT SHA-256 hashes match the values pinned in the download script.

| File | SHA-256 |
| --- | --- |
| `Light_Controller_Driver.zip` | `253E7CB9695BAA5D1CD500A4CBD6BF4AE2728830138432A2B29671A1239E09D9` |
| `MaxSun Xe Graphic Light Controller Setup.exe` | `386FC59F82FE1F921816227976715C6E4E1CA68AC038F14786D2140ED0F8373F` |
| `IntelXeGraphicI2cDeviceDriver.sys` | `B3E9B76DA94FA1B6D3DBD19593238D0C841F191E628776B41A8754019CCBC6AC` |
| `IntelXeGraphicI2cDeviceDriver.inf` | `9DEB9A88F15031025A947413D4954EFB6938B6DB71D26ECFBE75B6CB3DF12CD7` |
| `IntelXeGraphicI2cDeviceDriver.cat` | `440EF8D11BFC4B03096F029BD37527A5955F93BBE4D6970B6A42D9AC29AB3995` |

The ZIP and installer hashes are the fixed verification values in the download script.

## Security checks

- **Hashes**: The download script verifies each file against the pinned version.
- **Signatures**: Local `Get-AuthenticodeSignature` checks return `Valid` for CAT and SYS. The signer is `Microsoft Windows Hardware Compatibility Publisher`, issued by `Microsoft Windows Third Party Component CA 2014`.
- **Static review**: Existing review notes report no network functionality, process/thread/image callbacks, arbitrary physical-memory mapping imports or obvious generic kernel-memory read/write interface.
- **CLI restrictions**: A fixed device path, an `ISK...AP` check before writing, and only 95-byte frames containing 30 color positions.
