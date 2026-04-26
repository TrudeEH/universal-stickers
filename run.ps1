param(
    [switch]$NoRun
)

$ErrorActionPreference = "Stop"

$repoRoot = $PSScriptRoot
$qtRoot = Join-Path $repoRoot ".tools\qt"
$qtVersion = "6.8.3"
$qtArch = "mingw_64"

$cmake = Join-Path $qtRoot "Tools\CMake_64\bin\cmake.exe"
$mingwBin = Join-Path $qtRoot "Tools\mingw1310_64\bin"
$qtBin = Join-Path $qtRoot "$qtVersion\$qtArch\bin"
$prefixPath = Join-Path $qtRoot "$qtVersion\$qtArch"
$buildDir = Join-Path $repoRoot "desktop\build"
$exePath = Join-Path $buildDir "universal-stickers.exe"
$windeployqt = Join-Path $qtBin "windeployqt.exe"

foreach ($path in @($cmake, $mingwBin, $qtBin)) {
    if (-not (Test-Path $path)) {
        throw "Missing required local toolchain path: $path"
    }
}

$env:Path = "$($qtBin);$($mingwBin);$env:Path"

& $cmake -S (Join-Path $repoRoot "desktop") -B $buildDir -G Ninja "-DCMAKE_PREFIX_PATH=$prefixPath"
& $cmake --build $buildDir

if (Test-Path $windeployqt) {
    & $windeployqt --release --no-translations $exePath | Out-Host
}

if (-not $NoRun) {
    & $exePath
}
