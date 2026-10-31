#!/bin/sh
# Regenerates the committed SPIR-V from the GLSL (needs glslangValidator).
# The engine itself has no build-time dependency on it: the .spv files are checked in.
set -e
cd "$(dirname "$0")"
glslangValidator -V scene.vert -o scene.vert.spv
glslangValidator -V scene.frag -o scene.frag.spv
