use std::collections::{BTreeMap, BTreeSet};

use cadrum::{CancellationToken, DQuat, DVec3, Edge, MeshChunks, ProfileOrient, Solid, Tessellation};

const POSITION_SCALE: f64 = 100_000_000.0;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PositionKey([i64; 3]);

impl PositionKey {
	fn new(position: DVec3) -> Self {
		Self(position.to_array().map(|component| (component * POSITION_SCALE).round() as i64))
	}
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct EdgeKey {
	first: PositionKey,
	second: PositionKey,
}

impl EdgeKey {
	fn new(first: DVec3, second: DVec3) -> (Self, i32) {
		let first = PositionKey::new(first);
		let second = PositionKey::new(second);
		if first < second {
			(Self { first, second }, 1)
		} else {
			(Self { first: second, second: first }, -1)
		}
	}
}

fn presentation_options(parallel: bool) -> Tessellation {
	Tessellation { deflection_linear: 0.004, deflection_angular: 0.2, relative_linear: true, include_edges: true, parallel }
}

fn mesh(solid: &Solid) -> MeshChunks {
	Solid::mesh_chunks([solid], presentation_options(false)).expect("tessellate exact body")
}

fn assert_closed_well_shaped(name: &str, chunks: &MeshChunks, maximum_p95_aspect: f64) {
	let mut aspects = assert_closed_mesh(name, chunks);
	aspects.sort_by(f64::total_cmp);
	let p95_aspect = aspects[(aspects.len() - 1) * 95 / 100];
	assert!(p95_aspect <= maximum_p95_aspect, "{name} has widespread sliver triangles: p95 aspect {p95_aspect}, limit {maximum_p95_aspect}");
}

fn assert_closed_mesh(name: &str, chunks: &MeshChunks) -> Vec<f64> {
	assert!(!chunks.faces.is_empty(), "{name} has no tessellated faces");
	let mut face_indices = BTreeSet::new();
	let mut edge_uses = BTreeMap::<EdgeKey, (usize, i32)>::new();
	let mut aspects = Vec::new();

	for face in &chunks.faces {
		assert!(face_indices.insert(face.face_index), "{name} repeats face {}", face.face_index);
		assert_eq!(face.vertices.len(), face.normals.len());
		assert!(!face.indices.is_empty(), "{name} face {} is empty", face.face_index);
		assert!(face.indices.len().is_multiple_of(3));
		assert!(face.vertices.iter().all(|position| position.is_finite()));
		assert!(face.normals.iter().all(|normal| normal.is_finite() && (normal.length() - 1.0).abs() <= 1.0e-6));

		for (triangle_index, triangle) in face.indices.chunks_exact(3).enumerate() {
			assert!(triangle.iter().all(|index| (*index as usize) < face.vertices.len()), "{name} face {} triangle {triangle_index} has an invalid index", face.face_index);
			let positions = [triangle[0], triangle[1], triangle[2]].map(|index| face.vertices[index as usize]);
			let first_edge = positions[1] - positions[0];
			let second_edge = positions[2] - positions[0];
			let area_normal = first_edge.cross(second_edge);
			let double_area = area_normal.length();
			let longest_edge = [first_edge.length(), (positions[2] - positions[1]).length(), second_edge.length()].into_iter().fold(0.0_f64, f64::max);
			assert!(longest_edge > f64::EPSILON && double_area > longest_edge.powi(2) * 1.0e-12, "{name} face {} triangle {triangle_index} is degenerate: {positions:?}", face.face_index);

			let average_normal = [triangle[0], triangle[1], triangle[2]].into_iter().map(|index| face.normals[index as usize]).sum::<DVec3>().normalize();
			assert!(area_normal.normalize().dot(average_normal) > 0.0, "{name} face {} triangle {triangle_index} is wound against its surface normal", face.face_index);
			aspects.push(longest_edge.powi(2) / double_area);

			for [first, second] in [[triangle[0], triangle[1]], [triangle[1], triangle[2]], [triangle[2], triangle[0]]] {
				let (key, orientation) = EdgeKey::new(face.vertices[first as usize], face.vertices[second as usize]);
				assert_ne!(key.first, key.second, "{name} face {} triangle {triangle_index} has a collapsed edge", face.face_index);
				let uses = edge_uses.entry(key).or_default();
				uses.0 += 1;
				uses.1 += orientation;
			}
		}
	}

	let invalid_edges = edge_uses.iter().filter(|(_, (count, orientation_balance))| *count != 2 || *orientation_balance != 0).take(8).collect::<Vec<_>>();
	assert!(invalid_edges.is_empty(), "{name} is cracked, non-manifold, or inconsistently wound: {invalid_edges:?}");
	for edge in &chunks.edges {
		assert!(edge.points.len() >= 2, "{name} semantic edge {} has no polyline", edge.edge_index);
		for segment in edge.points.windows(2) {
			let (key, _) = EdgeKey::new(segment[0], segment[1]);
			// OCCT may retain a zero-length seam at a periodic surface pole. It has
			// no corresponding surface segment to match, but all nonzero semantic
			// edge segments must use the exact shared surface-boundary vertices.
			if key.first == key.second {
				continue;
			}
			assert!(edge_uses.contains_key(&key), "{name} semantic edge {} does not share the surface mesh boundary: {segment:?}", edge.edge_index);
		}
	}

	aspects
}

fn multiply_trimmed_prism() -> Solid {
	let outer = Edge::polygon(&[DVec3::new(-12.0, -8.0, 0.0), DVec3::new(12.0, -8.0, 0.0), DVec3::new(12.0, 8.0, 0.0), DVec3::new(-12.0, 8.0, 0.0)]).expect("outer profile");
	// Inner wires deliberately use the opposite winding from the outer wire.
	// Two disjoint holes exercise multiple nested trim-loop classification on
	// both planar caps without relying on a particular curve subdivision count.
	let first_hole = Edge::polygon(&[DVec3::new(-7.0, -2.0, 0.0), DVec3::new(-7.0, 2.0, 0.0), DVec3::new(-3.0, 2.0, 0.0), DVec3::new(-3.0, -2.0, 0.0)]).expect("first hole");
	let second_hole = Edge::polygon(&[DVec3::new(3.0, -2.0, 0.0), DVec3::new(3.0, 2.0, 0.0), DVec3::new(7.0, 2.0, 0.0), DVec3::new(7.0, -2.0, 0.0)]).expect("second hole");
	Solid::extrude_wires_cancelable([outer.iter(), first_hole.iter(), second_hole.iter()], DVec3::Z * 6.0, &CancellationToken::new()).expect("extrude profile with two inner trim loops")
}

fn top_cylinder_ring(cylinder: &Solid) -> &Edge {
	cylinder
		.iter_edge()
		.find(|edge| {
			let points = edge.approximation_segments(Tessellation::default());
			points.len() >= 2 && points.iter().all(|point| (point.z - 20.0).abs() < 1.0e-8)
		})
		.expect("top cylinder ring")
}

#[test]
fn thin_airfoil_rib_preserves_closed_boundaries_and_regular_interior_quality() {
	let archive = include_bytes!("fixtures/wing_section_rib.brep");
	let solids = Solid::read_brep(&mut archive.as_slice()).expect("read exact airfoil rib");
	assert_eq!(solids.len(), 1);
	for (name, deflection_linear, deflection_angular, relative_linear) in [("preview rib", 0.05, 0.5, false), ("standard rib", 0.004, 0.25, true)] {
		let options = Tessellation { deflection_linear, deflection_angular, relative_linear, include_edges: true, parallel: false };
		let chunks = Solid::mesh_chunks([&solids[0]], options).expect("thin rib tessellates within the unchanged audits");
		assert_eq!(chunks.faces.len(), 4);
		assert_closed_mesh(name, &chunks);
		// Exact shared edges can force thin transition cells on a narrow rib.
		// Apply the ordinary aspect limits to the regular interior; audit every triangle's topology.
		let boundary = chunks.edges.iter().flat_map(|edge| edge.points.iter().copied().map(PositionKey::new)).collect::<BTreeSet<_>>();
		let mut interior_aspects = Vec::new();
		for face in &chunks.faces {
			for triangle in face.indices.chunks_exact(3) {
				let points = [triangle[0], triangle[1], triangle[2]].map(|index| face.vertices[index as usize]);
				if points.iter().any(|point| boundary.contains(&PositionKey::new(*point))) {
					continue;
				}
				let edges = [points[1] - points[0], points[2] - points[1], points[0] - points[2]];
				let longest_squared = edges.iter().map(|edge| edge.length_squared()).fold(0.0, f64::max);
				interior_aspects.push(longest_squared / edges[0].cross(edges[1]).length());
			}
		}
		assert!(interior_aspects.len() >= 100, "{name} must exercise a regular interior");
		interior_aspects.sort_by(f64::total_cmp);
		let p95 = interior_aspects[(interior_aspects.len() - 1) * 95 / 100];
		assert!(p95 <= 30.0, "{name} has poor regular interior quality: p95 aspect {p95}");
		let worst = interior_aspects.last().copied().unwrap();
		assert!(worst <= 60.0, "{name} retains an interior needle: aspect {worst}");
	}
}

#[test]
fn fillet_and_chamfer_tessellations_are_closed_and_well_shaped() {
	let fillet_source = Solid::cylinder(10.0, DVec3::Z * 20.0);
	let chamfer_source = Solid::cylinder(10.0, DVec3::Z * 20.0);
	let fillet = fillet_source.fillet_edges(2.0, [top_cylinder_ring(&fillet_source)]).expect("fillet cylinder ring");
	let chamfer = chamfer_source.chamfer_edges(2.0, [top_cylinder_ring(&chamfer_source)]).expect("chamfer cylinder ring");

	assert_closed_well_shaped("fillet", &mesh(&fillet), 30.0);
	assert_closed_well_shaped("chamfer", &mesh(&chamfer), 20.0);
}

#[test]
fn sealed_shell_and_offset_tessellations_preserve_closed_surfaces() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(20.0));
	let shell = cube.shell(-1.0, std::iter::empty::<&cadrum::Face>()).expect("sealed inward shell");
	let offset = Solid::sphere(8.0).offset_surface(1.0, 1.0e-6).expect("outward sphere offset");

	assert_closed_well_shaped("sealed shell", &mesh(&shell), 20.0);
	assert_closed_well_shaped("offset sphere", &mesh(&offset), 30.0);
}

