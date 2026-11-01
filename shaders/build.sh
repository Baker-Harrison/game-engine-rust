#!/bin/sh
set -e
cd "$(dirname "$0")"
glslangValidator -V scene.vert -o scene.vert.spv
glslangValidator -V scene.frag -o scene.frag.spv
glslangValidator -V shadow.vert -o shadow.vert.spv
