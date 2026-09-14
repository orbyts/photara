#!/usr/bin/env python3
"""Inspect compositor PNGs; never synthesize or edit approval images. Requires Pillow."""
from pathlib import Path
import json
import sys
from PIL import Image

root = Path(sys.argv[1])
native_opening = '--native-opening' in sys.argv[2:]
report = {}
for appearance in ['light', 'dark']:
    for size in ['narrow', 'standard']:
        path = root / f'opening-{appearance}-{size}.png'
        image = Image.open(path).convert('RGB')
        pixels = []
        for x in [0.75, 0.8, 0.85, 0.9]:
            for y in [0.12, 0.18, 0.24]:
                pixels.append(image.getpixel((int(x * image.width), int(y * image.height))))
        if native_opening:
            # Native window colors are owned by macOS, not pinned theme values.
            assert all(max(abs(a - b) for a, b in zip(p, pixels[0])) <= 1 for p in pixels), (path, pixels)
        else:
            expected = 238 if appearance == 'light' else 32
            assert all(max(p) - min(p) <= 1 for p in pixels), (path, pixels)
            assert all(abs(p[0] - expected) <= 1 for p in pixels), (path, pixels)
        report[path.name] = {'pixels': image.size, 'blank_content_srgb': pixels[0], 'samples': len(pixels),
                            'surface_owner': 'macOS' if native_opening else 'Theme Foundation'}
image = Image.open(root / 'ladder.png').convert('RGB')
for offset, appearance, values in [(0, 'light', [238, 247, 255]), (0.5, 'dark', [32, 41, 51])]:
    samples = []
    for (x, y), expected in zip([(0.02, 0.22), (0.035, 0.33), (0.065, 0.45)], values):
        pixel = image.getpixel((int((x + offset) * image.width), int(y * image.height)))
        assert max(pixel) - min(pixel) <= 1 and abs(pixel[0] - expected) <= 1, (appearance, pixel, expected)
        samples.append(pixel)
    report[f'ladder-{appearance}'] = samples
print(json.dumps(report, indent=2))
