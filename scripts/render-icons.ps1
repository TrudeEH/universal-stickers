param(
    [Parameter(Mandatory = $true)]
    [string]$SvgPath,

    [Parameter(Mandatory = $true)]
    [string]$OutputDir,

    [string]$InkscapePath = "inkscape"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $SvgPath)) {
    throw "Missing SVG source: $SvgPath"
}

if (-not (Get-Command $InkscapePath -ErrorAction SilentlyContinue)) {
    throw "Inkscape was not found. Pass -InkscapePath or add it to PATH."
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$masterPng = Join-Path $OutputDir "universal-stickers.png"
& $InkscapePath $SvgPath --export-type=png --export-filename=$masterPng --export-width=1024 --export-height=1024 | Out-Null

Add-Type -AssemblyName System.Drawing

$bitmap = [System.Drawing.Bitmap]::FromFile($masterPng)

function New-ResizedBitmap([System.Drawing.Bitmap]$inputBitmap, [int]$size) {
    $resized = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($resized)
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.Clear([System.Drawing.Color]::Transparent)
    $graphics.DrawImage($inputBitmap, 0, 0, $size, $size)
    $graphics.Dispose()
    return $resized
}

foreach ($size in 512, 256, 128) {
    $resized = New-ResizedBitmap $bitmap $size
    $resized.Save((Join-Path $OutputDir ("universal-stickers-$size.png")), [System.Drawing.Imaging.ImageFormat]::Png)
    $resized.Dispose()
}

$frames = @()
foreach ($size in 256, 128, 64, 48, 32, 16) {
    $resized = New-ResizedBitmap $bitmap $size
    $stream = New-Object System.IO.MemoryStream
    $resized.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
    $frames += [pscustomobject]@{
        Size = $size
        Bytes = $stream.ToArray()
    }
    $stream.Dispose()
    $resized.Dispose()
}

$iconPath = Join-Path $OutputDir "universal-stickers.ico"
$fileStream = [System.IO.File]::Open($iconPath, [System.IO.FileMode]::Create)
$writer = New-Object System.IO.BinaryWriter($fileStream)
$writer.Write([UInt16]0)
$writer.Write([UInt16]1)
$writer.Write([UInt16]$frames.Count)
$offset = 6 + (16 * $frames.Count)

foreach ($frame in $frames) {
    $dimension = if ($frame.Size -ge 256) { 0 } else { [byte]$frame.Size }
    $writer.Write([byte]$dimension)
    $writer.Write([byte]$dimension)
    $writer.Write([byte]0)
    $writer.Write([byte]0)
    $writer.Write([UInt16]1)
    $writer.Write([UInt16]32)
    $writer.Write([UInt32]$frame.Bytes.Length)
    $writer.Write([UInt32]$offset)
    $offset += $frame.Bytes.Length
}

foreach ($frame in $frames) {
    $writer.Write($frame.Bytes)
}

$writer.Dispose()
$fileStream.Dispose()
$bitmap.Dispose()
