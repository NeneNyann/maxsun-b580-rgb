$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $Root

$DriverUrl = 'https://download.maxsun.com.cn:8443/vga/intel/Graphic_Light_Controller/Light_Controller_Driver.zip'
$Expected = @{
    Zip   = '253E7CB9695BAA5D1CD500A4CBD6BF4AE2728830138432A2B29671A1239E09D9'
    Setup = '386FC59F82FE1F921816227976715C6E4E1CA68AC038F14786D2140ED0F8373F'
    Sys   = 'B3E9B76DA94FA1B6D3DBD19593238D0C841F191E628776B41A8754019CCBC6AC'
    Inf   = '9DEB9A88F15031025A947413D4954EFB6938B6DB71D26ECFBE75B6CB3DF12CD7'
    Cat   = '440EF8D11BFC4B03096F029BD37527A5955F93BBE4D6970B6A42D9AC29AB3995'
}

function Assert-Sha256([string]$Path, [string]$ExpectedHash) {
    $actual = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToUpperInvariant()
    if ($actual -ne $ExpectedHash.ToUpperInvariant()) {
        throw "SHA-256 mismatch for $Path`nexpected: $ExpectedHash`nactual:   $actual"
    }
    Write-Host "SHA-256 OK: $([IO.Path]::GetFileName($Path))"
}

$Work = Join-Path $env:TEMP ("maxsun-b580-driver-{0}" -f [guid]::NewGuid().ToString('N'))
$Vendor = Join-Path $Work 'vendor'
$Tools = Join-Path $Work 'tools'
$Out = Join-Path $Work 'out'
$Zip = Join-Path $Work 'Light_Controller_Driver.zip'
$DriverDir = Join-Path $Root 'driver'

try {
    New-Item -ItemType Directory -Force -Path $Vendor, $Tools, $Out, $DriverDir | Out-Null

    Write-Host '[1/6] Downloading MAXSUN driver package...'
    Invoke-WebRequest -UseBasicParsing -Uri $DriverUrl -OutFile $Zip
    Assert-Sha256 $Zip $Expected.Zip

    Write-Host '[2/6] Extracting MAXSUN package...'
    Expand-Archive -LiteralPath $Zip -DestinationPath $Vendor -Force

    $Setup = Get-ChildItem -LiteralPath $Vendor -Recurse -File -Filter 'MaxSun Xe Graphic Light Controller Setup.exe' | Select-Object -First 1
    if (-not $Setup) { throw 'MAXSUN setup executable was not found.' }
    Assert-Sha256 $Setup.FullName $Expected.Setup

    Write-Host '[3/6] Using pinned innoextract_win release 670...'
    $InnoUrl = 'https://github.com/UserUnknownFactor/innoextract_win/releases/download/670/innoextract670.zip'
    $InnoSha256 = '79B69B9B1FCD98F42CCD4B245EFDF6A03BCFB674BA6AF482F5A46891C9ED4D14'

    Write-Host '[4/6] Downloading innoextract670.zip...'
    $InnoZip = Join-Path $Tools 'innoextract670.zip'
    Invoke-WebRequest -UseBasicParsing -Uri $InnoUrl -OutFile $InnoZip
    Assert-Sha256 $InnoZip $InnoSha256
    $InnoDir = Join-Path $Tools 'innoextract'
    Expand-Archive -LiteralPath $InnoZip -DestinationPath $InnoDir -Force
    $Inno = Get-ChildItem -LiteralPath $InnoDir -Recurse -File -Filter 'innoextract.exe' | Select-Object -First 1
    if (-not $Inno) { throw 'innoextract.exe was not found in the downloaded release.' }

    Write-Host '[5/6] Extracting MAXSUN installer without running it...'
    & $Inno.FullName -d $Out $Setup.FullName
    if ($LASTEXITCODE -ne 0) { throw "innoextract exited with code $LASTEXITCODE" }

    $Sys = Get-ChildItem -LiteralPath $Out -Recurse -File -Filter 'IntelXeGraphicI2cDeviceDriver.sys' | Select-Object -First 1
    $Inf = Get-ChildItem -LiteralPath $Out -Recurse -File -Filter 'IntelXeGraphicI2cDeviceDriver.inf' | Select-Object -First 1
    $Cat = Get-ChildItem -LiteralPath $Out -Recurse -File -Filter 'IntelXeGraphicI2cDeviceDriver.cat' | Select-Object -First 1
    if (-not $Sys -or -not $Inf -or -not $Cat) { throw 'Driver SYS/INF/CAT was not found after extraction.' }

    Assert-Sha256 $Sys.FullName $Expected.Sys
    Assert-Sha256 $Inf.FullName $Expected.Inf
    Assert-Sha256 $Cat.FullName $Expected.Cat

    Write-Host '[6/6] Copying verified driver package...'
    Copy-Item -Force $Sys.FullName (Join-Path $DriverDir 'IntelXeGraphicI2cDeviceDriver.sys')
    Copy-Item -Force $Inf.FullName (Join-Path $DriverDir 'IntelXeGraphicI2cDeviceDriver.inf')
    Copy-Item -Force $Cat.FullName (Join-Path $DriverDir 'IntelXeGraphicI2cDeviceDriver.cat')

    @(
        "$($Expected.Sys)  IntelXeGraphicI2cDeviceDriver.sys"
        "$($Expected.Inf)  IntelXeGraphicI2cDeviceDriver.inf"
        "$($Expected.Cat)  IntelXeGraphicI2cDeviceDriver.cat"
    ) | Set-Content -LiteralPath (Join-Path $DriverDir 'SHA256SUMS.txt') -Encoding ascii

    Write-Host 'Driver package ready.'
}
finally {
    if (Test-Path -LiteralPath $Work) {
        Remove-Item -LiteralPath $Work -Recurse -Force -ErrorAction SilentlyContinue
    }
}
