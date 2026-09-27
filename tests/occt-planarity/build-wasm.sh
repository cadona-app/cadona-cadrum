#!/usr/bin/env bash
set -euo pipefail

occt_root=${1:?Pass the extracted OCCT package directory}
output=${2:?Pass the output WebAssembly path}
probe_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
wasi_sdk=${WASI_SDK_PATH:-/opt/wasi-sdk}

# Use the package's C++ exception runtimes, matching the application's prebuilt link path.
"$wasi_sdk/bin/clang++" \
  --target=wasm32-wasip1 --sysroot="$wasi_sdk/share/wasi-sysroot" \
  -fwasm-exceptions -mllvm -wasm-use-legacy-eh=true -std=c++17 -O2 \
  -D_WASI_EMULATED_PROCESS_CLOCKS -D_WASI_EMULATED_SIGNAL \
  -D_WASI_EMULATED_MMAN -D_WASI_EMULATED_GETPID \
  -I"$occt_root/include/opencascade" "$probe_directory/planarity.cpp" \
  -L"$occt_root/lib" -nostdlib++ \
  -lTKGeomBase -lTKG3d -lTKG2d -lTKMath -lTKernel \
  -lc++ -lcadrum_c++abi -lcadrum_unwind -lcadrum_c \
  -lwasi-emulated-process-clocks -lwasi-emulated-signal \
  -lwasi-emulated-mman -lwasi-emulated-getpid -o "$output"
