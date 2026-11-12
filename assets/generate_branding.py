"""Export the original programmatic RaceDesk tray mark as reusable assets."""
from pathlib import Path
from PIL import Image

folder = Path(__file__).parent
image = Image.new('RGBA', (32, 32), (21, 29, 40, 255))
rectangles = []
for y in range(32):
    for x in range(32):
        if 7 <= x < 25 and 6 <= y < 20 and (x // 4 + y // 4) % 2 == 0:
            image.putpixel((x, y), (110, 231, 183, 255))
for y in range(4, 20, 4):
    for x in range(4, 28, 4):
        if (x // 4 + y // 4) % 2 == 0:
            left, right = max(x, 7), min(x + 4, 25)
            top, bottom = max(y, 6), min(y + 4, 20)
            if left < right and top < bottom:
                rectangles.append(f'<rect x="{left}" y="{top}" width="{right-left}" height="{bottom-top}"/>')
large = image.resize((512, 512), Image.Resampling.NEAREST)
large.save(folder / 'racedesk-logo.png')
large.save(folder / 'racedesk.ico', sizes=[(n, n) for n in [16, 24, 32, 48, 64, 128, 256]])
(folder / 'racedesk-64.rgba').write_bytes(image.resize((64, 64), Image.Resampling.NEAREST).tobytes())
prefix = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" shape-rendering="crispEdges"><title>RaceDesk racing flag</title>'
mark = '<g fill="#6ee7b7">' + ''.join(rectangles) + '</g></svg>'
(folder / 'racedesk-logo.svg').write_text(prefix + '<rect width="32" height="32" fill="#151d28"/>' + mark, encoding='utf-8')
(folder / 'racedesk-symbol.svg').write_text(prefix + mark, encoding='utf-8')
