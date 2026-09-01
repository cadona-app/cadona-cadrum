use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::TAU;

use cadrum::{BSplineEnd, DVec3, Edge, MeshChunks, ProfileOrient, Solid, Tessellation, TopologyQueryOptions, TopologySnapshot};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ExactPoint([u64; 3]);

impl ExactPoint {
	fn new(point: DVec3) -> Self {
		Self(point.to_array().map(|coordinate| if coordinate == 0.0 { 0 } else { coordinate.to_bits() }))
	}
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ExactSegment {
	first: ExactPoint,
	second: ExactPoint,
}

impl ExactSegment {
	fn new(first: DVec3, second: DVec3) -> Option<(Self, i32)> {
		let first = ExactPoint::new(first);
		let second = ExactPoint::new(second);
		match first.cmp(&second) {
			std::cmp::Ordering::Less => Some((Self { first, second }, 1)),
			std::cmp::Ordering::Greater => Some((Self { first: second, second: first }, -1)),
			std::cmp::Ordering::Equal => None,
		}
	}
}

#[derive(Debug, Default)]
struct SegmentUses {
	total: usize,
	orientation_balance: i32,
	faces: BTreeMap<u32, usize>,
}

#[derive(Clone, Copy, Debug)]
struct WorstTriangle {
	face: u32,
	triangle: usize,
	positions: [DVec3; 3],
	value: f64,
}

#[derive(Clone, Copy, Debug)]
struct MeshQuality {
	triangle_count: usize,
	worst_aspect: WorstTriangle,
	minimum_angle_degrees: WorstTriangle,
	worst_normal_alignment: WorstTriangle,
}

fn absolute_options(linear: f64) -> Tessellation {
	Tessellation { deflection_linear: linear, deflection_angular: 0.15, relative_linear: false, include_edges: true, parallel: true }
}

fn relative_options() -> Tessellation {
	Tessellation { deflection_linear: 0.004, deflection_angular: 0.15, relative_linear: true, include_edges: true, parallel: true }
}

fn exact_surface_segment_uses(chunks: &MeshChunks) -> BTreeMap<ExactSegment, SegmentUses> {
	let mut uses = BTreeMap::<ExactSegment, SegmentUses>::new();
	for face in &chunks.faces {
		for triangle in face.indices.chunks_exact(3) {
			for [first, second] in [[triangle[0], triangle[1]], [triangle[1], triangle[2]], [triangle[2], triangle[0]]] {
				let first = face.vertices[first as usize];
				let second = face.vertices[second as usize];
				let Some((segment, orientation)) = ExactSegment::new(first, second) else {
					continue;
				};
				let entry = uses.entry(segment).or_default();
				entry.total += 1;
				entry.orientation_balance += orientation;
				*entry.faces.entry(face.face_index).or_default() += 1;
			}
		}
	}
	uses
}

fn assert_global_shell_identity(name: &str, chunks: &MeshChunks, topology: &TopologySnapshot) {
	let surface_uses = exact_surface_segment_uses(chunks);
	let invalid_surface_segments = surface_uses.iter().filter(|(_, uses)| uses.total != 2 || uses.orientation_balance != 0).take(8).collect::<Vec<_>>();
	assert!(invalid_surface_segments.is_empty(), "{name} has cracks, multiplicity errors, or inconsistent winding: {invalid_surface_segments:?}");

	let mut audited_semantic_segments = 0;
	for edge in &chunks.edges {
		let facts = topology.edge_facts(edge.edge_index).expect("mesh edge ordinal must exist in the topology snapshot");
		let expected_faces = topology.edge_faces(edge.edge_index).expect("mesh edge ordinal must have face incidence").iter().copied().collect::<BTreeSet<_>>();
		for points in edge.points.windows(2) {
			let Some((segment, _)) = ExactSegment::new(points[0], points[1]) else {
				assert!(facts.degenerate, "{name} edge {} unexpectedly contains a collapsed segment", edge.edge_index);
				continue;
			};
			let uses = surface_uses.get(&segment).unwrap_or_else(|| panic!("{name} semantic edge {} does not use bit-identical surface boundary positions: {points:?}", edge.edge_index));
			audited_semantic_segments += 1;
			assert_eq!(uses.total, 2, "{name} semantic edge {} has the wrong global triangle multiplicity", edge.edge_index);
			assert_eq!(uses.orientation_balance, 0, "{name} semantic edge {} is not oppositely wound", edge.edge_index);
			let actual_faces = uses.faces.keys().copied().collect::<BTreeSet<_>>();
			assert!(actual_faces.is_subset(&expected_faces), "{name} semantic edge {} is used by unexpected faces: expected {expected_faces:?}, actual {actual_faces:?}", edge.edge_index);
			if facts.manifold && !facts.seam {
				assert_eq!(expected_faces.len(), 2, "{name} ordinary manifold edge {} does not have two incident faces", edge.edge_index);
				assert_eq!(actual_faces, expected_faces, "{name} ordinary manifold edge {} is missing an incident face", edge.edge_index);
				assert!(uses.faces.values().all(|count| *count == 1), "{name} ordinary manifold edge {} is not represented exactly once per face", edge.edge_index);
			} else if facts.seam {
				assert_eq!(actual_faces.len(), 1, "{name} periodic seam edge {} should occur twice on one face", edge.edge_index);
				assert_eq!(uses.faces.values().copied().sum::<usize>(), 2, "{name} periodic seam edge {} should occur twice in its incident face", edge.edge_index);
			}
		}
	}
	assert!(audited_semantic_segments > 0, "{name} did not expose any non-degenerate semantic edge segments");
}

fn assert_global_mesh_manifold(name: &str, chunks: &MeshChunks) {
	let surface_uses = exact_surface_segment_uses(chunks);
	let invalid_surface_segments = surface_uses.iter().filter(|(_, uses)| uses.total != 2 || uses.orientation_balance != 0).take(8).collect::<Vec<_>>();
	assert!(invalid_surface_segments.is_empty(), "{name} has cracks, multiplicity errors, or inconsistent winding: {invalid_surface_segments:?}");
}

fn triangle_minimum_angle_degrees(positions: [DVec3; 3]) -> f64 {
	(0..3)
		.map(|corner| {
			let first = positions[(corner + 1) % 3] - positions[corner];
			let second = positions[(corner + 2) % 3] - positions[corner];
			first.normalize().dot(second.normalize()).clamp(-1.0, 1.0).acos().to_degrees()
		})
		.fold(f64::INFINITY, f64::min)
}

fn mesh_quality(name: &str, chunks: &MeshChunks) -> MeshQuality {
	let mut triangle_count = 0;
	let mut worst_aspect = WorstTriangle { face: 0, triangle: 0, positions: [DVec3::ZERO; 3], value: f64::NEG_INFINITY };
	let mut minimum_angle_degrees = WorstTriangle { face: 0, triangle: 0, positions: [DVec3::ZERO; 3], value: f64::INFINITY };
	let mut worst_normal_alignment = WorstTriangle { face: 0, triangle: 0, positions: [DVec3::ZERO; 3], value: f64::INFINITY };

	for face in &chunks.faces {
		assert_eq!(face.vertices.len(), face.normals.len(), "{name} face {} has mismatched vertices and normals", face.face_index);
		assert!(face.indices.len().is_multiple_of(3), "{name} face {} has a partial triangle", face.face_index);
		assert!(face.vertices.iter().all(|point| point.is_finite()), "{name} face {} has a non-finite vertex", face.face_index);
		assert!(face.normals.iter().all(|normal| normal.is_finite() && (normal.length() - 1.0).abs() <= 1.0e-6), "{name} face {} has an invalid exact-surface normal", face.face_index);

		for (triangle_index, triangle) in face.indices.chunks_exact(3).enumerate() {
			assert!(triangle.iter().all(|index| (*index as usize) < face.vertices.len()), "{name} face {} triangle {triangle_index} has an out-of-range index", face.face_index);
			let positions = [triangle[0], triangle[1], triangle[2]].map(|index| face.vertices[index as usize]);
			let edges = [positions[1] - positions[0], positions[2] - positions[1], positions[0] - positions[2]];
			let longest_squared = edges.into_iter().map(DVec3::length_squared).fold(0.0_f64, f64::max);
			let area_normal = (positions[1] - positions[0]).cross(positions[2] - positions[0]);
			let double_area = area_normal.length();
			assert!(longest_squared > 0.0 && double_area > longest_squared * 1.0e-12, "{name} face {} triangle {triangle_index} is degenerate: {positions:?}", face.face_index);
			let averaged_normal = triangle.iter().map(|index| face.normals[*index as usize]).sum::<DVec3>().normalize();
			let normal_alignment = area_normal.normalize().dot(averaged_normal);
			if normal_alignment < worst_normal_alignment.value {
				worst_normal_alignment = WorstTriangle { face: face.face_index, triangle: triangle_index, positions, value: normal_alignment };
			}
			let aspect = longest_squared / double_area;
			if aspect > worst_aspect.value {
				worst_aspect = WorstTriangle { face: face.face_index, triangle: triangle_index, positions, value: aspect };
			}
			let minimum_angle = triangle_minimum_angle_degrees(positions);
			if minimum_angle < minimum_angle_degrees.value {
				minimum_angle_degrees = WorstTriangle { face: face.face_index, triangle: triangle_index, positions, value: minimum_angle };
			}
			triangle_count += 1;
		}
	}

	assert!(triangle_count > 0, "{name} has no triangles");
	let quality = MeshQuality { triangle_count, worst_aspect, minimum_angle_degrees, worst_normal_alignment };
	eprintln!(
		"{name}: {} triangles; worst aspect {:.6} at face {} triangle {} {:?}; minimum angle {:.6} degrees at face {} triangle {} {:?}; worst exact-normal alignment {:.6} at face {} triangle {} {:?}",
		quality.triangle_count, quality.worst_aspect.value, quality.worst_aspect.face, quality.worst_aspect.triangle, quality.worst_aspect.positions, quality.minimum_angle_degrees.value, quality.minimum_angle_degrees.face, quality.minimum_angle_degrees.triangle, quality.minimum_angle_degrees.positions, quality.worst_normal_alignment.value, quality.worst_normal_alignment.face, quality.worst_normal_alignment.triangle, quality.worst_normal_alignment.positions
	);
	quality
}

fn assert_hard_quality(name: &str, quality: MeshQuality, maximum_aspect: f64, minimum_angle_degrees: f64) {
	assert!(quality.worst_normal_alignment.value > 0.0, "{name} contains a triangle wound against its exact-surface normal: {:?}", quality.worst_normal_alignment);
	assert!(quality.worst_aspect.value <= maximum_aspect, "{name} worst triangle aspect {} exceeds the hard limit {maximum_aspect}: {:?}", quality.worst_aspect.value, quality.worst_aspect);
	assert!(quality.minimum_angle_degrees.value >= minimum_angle_degrees, "{name} minimum triangle angle {} degrees is below the hard limit {minimum_angle_degrees}: {:?}", quality.minimum_angle_degrees.value, quality.minimum_angle_degrees);
}

fn assert_exact_interior_error(name: &str, solid: &Solid, chunks: &MeshChunks, maximum_error: f64) {
	let faces = solid.iter_face().collect::<Vec<_>>();
	let barycentric_samples = [[1.0 / 3.0; 3], [0.6, 0.2, 0.2], [0.2, 0.6, 0.2], [0.2, 0.2, 0.6]];
	let mut worst_error = 0.0_f64;
	let mut worst_sample = None;
	for face_mesh in &chunks.faces {
		let face = faces.get(face_mesh.face_index as usize).expect("mesh face ordinal must exist in exact shape");
		for (triangle_index, triangle) in face_mesh.indices.chunks_exact(3).enumerate() {
			let positions = [triangle[0], triangle[1], triangle[2]].map(|index| face_mesh.vertices[index as usize]);
			for weights in barycentric_samples {
				let sample = positions[0] * weights[0] + positions[1] * weights[1] + positions[2] * weights[2];
				let (exact, _) = face.project(sample).unwrap_or_else(|error| panic!("{name} could not project a face-interior sample on face {} triangle {triangle_index}: {error:?}", face_mesh.face_index));
				let error = sample.distance(exact);
				if error > worst_error {
					worst_error = error;
					worst_sample = Some((face_mesh.face_index, triangle_index, weights, sample, exact));
				}
			}
		}
	}
	eprintln!("{name}: worst sampled triangle-interior surface error {worst_error:.12} at {worst_sample:?}");
	assert!(worst_error <= maximum_error, "{name} triangle-interior deviation {worst_error} exceeds {maximum_error}: {worst_sample:?}");
}

fn mesh_and_topology(name: &str, solid: &Solid, options: Tessellation) -> (MeshChunks, TopologySnapshot) {
	let validation = solid.validate().unwrap_or_else(|error| panic!("{name}: validate exact fixture: {error:?}"));
	assert!(validation.valid, "{name}: adversarial fixture is not a valid exact solid: {validation:?}");
	let topology = solid.topology_snapshot_with_options(TopologyQueryOptions::SEMANTIC_IDENTITY).unwrap_or_else(|error| panic!("{name}: snapshot exact topology: {error:?}"));
	let chunks = Solid::mesh_chunks([solid], options).unwrap_or_else(|error| panic!("{name}: tessellate valid exact fixture: {error:?}"));
	(chunks, topology)
}

fn polygon(points: &[DVec3]) -> Vec<Edge> {
	Edge::polygon(points).expect("construct polygon wire")
}

fn nested_island_prisms() -> (Solid, Solid) {
	let outer = polygon(&[DVec3::new(-12.0, -12.0, 0.0), DVec3::new(12.0, -12.0, 0.0), DVec3::new(12.0, 12.0, 0.0), DVec3::new(-12.0, 12.0, 0.0)]);
	let hole = polygon(&[DVec3::new(-8.0, -8.0, 0.0), DVec3::new(-8.0, 8.0, 0.0), DVec3::new(8.0, 8.0, 0.0), DVec3::new(8.0, -8.0, 0.0)]);
	let island = polygon(&[DVec3::new(-3.0, -3.0, 0.0), DVec3::new(3.0, -3.0, 0.0), DVec3::new(3.0, 3.0, 0.0), DVec3::new(-3.0, 3.0, 0.0)]);
	let outer_prism = Solid::extrude(&outer, DVec3::Z * 4.0).expect("extrude parity fixture outer boundary");
	let hole_prism = Solid::extrude(&hole, DVec3::Z * 4.0).expect("extrude parity fixture hole");
	let ring: Solid = (&outer_prism - &hole_prism).build().expect("subtract parity fixture hole");
	let island = Solid::extrude(&island, DVec3::Z * 4.0).expect("extrude parity fixture island");
	(ring, island)
}

fn periodic_loft_section(z: f64, scale: f64, x_offset: f64, twist: f64) -> Edge {
	let point_count = 16;
	let points = (0..point_count)
		.map(|index| {
			let fraction = index as f64 / point_count as f64;
			let angle = TAU * fraction.powf(1.35) + twist;
			let radius = scale * (8.0 + 1.4 * (3.0 * angle).cos() + 0.7 * (5.0 * angle).sin());
			DVec3::new(x_offset + radius * angle.cos(), radius * 0.62 * angle.sin(), z)
		})
		.collect::<Vec<_>>();
	Edge::bspline(&points, BSplineEnd::Periodic).expect("construct nonuniform periodic B-spline section")
}

#[test]
fn cone_apex_is_finite_watertight_and_has_no_hard_slivers() {
	let cone = Solid::cone(8.0, 0.0, DVec3::Z * 20.0);
	let (chunks, topology) = mesh_and_topology("cone apex", &cone, absolute_options(0.05));
	let quality = mesh_quality("cone apex", &chunks);

	assert_global_shell_identity("cone apex", &chunks, &topology);
	assert_hard_quality("cone apex", quality, 40.0, 0.75);
}

#[test]
fn periodic_cylinder_trim_crossing_seam_preserves_exact_occurrence_multiplicity() {
	let cylinder = Solid::cylinder(8.0, DVec3::Z * 20.0);
	// OCCT's canonical cylinder seam is at +X. The notch crosses that seam
	// without removing the full cylindrical side, forcing two trim branches to
	// meet the surviving periodic seam.
	let notch = Solid::cube(DVec3::new(6.0, -2.5, 5.0), DVec3::new(10.0, 2.5, 15.0));
	let trimmed: Solid = (&cylinder - &notch).build().expect("cut a seam-crossing cylindrical notch");
	let validation = trimmed.validate().expect("validate seam-crossing cylinder trim");
	assert!(validation.valid, "seam-crossing cylinder trim is not an exact-valid fixture: {validation:?}");
	let topology = trimmed.topology_snapshot_with_options(TopologyQueryOptions::SEMANTIC_IDENTITY).expect("snapshot seam-crossing cylinder topology");
	let seam_edges = topology.edge_ids().iter().enumerate().filter(|(edge, _)| topology.edge_facts(*edge as u32).is_some_and(|facts| facts.seam)).map(|(edge, _)| edge).collect::<Vec<_>>();
	assert!(!seam_edges.is_empty(), "fixture did not retain a periodic seam");
	eprintln!("seam-crossing cylinder trim: exact-valid fixture contains periodic seam edges {seam_edges:?}");
	let chunks = Solid::mesh_chunks([&trimmed], absolute_options(0.04)).expect("seam-crossing cylinder trim: tessellate exact-valid periodic fixture");
	let quality = mesh_quality("seam-crossing cylinder trim", &chunks);

	assert_global_shell_identity("seam-crossing cylinder trim", &chunks, &topology);
	assert_hard_quality("seam-crossing cylinder trim", quality, 50.0, 0.5);
}

#[test]
fn nested_trim_loop_parity_preserves_the_island_and_hole() {
	let (ring, island) = nested_island_prisms();
	let expected_volume = (24.0 * 24.0 - 16.0 * 16.0 + 6.0 * 6.0) * 4.0;
	let actual_volume = ring.volume() + island.volume();
	assert!((actual_volume - expected_volume).abs() <= expected_volume * 1.0e-9, "nested trim parity produced volume {actual_volume}, expected {expected_volume}");
	for (name, solid) in [("outer-minus-hole ring", &ring), ("nested island", &island)] {
		let validation = solid.validate().unwrap_or_else(|error| panic!("{name}: validate exact fixture: {error:?}"));
		assert!(validation.valid, "{name}: adversarial fixture is not a valid exact solid: {validation:?}");
	}
	let chunks = Solid::mesh_chunks([&ring, &island], absolute_options(0.04)).expect("tessellate the valid nested ring-and-island compound");
	let quality = mesh_quality("outer/hole/island prism", &chunks);

	assert_global_mesh_manifold("outer/hole/island compound", &chunks);
	assert_hard_quality("outer/hole/island prism", quality, 25.0, 1.0);
}

#[test]
fn relative_tessellation_of_curved_sweeps_is_scale_equivariant() {
	// Establish a nominal-scale reference before exercising both extremes, so a
	// failure identifies scale handling rather than only reporting the first
	// element of an ascending input list.
	let scales = [1.0, 1.0e-3, 1.0e3];
	let mut reference_counts = None;
	for scale in scales {
		let profile = Edge::ellipse(3.0 * scale, 1.0 * scale, DVec3::X, DVec3::Z).expect("construct elliptical sweep profile");
		let spine = Edge::line(DVec3::ZERO, DVec3::Z * (12.0 * scale)).expect("construct sweep spine");
		let sweep = Solid::sweep([&profile], [&spine], ProfileOrient::Fixed).expect("construct curved-profile sweep");
		let name = format!("relative elliptical sweep at scale {scale:e}");
		let (chunks, topology) = mesh_and_topology(&name, &sweep, relative_options());
		let quality = mesh_quality(&name, &chunks);
		assert_global_shell_identity(&name, &chunks, &topology);
		assert_hard_quality(&name, quality, 35.0, 0.75);
		let counts = (chunks.faces.len(), chunks.edges.len(), chunks.faces.iter().map(|face| face.vertices.len()).sum::<usize>(), chunks.faces.iter().map(|face| face.indices.len() / 3).sum::<usize>());
		if let Some(reference) = reference_counts {
			assert_eq!(counts, reference, "relative tessellation changed discrete topology across uniform scale at {scale:e}");
		} else {
			reference_counts = Some(counts);
		}
	}
}

#[test]
fn nonuniform_high_curvature_bspline_loft_meets_interior_error_and_hard_quality_bounds() {
	let sections = [periodic_loft_section(0.0, 1.0, 0.0, 0.0), periodic_loft_section(8.0, 0.68, 1.5, 0.42), periodic_loft_section(21.0, 0.52, 2.0, 1.3)];
	let loft = Solid::loft(sections.iter().map(std::iter::once), false).expect("loft high-curvature periodic B-spline sections");
	let linear_tolerance = 0.05;
	let (chunks, topology) = mesh_and_topology("nonuniform high-curvature B-spline loft", &loft, absolute_options(linear_tolerance));
	let quality = mesh_quality("nonuniform high-curvature B-spline loft", &chunks);

	assert_exact_interior_error("nonuniform high-curvature B-spline loft", &loft, &chunks, linear_tolerance * 1.01);
	assert_global_shell_identity("nonuniform high-curvature B-spline loft", &chunks, &topology);
	assert_hard_quality("nonuniform high-curvature B-spline loft", quality, 30.0, 0.75);
}
