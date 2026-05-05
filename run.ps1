param(
    [switch]$NoRun
)

$ErrorActionPreference = "Stop"
$repoRoot = $PSScriptRoot

function Invoke-ExternalCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [string[]]$ArgumentList = @()
    )

    & $FilePath @ArgumentList
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code ${LASTEXITCODE}: $FilePath $($ArgumentList -join ' ')"
    }
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Missing cargo. Install the Rust toolchain before building."
}

Push-Location $repoRoot
try {
    Invoke-ExternalCommand -FilePath "cargo" -ArgumentList @("build", "-p", "universal-stickers")
    if (-not $NoRun) {
        Invoke-ExternalCommand -FilePath (Join-Path $repoRoot "target\debug\universal-stickers.exe")
    }
} finally {
    Pop-Location
}
