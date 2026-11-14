# Starts the built program on Windows and checks that it opens its main window, draws
# into it and stays open. CI runs this on a machine without a graphics card, with the
# Mesa software renderer placed next to the executable.
#
#   pwsh tools/smoke-test.ps1 [-Binary path\to\rucheckers.exe] [-Screenshot window.png]
param(
    [string]$Binary = "target\release\rucheckers.exe",
    [string]$Screenshot = ""
)
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class NativeWindow {
    [StructLayout(LayoutKind.Sequential)]
    public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetClassName(IntPtr window, StringBuilder name, int capacity);
    [DllImport("user32.dll")]
    public static extern bool SetProcessDPIAware();
}
"@
# Without this the script would measure a scaled screen in scaled coordinates.
[void][NativeWindow]::SetProcessDPIAware()

# The settings stay out of the profile of the user; the language does not depend on
# the machine.
$work = Join-Path ([IO.Path]::GetTempPath()) ("rucheckers-smoke-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $work | Out-Null
$env:APPDATA = $work
$env:LC_ALL = "en_US.UTF-8"

$app = Start-Process -FilePath $Binary -PassThru
try {
    # A program that cannot open its window shows a message box under the same title:
    # the main window is told apart by its class and its size.
    $rect = New-Object NativeWindow+Rect
    $found = $false
    foreach ($attempt in 1..30) {
        Start-Sleep -Seconds 1
        $app.Refresh()
        if ($app.HasExited) { throw "the program exited with code $($app.ExitCode)" }
        $window = $app.MainWindowHandle
        if ($window -eq [IntPtr]::Zero) { continue }
        $class = New-Object System.Text.StringBuilder 256
        [void][NativeWindow]::GetClassName($window, $class, $class.Capacity)
        if ($class.ToString() -eq "#32770") { throw "the program reported a failure instead of opening its window" }
        [void][NativeWindow]::GetWindowRect($window, [ref]$rect)
        if (($rect.Right - $rect.Left) -ge 400 -and ($rect.Bottom - $rect.Top) -ge 300) {
            $found = $true
            break
        }
    }
    if (-not $found) { throw "the window did not appear" }
    if ($app.MainWindowTitle -ne "Russian Draughts") { throw "unexpected window title: '$($app.MainWindowTitle)'" }

    # The first frames take a moment with the software renderer.
    Start-Sleep -Seconds 5
    $app.Refresh()
    if ($app.HasExited) { throw "the program exited after opening its window" }

    # An empty window is one flat colour; the board with its pieces has thousands.
    [void][NativeWindow]::GetWindowRect($app.MainWindowHandle, [ref]$rect)
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    $bitmap = New-Object System.Drawing.Bitmap $width, $height
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
    $graphics.Dispose()
    $colours = New-Object 'System.Collections.Generic.HashSet[int]'
    for ($y = 0; $y -lt $height; $y += 2) {
        for ($x = 0; $x -lt $width; $x += 2) {
            [void]$colours.Add($bitmap.GetPixel($x, $y).ToArgb())
        }
    }
    if ($Screenshot) { $bitmap.Save($Screenshot, [System.Drawing.Imaging.ImageFormat]::Png) }
    $bitmap.Dispose()
    if ($colours.Count -lt 1000) { throw "the window is not drawn: $($colours.Count) colours in it" }
    Write-Host "smoke test (windows) passed: a ${width}x${height} window is open, $($colours.Count) colours in it"
}
finally {
    if (-not $app.HasExited) { Stop-Process -Id $app.Id -Force }
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
