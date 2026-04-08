$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
$iconRoot = Join-Path $repoRoot "assets\icons"
$iconSetRoot = Join-Path $iconRoot "icon.iconset"

New-Item -ItemType Directory -Force -Path $iconSetRoot | Out-Null

$sizes = 16, 32, 64, 128, 256, 512, 1024
$iconSetFiles = @{
    16 = @("icon_16x16.png")
    32 = @("icon_16x16@2x.png", "icon_32x32.png")
    64 = @("icon_32x32@2x.png")
    128 = @("icon_128x128.png")
    256 = @("icon_128x128@2x.png", "icon_256x256.png")
    512 = @("icon_256x256@2x.png", "icon_512x512.png")
    1024 = @("icon_512x512@2x.png")
}

function New-RoundedRectanglePath {
    param(
        [System.Drawing.RectangleF]$Rect,
        [float]$Radius
    )

    $diameter = $Radius * 2
    $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $path.AddArc($Rect.X, $Rect.Y, $diameter, $diameter, 180, 90)
    $path.AddArc($Rect.Right - $diameter, $Rect.Y, $diameter, $diameter, 270, 90)
    $path.AddArc($Rect.Right - $diameter, $Rect.Bottom - $diameter, $diameter, $diameter, 0, 90)
    $path.AddArc($Rect.X, $Rect.Bottom - $diameter, $diameter, $diameter, 90, 90)
    $path.CloseFigure()
    return $path
}

function New-IconBitmap {
    param([int]$Size)

    $bitmap = [System.Drawing.Bitmap]::new(
        $Size,
        $Size,
        [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
    )
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.Clear([System.Drawing.Color]::Transparent)

    $outerPadding = $Size * 0.075
    $outerRect = [System.Drawing.RectangleF]::new(
        $outerPadding,
        $outerPadding,
        $Size - (2 * $outerPadding),
        $Size - (2 * $outerPadding)
    )
    $outerPath = New-RoundedRectanglePath -Rect $outerRect -Radius ($Size * 0.21)

    $fillBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        [System.Drawing.PointF]::new($outerRect.Left, $outerRect.Top),
        [System.Drawing.PointF]::new($outerRect.Right, $outerRect.Bottom),
        [System.Drawing.ColorTranslator]::FromHtml("#6C9A4F"),
        [System.Drawing.ColorTranslator]::FromHtml("#213626")
    )
    $graphics.FillPath($fillBrush, $outerPath)

    $shadowPen = [System.Drawing.Pen]::new(
        [System.Drawing.Color]::FromArgb(70, 7, 16, 9),
        [float]($Size * 0.035)
    )
    $shadowPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    $graphics.DrawPath($shadowPen, $outerPath)

    $innerInset = $Size * 0.03
    $innerRect = [System.Drawing.RectangleF]::new(
        $outerRect.X + $innerInset,
        $outerRect.Y + $innerInset,
        $outerRect.Width - (2 * $innerInset),
        $outerRect.Height - (2 * $innerInset)
    )
    $innerPath = New-RoundedRectanglePath -Rect $innerRect -Radius ($Size * 0.18)
    $innerPen = [System.Drawing.Pen]::new(
        [System.Drawing.Color]::FromArgb(90, 224, 246, 196),
        [float]($Size * 0.016)
    )
    $innerPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    $graphics.DrawPath($innerPen, $innerPath)

    $glyphPen = [System.Drawing.Pen]::new(
        [System.Drawing.ColorTranslator]::FromHtml("#E9F7DA"),
        [float]($Size * 0.13)
    )
    $glyphPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $glyphPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $glyphPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round

    $glyphShadowPen = [System.Drawing.Pen]::new(
        [System.Drawing.Color]::FromArgb(38, 10, 20, 11),
        [float]($Size * 0.13)
    )
    $glyphShadowPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $glyphShadowPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $glyphShadowPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round

    $glyphRect = [System.Drawing.RectangleF]::new(
        $Size * 0.255,
        $Size * 0.225,
        $Size * 0.49,
        $Size * 0.49
    )
    $glyphPath = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $glyphPath.AddArc($glyphRect, 35, 292)

    $barStart = [System.Drawing.PointF]::new($Size * 0.50, $Size * 0.50)
    $barEnd = [System.Drawing.PointF]::new($Size * 0.68, $Size * 0.50)
    $stemStart = $barEnd
    $stemEnd = [System.Drawing.PointF]::new($Size * 0.68, $Size * 0.60)

    $shadowOffset = $Size * 0.018
    $graphics.TranslateTransform($shadowOffset, $shadowOffset)
    $graphics.DrawPath($glyphShadowPen, $glyphPath)
    $graphics.DrawLine($glyphShadowPen, $barStart, $barEnd)
    $graphics.DrawLine($glyphShadowPen, $stemStart, $stemEnd)
    $graphics.ResetTransform()

    $graphics.DrawPath($glyphPen, $glyphPath)
    $graphics.DrawLine($glyphPen, $barStart, $barEnd)
    $graphics.DrawLine($glyphPen, $stemStart, $stemEnd)

    $glyphPath.Dispose()
    $glyphShadowPen.Dispose()
    $glyphPen.Dispose()
    $innerPen.Dispose()
    $innerPath.Dispose()
    $shadowPen.Dispose()
    $fillBrush.Dispose()
    $outerPath.Dispose()
    $graphics.Dispose()

    return $bitmap
}

