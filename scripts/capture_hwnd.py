import ctypes
from ctypes import wintypes
import sys
import os

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32

def capture_window(title_substring, out_path):
    # Find window
    hwnd_found = []
    def enum_cb(hwnd, lparam):
        if user32.IsWindowVisible(hwnd):
            length = user32.GetWindowTextLengthW(hwnd)
            if length > 0:
                buff = ctypes.create_unicode_buffer(length + 1)
                user32.GetWindowTextW(hwnd, buff, length + 1)
                if title_substring.lower() in buff.value.lower():
                    hwnd_found.append((hwnd, buff.value))
        return True

    WNDENUMPROC = ctypes.WINFUNCTYPE(ctypes.c_bool, wintypes.HWND, wintypes.LPARAM)
    user32.EnumWindows(WNDENUMPROC(enum_cb), 0)

    if not hwnd_found:
        print(f"Window with title containing '{title_substring}' not found.")
        # Fallback to desktop window
        hwnd = user32.GetDesktopWindow()
        title = "Desktop"
    else:
        hwnd, title = hwnd_found[0]
        print(f"Found window: '{title}' (HWND: {hwnd})")

    rect = wintypes.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(rect))
    width = rect.right - rect.left
    height = rect.bottom - rect.top

    if width <= 0 or height <= 0:
        width = 1280
        height = 800

    hwnd_dc = user32.GetWindowDC(hwnd)
    mem_dc = gdi32.CreateCompatibleDC(hwnd_dc)
    bitmap = gdi32.CreateCompatibleBitmap(hwnd_dc, width, height)
    old_bitmap = gdi32.SelectObject(mem_dc, bitmap)

    # Use PrintWindow with PW_RENDERFULLCONTENT (2) or BitBlt
    PW_RENDERFULLCONTENT = 2
    res = user32.PrintWindow(hwnd, mem_dc, PW_RENDERFULLCONTENT)
    if not res:
        gdi32.BitBlt(mem_dc, 0, 0, width, height, hwnd_dc, 0, 0, 0x00CC0020) # SRCCOPY

    # Save to BMP
    class BITMAPINFOHEADER(ctypes.Structure):
        _fields_ = [
            ('biSize', wintypes.DWORD),
            ('biWidth', wintypes.LONG),
            ('biHeight', wintypes.LONG),
            ('biPlanes', wintypes.WORD),
            ('biBitCount', wintypes.WORD),
            ('biCompression', wintypes.DWORD),
            ('biSizeImage', wintypes.DWORD),
            ('biXPelsPerMeter', wintypes.LONG),
            ('biYPelsPerMeter', wintypes.LONG),
            ('biClrUsed', wintypes.DWORD),
            ('biClrImportant', wintypes.DWORD)
        ]

    bi = BITMAPINFOHEADER()
    bi.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    bi.biWidth = width
    bi.biHeight = -height  # top-down
    bi.biPlanes = 1
    bi.biBitCount = 32
    bi.biCompression = 0 # BI_RGB

    buf_size = width * height * 4
    pixels = ctypes.create_string_buffer(buf_size)
    gdi32.GetDIBits(mem_dc, bitmap, 0, height, pixels, ctypes.byref(bi), 0)

    # Cleanup GDI
    gdi32.SelectObject(mem_dc, old_bitmap)
    gdi32.DeleteObject(bitmap)
    gdi32.DeleteDC(mem_dc)
    user32.ReleaseDC(hwnd, hwnd_dc)

    # Save as PNG via PIL or uncompressed BMP
    bmp_header_size = 14
    info_header_size = 40
    file_size = bmp_header_size + info_header_size + buf_size

    # BMP File Header
    bm_magic = b'BM'
    bf_reserved = 0
    bf_off_bits = bmp_header_size + info_header_size

    import struct
    header = struct.pack('<2sIHHI', bm_magic, file_size, 0, 0, bf_off_bits)
    info = struct.pack('<IIIHHIIIIII', info_header_size, width, height, 1, 32, 0, buf_size, 0, 0, 0, 0)

    # Note: BMP is bottom-up if biHeight is positive
    # Flip lines for BMP format:
    flipped_pixels = bytearray(buf_size)
    row_bytes = width * 4
    for y in range(height):
        src_row = (height - 1 - y) * row_bytes
        dst_row = y * row_bytes
        flipped_pixels[dst_row:dst_row+row_bytes] = pixels[src_row:src_row+row_bytes]

    os.makedirs(os.path.dirname(os.path.abspath(out_path)), exist_ok=True)
    bmp_path = out_path if out_path.endswith('.bmp') else out_path + '.bmp'
    with open(bmp_path, 'wb') as f:
        f.write(header)
        f.write(info)
        f.write(flipped_pixels)

    print(f"Captured window image saved to: {bmp_path}")
    try:
        from PIL import Image
        img = Image.open(bmp_path)
        img.save(out_path)
        print(f"Converted to PNG: {out_path}")
    except Exception as e:
        pass

if __name__ == "__main__":
    title = sys.argv[1] if len(sys.argv) > 1 else "ApexCode"
    out = sys.argv[2] if len(sys.argv) > 2 else "screenshot.png"
    capture_window(title, out)
