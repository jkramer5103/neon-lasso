#!/usr/bin/env python3
"""Render a `tmux capture-pane -e -p` dump (truecolor SGR) to a PNG."""
import re
import sys
from PIL import Image, ImageDraw, ImageFont

CW, CH = 9, 18
FONT = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 14)


def main(src, dst, cols, rows):
    data = open(src, encoding="utf-8", errors="replace").read()
    img = Image.new("RGB", (cols * CW, rows * CH), (0, 0, 0))
    d = ImageDraw.Draw(img)
    fg, bg = (200, 200, 200), (0, 0, 0)
    tok = re.compile(r"\x1b\[([0-9;]*)m|(\n)|(.)", re.S)
    x = y = 0
    for m in tok.finditer(data):
        if m.group(1) is not None:
            p = [int(v) if v else 0 for v in m.group(1).split(";")]
            i = 0
            while i < len(p):
                v = p[i]
                if v == 0:
                    fg, bg = (200, 200, 200), (0, 0, 0)
                elif v == 38 and i + 4 < len(p) + 0 and p[i + 1] == 2:
                    fg = tuple(p[i + 2:i + 5]); i += 4
                elif v == 48 and p[i + 1] == 2:
                    bg = tuple(p[i + 2:i + 5]); i += 4
                elif v == 39:
                    fg = (200, 200, 200)
                elif v == 49:
                    bg = (0, 0, 0)
                i += 1
        elif m.group(2):
            x, y = 0, y + 1
        else:
            ch = m.group(3)
            if y >= rows:
                break
            if x < cols:
                x0, y0 = x * CW, y * CH
                d.rectangle([x0, y0, x0 + CW - 1, y0 + CH - 1], fill=bg)
                if ch == "▀":
                    d.rectangle([x0, y0, x0 + CW - 1, y0 + CH // 2 - 1], fill=fg)
                elif ch != " ":
                    d.text((x0, y0 + 1), ch, font=FONT, fill=fg)
            x += 1
    img.save(dst)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]))
