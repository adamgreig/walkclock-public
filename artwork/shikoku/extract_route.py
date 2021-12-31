"""
Process all layers named "Day *", extracting the sequence of pixels to set
and how many to set per day.

Then, process the layer named "Temples" and output an ordered list of temples.
"""

import sys
from gimpformats.gimpXcfDocument import GimpDocument

if len(sys.argv) != 2 or not sys.argv[1].endswith(".xcf"):
    print(f"Usage: {sys.argv[0]} <xcf to extract>")

doc = GimpDocument(sys.argv[1])
n_days = sum(1 for layer in doc.layers if layer.name.startswith("Day "))
print(f"Found {len(doc.layers)} layers, of which {n_days} contain day routes.")

days = {}

for layer in doc.layers:
    if not layer.name.startswith("Day "):
        continue
    print(f"Processing {layer.name}... ", end='', flush=True)
    day = int(layer.name.split(" ")[1])
    days[day] = []
    for x in range(64):
        for y in range(64):
            px = layer.image.getpixel((x, y))
            if px[3] == 255:
                days[day].append((px[0], x, y))
    days[day].sort(reverse=True)
    days[day] = [(p[1], p[2]) for p in days[day]]
    print(f"{len(days[day])} pixels.")

route_pixels = []
day_indices = []

for day in sorted(days.keys()):
    for px in days[day]:
        if px in route_pixels:
            print(f"WARNING: Duplicate pixel {px}")
        route_pixels.append(px)
    day_indices.append(len(route_pixels))

print("pub static ROUTE: [(u8, u8); 292] = [")
for pxs in [route_pixels[i:i+8] for i in range(0, len(route_pixels), 8)]:
    print("    ", ", ".join(str(px) for px in pxs), ",", sep="")
print("];")

print("pub static DAYS: [u16; 52] = [")
print("   ", ", ".join(str(d) for d in [0] + day_indices))
print("];")

temples = []
for layer in doc.layers:
    if layer.name != "Temples":
        continue
    for x in range(64):
        for y in range(64):
            px = layer.image.getpixel((x, y))
            if px == (255, 0, 0, 255):
                temples.append((x, y))
temples.sort(key=lambda px: route_pixels.index(px))

print("pub static TEMPLES: [(u8, u8); 88] = [")
for pxs in [temples[i:i+8] for i in range(0, len(temples), 8)]:
    print("    ", ", ".join(str(px) for px in pxs), ",", sep="")
print("];")
