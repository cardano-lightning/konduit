#!/usr/bin/env bash

pandoc index.md \
    --lua-filter=d2-filter.lua \
    --standalone \
    --embed-resources \
    --toc \
    -f markdown \
    --css ./assets/style.css \
    -o index.html

echo "✅ Build Complete! Open index.html to view."
