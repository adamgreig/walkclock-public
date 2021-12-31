#!/bin/bash
set -eu

for f in crops/*; do
    convert "$f" \
        -resize 64x64 \
        -strokewidth 0 \
        -fill "rgba(0,0,0,0.5)" \
        -draw "rectangle 0,0 64,7" \
        -strip \
        -quality 80 \
        -sampling-factor 4:4:4 \
        "resized/$(basename "$f")"
done
