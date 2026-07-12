#!/usr/bin/env python3
"""Generate a retro CRT terminal icon (pixel art) as PNG, no deps."""
import struct, zlib, sys

GRID = 64          # design grid
SCALE = 16         # 64*16 = 1024px
W = H = GRID * SCALE

# palette
T   = None                    # transparent
BZ1 = (0x2b, 0x2b, 0x28, 255) # bezel dark
BZ2 = (0x3a, 0x3a, 0x35, 255) # bezel light (top highlight)
BZ3 = (0x1e, 0x1e, 0x1c, 255) # bezel shadow
SCR = (0x02, 0x0a, 0x02, 255) # screen bg
SCN = (0x04, 0x14, 0x04, 255) # scanline
GRN = (0x33, 0xff, 0x33, 255) # phosphor
GLW = (0x11, 0x55, 0x11, 255) # glow halo
FT  = (0x15, 0x15, 0x13, 255) # stand

px = [[T]*GRID for _ in range(GRID)]

def rect(x0, y0, x1, y1, c):
    for y in range(y0, y1+1):
        for x in range(x0, x1+1):
            if 0 <= x < GRID and 0 <= y < GRID:
                px[y][x] = c

# --- monitor body (rounded) ---
BX0, BY0, BX1, BY1 = 4, 6, 59, 49
rect(BX0, BY0, BX1, BY1, BZ1)
# round corners (cut 2px steps)
for (cx, cy, dx, dy) in [(BX0,BY0,1,1),(BX1,BY0,-1,1),(BX0,BY1,1,-1),(BX1,BY1,-1,-1)]:
    px[cy][cx] = T
    px[cy][cx+dx] = T
    px[cy+dy][cx] = T
# top highlight
rect(BX0+2, BY0+1, BX1-2, BY0+1, BZ2)
# bottom shadow
rect(BX0+2, BY1-1, BX1-2, BY1, BZ3)

# --- screen inset ---
SX0, SY0, SX1, SY1 = 9, 10, 54, 43
rect(SX0-1, SY0-1, SX1+1, SY1+1, BZ3)   # screen bezel lip
rect(SX0, SY0, SX1, SY1, SCR)
# scanlines
for y in range(SY0, SY1+1, 2):
    rect(SX0, y, SX1, y, SCN)

# --- prompt "> " + block cursor (chunky 70s glyph) ---
# ">" arrow: 3px thick strokes
gx, gy = 16, 20   # glyph origin
th = 3
for i in range(7):
    rect(gx+i, gy+i, gx+i+th-1, gy+i, GRN)      # down stroke
for i in range(7):
    rect(gx+i, gy+12-i, gx+i+th-1, gy+12-i, GRN) # up stroke
# block cursor
rect(gx+15, gy+7, gx+23, gy+12, GRN)
# (glow line removed)


# --- stand ---
rect(26, 50, 37, 52, FT)
rect(18, 53, 45, 55, BZ1)
rect(18, 55, 45, 55, BZ3)

# --- emit PNG scaled with soft CRT vignette on screen ---
def at(x, y):
    gxp, gyp = x // SCALE, y // SCALE
    c = px[gyp][gxp]
    return c if c else (0, 0, 0, 0)

rows = []
for y in range(H):
    row = bytearray()
    row.append(0)
    for x in range(W):
        r, g, b, a = at(x, y)
        row += bytes((r, g, b, a))
    rows.append(bytes(row))

raw = b"".join(rows)
def chunk(tag, data):
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag+data))
png = (b"\x89PNG\r\n\x1a\n"
       + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 6, 0, 0, 0))
       + chunk(b"IDAT", zlib.compress(raw, 9))
       + chunk(b"IEND", b""))
out = sys.argv[1] if len(sys.argv) > 1 else "icon_1024.png"
open(out, "wb").write(png)
print(f"wrote {out} ({W}x{H})")
