# OCCT 8 extrusion planarity correction

This build branch prepares `occt-8_0_0_cadona1` in `cadona-app/cadona-cadrum`.
The workflow creates a **draft** release. Do not merge the dependency revision or update
Cadona's Cadrum pin until the complete packages are qualified and downloadable. The existing
client continues using OCCT 8.0.0 rev5 in the meantime.

## Defect and correction

`GeomLib_IsPlanarSurface` computes the extrusion normal as `Du cross Dv`, then tests its
perpendicularity against the extrusion direction with angular tolerance
`Tol / (Vmax - Vmin)`. An unbounded extrusion has V bounds near +/-2e100, making that
tolerance far smaller than floating-point roundoff. Exactly planar extrusions can therefore
be classified as nonplanar. Cadona's live sliver extrusion then retains extra faces during
same-domain cleanup on Linux, breaking downstream fillet and chamfer references.

The source patch removes that redundant angular test: the extrusion's V derivative is its
direction, so the computed cross product is already perpendicular by construction. It keeps
the existing nondegenerate-normal guard and basis-curve check against the plane at the
original linear tolerance. It does not change modeling tolerances or repair feature inputs.
The patch fails if the upstream fragment is missing or duplicated and is safe to apply twice.
The modified OCCT source remains included in the packaged source distribution.

Upstream source:
[GeomLib_IsPlanarSurface.cxx at V8_0_0](https://github.com/Open-Cascade-SAS/OCCT/blob/V8_0_0/src/ModelingData/TKGeomBase/GeomLib/GeomLib_IsPlanarSurface.cxx).

## Regression checks

`tests/occt-planarity` directly tests the library, avoiding fixture serialization that can
change the last few floating-point bits and hide this defect. It constructs 100 cubic
B-spline extrusion surfaces in known planes, tests both unbounded and bounded versions,
checks points against each returned plane, and repeats with an out-of-plane control point.
A circular extrusion and a degenerate parallel-line extrusion must remain nonplanar.

Run against an installed OCCT tree:

```sh
cmake -S tests/occt-planarity -B out/planarity \
  -DOCCT_ROOT=/absolute/path/to/occt -DCMAKE_BUILD_TYPE=Release
cmake --build out/planarity --config Release
ctest --test-dir out/planarity -C Release --output-on-failure
```

The original rev5 libraries fail this regression on macOS arm64 and Linux arm64. The
isolated corrected libraries pass all 402 checks on both, and Cadona's original
`face_blend_regression` and `blended_sliver_boundaries_preserve_display_topology` cases
pass unchanged on both. This is initial evidence, not full package qualification.

The prebuilt workflow builds all seven existing targets and runs the native regression
on Linux x86_64/arm64, macOS x86_64/arm64, and Windows MSVC. Windows GNU and WebAssembly
still require execution qualification before the client dependency can be adopted.
