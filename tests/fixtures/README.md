# Exact geometry regression fixtures

`wing_section_rib.brep` is a synthetic 2 mm thick internal rib generated from Cadona's default
NACA 2412 wing on Linux arm64 with OCCT 8.0.0 rev5 (client `f74c8abf`, Cadrum `07000db`). It is
body index 2 of feature ID 1 with internal structure enabled, and has two long curved faces and two planar
caps. The 7,410-byte binary B-rep preserves the coordinates that reproduced the failure on both
Linux and macOS; regenerating it on macOS changed the last few floating-point bits and hid the
standard-quality failure.

The original Rust mesher failed both the absolute preview request (0.05 mm, 0.5 rad) and the
relative standard request (0.004, 0.25 rad). Aspect refinement repeatedly split anisotropic
boundary collars already exempt from the final aspect audit, leaving rejected interior slivers.
The regression checks every triangle's winding and manifold continuity, exact shared-edge
identity, and aspect ratios in the regular interior. The final mesher audits remain unchanged;
exact-edge transition cells keep their existing aspect exemptions.