function Write-RgbaFile {
    param(
        [System.Drawing.Bitmap]$Bitmap,
        [string]$OutputPath
    )

    $bytes = New-Object byte[] ($Bitmap.Width * $Bitmap.Height * 4)
    $index = 0

    for ($y = 0; $y -lt $Bitmap.Height; $y++) {
        for ($x = 0; $x -lt $Bitmap.Width; $x++) {
            $pixel = $Bitmap.GetPixel($x, $y)
            $bytes[$index++] = $pixel.R
            $bytes[$index++] = $pixel.G
            $bytes[$index++] = $pixel.B
            $bytes[$index++] = $pixel.A
        }
    }

    [System.IO.File]::WriteAllBytes($OutputPath, $bytes)
}

function Write-IcoFile {
    param(
        [string]$OutputPath,
        [object[]]$Entries
    )

    $stream = [System.IO.MemoryStream]::new()
    $writer = [System.IO.BinaryWriter]::new($stream)

    try {
        $writer.Write([uint16]0)
        $writer.Write([uint16]1)
        $writer.Write([uint16]$Entries.Count)

        $offset = 6 + (16 * $Entries.Count)
        foreach ($entry in $Entries) {
            $sizeByte = if ($entry.Size -ge 256) { [byte]0 } else { [byte]$entry.Size }
            $writer.Write($sizeByte)
            $writer.Write($sizeByte)
            $writer.Write([byte]0)
            $writer.Write([byte]0)
            $writer.Write([uint16]1)
            $writer.Write([uint16]32)
            $writer.Write([uint32]$entry.Data.Length)
            $writer.Write([uint32]$offset)
            $offset += $entry.Data.Length
        }

        foreach ($entry in $Entries) {
            $writer.Write($entry.Data)
        }

        [System.IO.File]::WriteAllBytes($OutputPath, $stream.ToArray())
    }
    finally {
        $writer.Dispose()
        $stream.Dispose()
    }
}

$icoEntries = @()
$rgbaSource = $null

foreach ($size in $sizes) {
    $bitmap = New-IconBitmap -Size $size
    try {
        foreach ($fileName in $iconSetFiles[$size]) {
            $pngPath = Join-Path $iconSetRoot $fileName
            $bitmap.Save($pngPath, [System.Drawing.Imaging.ImageFormat]::Png)
        }

        $pngSourcePath = Join-Path $iconSetRoot $iconSetFiles[$size][0]
        $icoEntries += [pscustomobject]@{
            Size = $size
            Data = [System.IO.File]::ReadAllBytes($pngSourcePath)
        }

        if ($size -eq 64) {
            $rgbaSource = $bitmap.Clone()
        }
    }
    finally {
        $bitmap.Dispose()
    }
}

if ($null -eq $rgbaSource) {
    throw "Expected a 64px icon to generate the runtime RGBA asset."
}

try {
    Write-RgbaFile -Bitmap $rgbaSource -OutputPath (Join-Path $iconRoot "window-icon-64.rgba")
}
finally {
    $rgbaSource.Dispose()
}

Write-IcoFile -OutputPath (Join-Path $iconRoot "gabalah.ico") -Entries $icoEntries

Write-Host "Generated icons in $iconRoot"