#[test]
fn tight_sphere_angular_tolerance_refines_the_complete_periodic_chart() {
	let sphere = Solid::sphere(30.0);
	let options = Tessellation { deflection_linear: 0.01, deflection_angular: 0.02, relative_linear: true, include_edges: true, parallel: false };
	let chunks = Solid::mesh_chunks([&sphere], options).expect("tight-angle sphere tessellates");

	assert_closed_well_shaped("tight-angle sphere", &chunks, 12.0);
	let triangle_count = chunks.faces.iter().map(|face| face.indices.len() / 3).sum::<usize>();
	assert!(triangle_count <= 300_000, "tight-angle sphere exceeded its bounded structured mesh budget: {triangle_count} triangles");
}

#[test]
fn multiple_inner_trim_loops_remain_open_in_caps_and_watertight() {
	let prism = multiply_trimmed_prism();
	let expected_volume = (24.0 * 16.0 - 2.0 * 4.0 * 4.0) * 6.0;

	assert!((prism.volume() - expected_volume).abs() <= expected_volume * 1.0e-8);
	assert_closed_well_shaped("multiply trimmed prism", &mesh(&prism), 30.0);
}

#[test]
fn sweep_tessellation_is_rigid_placement_equivariant() {
	let profile = Edge::polygon(&[DVec3::new(-2.0, -1.0, 0.0), DVec3::new(2.0, -1.0, 0.0), DVec3::new(2.0, 1.0, 0.0), DVec3::new(-2.0, 1.0, 0.0)]).expect("sweep profile");
	let spine = Edge::line(DVec3::ZERO, DVec3::Z * 18.0).expect("sweep spine");
	let sweep = Solid::sweep(&profile, [&spine], ProfileOrient::Fixed).expect("sweep profile");
	let axis = DVec3::new(1.0, -2.0, 0.5).normalize();
	let angle = 0.73;
	let translation = DVec3::new(17.0, -9.0, 6.0);
	let rotation = DQuat::from_axis_angle(axis, angle);
	let placed = sweep.located(axis, angle, translation);
	let source_mesh = mesh(&sweep);
	let placed_mesh = mesh(&placed);

	assert_closed_well_shaped("sweep", &source_mesh, 20.0);
	assert_closed_well_shaped("placed sweep", &placed_mesh, 20.0);
	assert_eq!(placed_mesh.faces.len(), source_mesh.faces.len());
	for (source, placed) in source_mesh.faces.iter().zip(&placed_mesh.faces) {
		assert_eq!(placed.face_index, source.face_index);
		assert_eq!(placed.indices.len(), source.indices.len());
		assert_eq!(placed.vertices.len(), source.vertices.len());
		let mut expected_vertices = source.vertices.iter().map(|position| PositionKey::new(rotation * *position + translation)).collect::<Vec<_>>();
		let mut actual_vertices = placed.vertices.iter().copied().map(PositionKey::new).collect::<Vec<_>>();
		expected_vertices.sort_unstable();
		actual_vertices.sort_unstable();
		assert_eq!(actual_vertices, expected_vertices, "face {} changed its sampled surface vertices under rigid placement", source.face_index);

		let mut expected_normals = source.normals.iter().map(|normal| PositionKey::new(rotation * *normal)).collect::<Vec<_>>();
		let mut actual_normals = placed.normals.iter().copied().map(PositionKey::new).collect::<Vec<_>>();
		expected_normals.sort_unstable();
		actual_normals.sort_unstable();
		assert_eq!(actual_normals, expected_normals, "face {} changed its exact surface normals under rigid placement", source.face_index);
	}
}

