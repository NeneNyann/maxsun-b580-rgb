# MAXSUN B580 RGB

[English](README.md) | 简体中文  

自用的 Windows Rust 命令行灯控工具，针对铭瑄 Intel Arc B580 iCraft 12G。

起因是想关掉显卡 RGB，不想运行官方灯控软件，也不希望灯控驱动留在系统里。程序临时加载官方 I2C 驱动，完成操作后卸载驱动包并退出，不常驻。

## 构建

需要 Rust/Cargo（`x86_64-pc-windows-msvc`）、MSVC 和 Windows SDK。可通过 Visual Studio Build Tools 的“使用 C++ 的桌面开发”组件安装后两项。

不提供预编译 Release：程序会内嵌铭瑄官方驱动，为避免再分发第三方驱动，仅提供源码和构建脚本。构建脚本会从官方来源获取驱动并在本地编译。

```powershell
.\build.ps1
```

## 使用

在管理员 PowerShell 中运行：

```powershell
# 检测控制器
.\maxsun-b580-rgb.exe probe

# 关灯
.\maxsun-b580-rgb.exe off

# 全红
.\maxsun-b580-rgb.exe set FF0000

# 静态红蓝渐变
.\maxsun-b580-rgb.exe gradient FF0000 0000FF

# 静态彩虹
.\maxsun-b580-rgb.exe rainbow-static

# 自定义颜色帧：30 个 RRGGBB，下面为 10 红、10 绿、10 蓝
.\maxsun-b580-rgb.exe frame `
    FF0000 FF0000 FF0000 FF0000 FF0000 FF0000 FF0000 FF0000 FF0000 FF0000 `
    00FF00 00FF00 00FF00 00FF00 00FF00 00FF00 00FF00 00FF00 00FF00 00FF00 `
    0000FF 0000FF 0000FF 0000FF 0000FF 0000FF 0000FF 0000FF 0000FF 0000FF
```

静态效果无需后台刷新，不保证断电后保留。

正常结束时移除 MAXSUN 驱动包，保留 Intel Xe 的底层 PnP 子设备。异常中断或清理失败时可能留下驱动。

[驱动职责、版本与检验记录](DRIVER_AUDIT.md)
