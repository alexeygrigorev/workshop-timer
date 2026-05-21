param(
    [string]$InPath  = "scripts/rendered.png",
    [string]$OutPath = "scripts/annotated.png",
    [int]$Scale = 4
)

Add-Type -AssemblyName System.Drawing

$src = [System.Drawing.Image]::FromFile((Resolve-Path $InPath))
$w = $src.Width * $Scale
$h = $src.Height * $Scale

$dst = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($dst)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
$g.DrawImage($src, 0, 0, $w, $h)

# Horizontal mid-line
$pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(180, 255, 80, 80)), 1
$mid = [int]($h / 2)
$g.DrawLine($pen, 0, $mid, $w, $mid)

# Quarter marks (also dashed thirds for finer check)
$lightPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(120, 80, 200, 255)), 1
$lightPen.DashStyle = [System.Drawing.Drawing2D.DashStyle]::Dot
$g.DrawLine($lightPen, 0, [int]($h / 4), $w, [int]($h / 4))
$g.DrawLine($lightPen, 0, [int](3 * $h / 4), $w, [int](3 * $h / 4))

# Vertical guides every 1/12 of width (to roughly section off button columns)
for ($i = 1; $i -lt 12; $i++) {
    $x = [int]($w * $i / 12)
    $g.DrawLine($lightPen, $x, 0, $x, $h)
}

$dst.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $dst.Dispose(); $src.Dispose()
Write-Output "Saved $OutPath ($w by $h)"