#[test]
fn serial_and_parallel_tessellations_are_byte_identical() {
	let trimmed = multiply_trimmed_prism();
	let fillet_source = Solid::cylinder(10.0, DVec3::Z * 20.0).translate(DVec3::X * 40.0);
	let fillet = fillet_source.fillet_edges(2.0, [top_cylinder_ring(&fillet_source)]).expect("fillet fixture");

	let serial = Solid::mesh_chunks([&trimmed, &fillet], presentation_options(false)).expect("serial tessellation");
	let parallel = Solid::mesh_chunks([&trimmed, &fillet], presentation_options(true)).expect("parallel tessellation");

	assert_eq!(parallel, serial, "parallel face scheduling must not affect mesh bytes or ordering");
}

#[test]
fn periodic_planar_annulus_keeps_both_sides_of_its_seam() {
	let profile = Edge::polygon(&[DVec3::new(2.0, -1.0, 0.0), DVec3::new(4.0, -1.0, 0.0), DVec3::new(4.0, 1.0, 0.0), DVec3::new(2.0, 1.0, 0.0)]).expect("annular section");
	let spine = Edge::circle(1.0, DVec3::Y).expect("full revolution");
	let solid = Solid::sweep(&profile, [&spine], ProfileOrient::Up(DVec3::Y)).expect("annular solid");
	assert!((solid.volume() - 24.0 * std::f64::consts::PI).abs() < 1.0e-6);
	let serial = Solid::mesh_chunks([&solid], presentation_options(false)).expect("serial annulus mesh");
	assert_closed_well_shaped("periodic planar annulus", &serial, 30.0);
	let parallel = Solid::mesh_chunks([&solid], presentation_options(true)).expect("parallel annulus mesh");
	assert_eq!(serial.faces.len(), parallel.faces.len());
	for (left, right) in serial.faces.iter().zip(&parallel.faces) {
		assert_eq!(left.vertices, right.vertices);
		assert_eq!(left.indices, right.indices);
	}
}

