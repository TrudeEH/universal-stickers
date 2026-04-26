param(
    [switch]$NoRun
)

$ErrorActionPreference = "Stop"

$repoRoot = $PSScriptRoot
$toolsRoot = Join-Path $repoRoot ".tools"
$qtRoot = Join-Path $toolsRoot "qt"
$qtVersion = "6.8.3"
$aqtQtArch = "win64_mingw"
$qtArch = "mingw_64"
$rustWindowsTarget = "x86_64-pc-windows-gnu"

$cmake = Join-Path $qtRoot "Tools\CMake_64\bin\cmake.exe"
$mingwBin = Join-Path $qtRoot "Tools\mingw1310_64\bin"
$qtBin = Join-Path $qtRoot "$qtVersion\$qtArch\bin"
$prefixPath = Join-Path $qtRoot "$qtVersion\$qtArch"
$buildDir = Join-Path $repoRoot "desktop\build"
$exePath = Join-Path $buildDir "universal-stickers.exe"
$windeployqt = Join-Path $qtBin "windeployqt.exe"

function Invoke-ExternalCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [string[]]$ArgumentList = @()
    )

    & $FilePath @ArgumentList
    if ($LASTEXITCODE -ne 0) {
        $commandText = ($ArgumentList | ForEach-Object {
            if ($_ -match "\s") {
                '"' + $_ + '"'
            } else {
                $_
            }
        }) -join " "
        throw "Command failed with exit code ${LASTEXITCODE}: $FilePath $commandText"
    }
}

function Test-ToolchainReady {
    foreach ($path in @($cmake, $mingwBin, $qtBin)) {
        if (-not (Test-Path $path)) {
            return $false
        }
    }

    return $true
}

function Resolve-PythonExecutable {
    $candidates = New-Object System.Collections.Generic.List[string]

    if ($env:PYTHON) {
        $candidates.Add($env:PYTHON)
    }

    foreach ($pattern in @(
        (Join-Path $env:LOCALAPPDATA "Python\pythoncore-*\python.exe"),
        (Join-Path $env:LOCALAPPDATA "Programs\Python\Python*\python.exe")
    )) {
        Get-ChildItem -Path $pattern -File -ErrorAction SilentlyContinue |
            Sort-Object FullName -Descending |
            ForEach-Object { $candidates.Add($_.FullName) }
    }

    foreach ($candidate in @(
        (Get-Command python -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -ErrorAction SilentlyContinue),
        (Get-Command python3 -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -ErrorAction SilentlyContinue),
        (Get-Command py -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -ErrorAction SilentlyContinue)
    )) {
        if ($candidate) {
            $candidates.Add($candidate)
        }
    }

    foreach ($candidate in $candidates | Select-Object -Unique) {
        try {
            & $candidate --version *> $null
            if ($LASTEXITCODE -eq 0) {
                return $candidate
            }
        } catch {
        }
    }

    throw "Unable to find a working Python interpreter. Install Python 3 with pip so the repo-local Qt toolchain can be bootstrapped automatically."
}

function Ensure-AqtInstalled {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PythonExe
    )

    $aqtCheckArgs = @(
        "-c",
        "import importlib.util, sys; sys.exit(0 if importlib.util.find_spec('aqt') else 1)"
    )

    & $PythonExe @aqtCheckArgs *> $null
    if ($LASTEXITCODE -eq 0) {
        return
    }

    Write-Host "Installing Python package 'aqtinstall' for local Qt bootstrap..."
    Invoke-ExternalCommand -FilePath $PythonExe -ArgumentList @("-m", "pip", "install", "--user", "aqtinstall")
}

function Install-QtToolchain {
    if (Test-ToolchainReady) {
        return
    }

    New-Item -ItemType Directory -Path $toolsRoot -Force | Out-Null

    $pythonExe = Resolve-PythonExecutable
    Ensure-AqtInstalled -PythonExe $pythonExe

    $aqtArgsPrefix = @("-m", "aqt")

    if (-not (Test-Path $cmake)) {
        Write-Host "Installing repo-local CMake via aqt..."
        Invoke-ExternalCommand -FilePath $pythonExe -ArgumentList ($aqtArgsPrefix + @(
            "install-tool",
            "windows",
            "desktop",
            "tools_cmake",
            "qt.tools.cmake",
            "-O",
            $qtRoot
        ))
    }

    if (-not (Test-Path $mingwBin)) {
        Write-Host "Installing repo-local MinGW via aqt..."
        Invoke-ExternalCommand -FilePath $pythonExe -ArgumentList ($aqtArgsPrefix + @(
            "install-tool",
            "windows",
            "desktop",
            "tools_mingw1310",
            "qt.tools.win64_mingw1310",
            "-O",
            $qtRoot
        ))
    }

    if (-not (Test-Path $qtBin)) {
        Write-Host "Installing repo-local Qt $qtVersion via aqt..."
        Invoke-ExternalCommand -FilePath $pythonExe -ArgumentList ($aqtArgsPrefix + @(
            "install-qt",
            "windows",
            "desktop",
            $qtVersion,
            $aqtQtArch,
            "-O",
            $qtRoot
        ))
    }

    if (-not (Test-ToolchainReady)) {
        throw "Qt bootstrap completed, but the expected repo-local toolchain paths were still not found under $qtRoot"
    }
}

function Ensure-RustWindowsTarget {
    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
        throw "Missing rustup. Install the Rust toolchain before building."
    }

    $installedTargets = & rustup target list --installed
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to query installed Rust targets with rustup."
    }

    if ($installedTargets -notcontains $rustWindowsTarget) {
        Write-Host "Installing Rust target $rustWindowsTarget..."
        Invoke-ExternalCommand -FilePath "rustup" -ArgumentList @("target", "add", $rustWindowsTarget)
    }
}

function Resolve-CMakeGenerator {
    if (Get-Command ninja -ErrorAction SilentlyContinue) {
        return "Ninja"
    }

    if (Test-Path (Join-Path $mingwBin "mingw32-make.exe")) {
        return "MinGW Makefiles"
    }

    throw "Unable to find a supported build generator. Install Ninja or provide mingw32-make.exe in the repo-local MinGW toolchain."
}

Install-QtToolchain
Ensure-RustWindowsTarget

$env:Path = "$($qtBin);$($mingwBin);$env:Path"

$cmakeArgs = @(
    "-S",
    (Join-Path $repoRoot "desktop"),
    "-B",
    $buildDir,
    "-DCMAKE_PREFIX_PATH=$prefixPath"
)

$cacheFile = Join-Path $buildDir "CMakeCache.txt"
if (-not (Test-Path $cacheFile)) {
    $cmakeArgs += @("-G", (Resolve-CMakeGenerator))
}

Invoke-ExternalCommand -FilePath $cmake -ArgumentList $cmakeArgs
Invoke-ExternalCommand -FilePath $cmake -ArgumentList @("--build", $buildDir)

if (Test-Path $windeployqt) {
    Invoke-ExternalCommand -FilePath $windeployqt -ArgumentList @("--release", "--no-translations", $exePath)
}

if (-not $NoRun) {
    Invoke-ExternalCommand -FilePath $exePath
}
