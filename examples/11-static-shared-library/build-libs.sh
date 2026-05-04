#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
build_dir="${script_dir}/build"
lib_dir="${script_dir}/lib"

mkdir -p "${build_dir}" "${lib_dir}"

cxx="${CXX:-c++}"

"${cxx}" -std=c++17 -fPIC -I"${script_dir}/input" \
  -c "${script_dir}/native/static_math.cpp" \
  -o "${build_dir}/static_math.o"

ar rcs "${lib_dir}/libnative_static_math.a" "${build_dir}/static_math.o"

"${cxx}" -std=c++17 -fPIC -I"${script_dir}/input" \
  -shared "${script_dir}/native/shared_multiplier.cpp" \
  -o "${lib_dir}/libnative_shared_multiplier.so"
