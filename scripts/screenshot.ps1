param(
    [string]$OutPath = "scripts/last.png"
)

Add-Type -AssemblyName System.Drawing, System.Windows.Forms

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
    [DllImport("user32.dll")] public static extern IntPtr SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

$proc = Get-Process workshop-timer -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $proc) {
    Write-Error "workshop-timer process not running"
    exit 1
}
$hwnd = $proc.MainWindowHandle
if ($hwnd -eq [IntPtr]::Zero) {
    Write-Error "workshop-timer has no main window yet"
    exit 1
}

# Try to bring it forward and ensure it's drawn
[Win]::ShowWindow($hwnd, 5) | Out-Null   # SW_SHOW
[Win]::SetForegroundWindow($hwnd) | Out-Null
Start-Sleep -Milliseconds 200

$r = New-Object Win+RECT
[void][Win]::GetWindowRect($hwnd, [ref]$r)
$w = $r.Right - $r.Left
$h = $r.Bottom - $r.Top
if ($w -le 0 -or $h -le 0) { Write-Error "Bad window rect"; exit 1 }

$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.Clear([System.Drawing.Color]::FromArgb(255, 30, 30, 30))  # opaque gray bg
$g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)

$bmp.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()

Write-Output "Saved $OutPath"
Write-Output "Size: $w by $h"
Write-Output "Position: $($r.Left), $($r.Top)"
