param([string]$InPath = "scripts/rendered.png")

# Reads the rendered bar screenshot and reports the centroid of glyph pixels
# inside each of the five right-side buttons (last 5 * 26 logical px = ~last
# 312 actual px at 2x DPI). For each button, reports horizontal and vertical
# offset of the glyph centroid from the button center, in image pixels.

Add-Type -AssemblyName System.Drawing

$src = [System.Drawing.Bitmap]::FromFile((Resolve-Path $InPath))
$W = $src.Width
$H = $src.Height
Write-Output ("Image: " + $W + "x" + $H)

# The screenshot is captured at egui's pixels_per_point (typically 2x on Windows).
# Bar logical: 360x44, buttons each 26x26 in logical px.
# So expect actual screenshot ~720x88 with 52x52 buttons.
$ppp = $W / 360.0
Write-Output ("Detected ppp: " + $ppp)
$btn = [Math]::Round(26.0 * $ppp)        # button size
$gap = [Math]::Round(4.0 * $ppp)         # item_spacing default ~4
$pad_right = [Math]::Round(8.0 * $ppp)   # inner_margin right

# Layout right-to-left: × | ↻ | ⏭ | ▶/⏸ | ⏮
# Labels in screen order (left to right): Prev, Play/Pause, Next, Reload, Close
$labels = @("Prev", "PlayPause", "Next", "Reload", "Close")
$n = 5
$rightEdge = $W - $pad_right
$buttons = @()
for ($i = $n - 1; $i -ge 0; $i--) {
    $right  = $rightEdge - ($n - 1 - $i) * ($btn + $gap)
    $left   = $right - $btn
    $top    = [int](($H - $btn) / 2)
    $bottom = $top + $btn
    $buttons += ,@{ Label = $labels[$i]; Left = [int]$left; Right = [int]$right; Top = [int]$top; Bottom = [int]$bottom }
}

# Helper: scan a rect, compute centroid of bright (glyph) pixels.
foreach ($b in $buttons) {
    $sumX = 0.0; $sumY = 0.0; $count = 0
    # Background button bg is medium-dark (~50,50,55), glyph is near-white (~220).
    # Use grayscale > threshold to find glyph pixels.
    for ($y = $b.Top; $y -lt $b.Bottom; $y++) {
        for ($x = $b.Left; $x -lt $b.Right; $x++) {
            if ($x -lt 0 -or $x -ge $W -or $y -lt 0 -or $y -ge $H) { continue }
            $c = $src.GetPixel($x, $y)
            $lum = (0.299 * $c.R + 0.587 * $c.G + 0.114 * $c.B)
            if ($lum -gt 150) {
                $sumX += $x; $sumY += $y; $count++
            }
        }
    }
    if ($count -gt 0) {
        $cx = $sumX / $count
        $cy = $sumY / $count
        $bcx = ($b.Left + $b.Right) / 2.0
        $bcy = ($b.Top + $b.Bottom) / 2.0
        $dx = [Math]::Round($cx - $bcx, 2)
        $dy = [Math]::Round($cy - $bcy, 2)
        Write-Output ("{0,-10} centroid=({1,6:N1},{2,6:N1})  btn_center=({3,6:N1},{4,6:N1})  offset dx={5,5}  dy={6,5}  px(count={7})" -f $b.Label, $cx, $cy, $bcx, $bcy, $dx, $dy, $count)
    } else {
        Write-Output ($b.Label + ": no glyph pixels found")
    }
}

$src.Dispose()