#[test]
fn sparse_straight_slot_boundaries_survive_boolean_and_planar_meshing() {
	let at = |center: f64, angle: f64| DVec3::new(center + 5.0 * angle.cos(), 5.0 * angle.sin(), 0.0);
	let pi = std::f64::consts::PI;
	let profile = [Edge::line(DVec3::new(0.0, -5.0, 0.0), DVec3::new(20.0, -5.0, 0.0)).unwrap(), Edge::arc_3pts(at(20.0, -pi / 2.0), at(20.0, 0.0), at(20.0, pi / 2.0)).unwrap(), Edge::line(DVec3::new(20.0, 5.0, 0.0), DVec3::new(0.0, 5.0, 0.0)).unwrap(), Edge::arc_3pts(at(0.0, pi / 2.0), at(0.0, pi), at(0.0, 1.5 * pi)).unwrap()];
	let cancellation = CancellationToken::new();
	let slot = Solid::extrude_cancelable(&profile, DVec3::Z * 3.0, &cancellation).unwrap();
	let cutter = Solid::cylinder(1.0, DVec3::Z * 3.0).translate(DVec3::X * 10.0);
	let solids = Solid::boolean_build_regularized_cancelable(&(&slot - &cutter), &cancellation).unwrap();
	assert_eq!(solids.len(), 1);
	let solid = &solids[0];
	assert!((solid.volume() - (200.0 + 24.0 * pi) * 3.0).abs() < 1.0e-7);
	assert_eq!(solid.iter_face().count(), 7);
	let options = Tessellation::default();
	let serial = Solid::mesh_chunks([solid], options).expect("slot mesh with sparse straight boundaries");
	assert_closed_well_shaped("slot with drilled hole", &serial, 20.0);
	assert!(assert_closed_mesh("slot with drilled hole", &serial).into_iter().all(|aspect| aspect <= 56.0));
	let parallel = Solid::mesh_chunks([solid], Tessellation { parallel: true, ..options }).unwrap();
	assert_eq!(serial, parallel);
}
