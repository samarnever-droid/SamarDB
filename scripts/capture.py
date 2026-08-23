import sys
import os
import subprocess
import time

try:
    from PIL import ImageGrab
    has_pil = True
except ImportError:
    has_pil = False

def capture(out_path):
    os.makedirs(os.path.dirname(os.path.abspath(out_path)), exist_ok=True)
    if has_pil:
        im = ImageGrab.grab()
        im.save(out_path)
        print(f"Captured screenshot to {out_path} using PIL")
    else:
        # PowerShell fallback
        ps_cmd = f"""
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$graphics = [System.Drawing.Graphics]::FromImage($bmp)
$graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
$bmp.Save('{out_path}', [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bmp.Dispose()
"""
        subprocess.run(["powershell", "-Command", ps_cmd], check=True)
        print(f"Captured screenshot to {out_path} using PowerShell")

if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "screenshot.png"
    capture(out)
