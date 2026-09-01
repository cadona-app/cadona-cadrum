use std::{
	collections::{BTreeMap, BTreeSet},
	f64::consts::TAU,
};

use cadrum::{BSplineEnd, DVec3, Edge, MeshChunks, Solid, Tessellation};

#[derive(Clone, Copy, Debug, PartialEq)]
struct GradationMetrics {
	triangle_count: usize,
	adjacent_pair_count: usize,
	interior_vertex_count: usize,
	p95_adjacent_area_ratio: f64,
	p99_adjacent_area_ratio: f64,
	p95_adjacent_edge_ratio: f64,
	p99_adjacent_edge_ratio: f64,
	p95_interior_valence: usize,
	maximum_interior_valence: usize,
}

#[derive(Clone, Copy)]
struct TriangleMetrics {
	area: f64,
	longest_edge: f64,
}

fn periodic_section(z: f64, scale: f64, x_offset: f64, twist: f64) -> Edge {
	let points = (0..16)
		.map(|index| {
			let angle = TAU * index as f64 / 16.0 + twist;
			let radius = scale * (9.0 + 1.2 * (3.0 * angle).cos() + 0.55 * (5.0 * angle).sin());
			DVec3::new(x_offset + radius * angle.cos(), radius * 0.68 * angle.sin(), z)
		})
		.collect::<Vec<_>>();
	Edge::bspline(&points, BSplineEnd::Periodic).expect("construct periodic organic section")
}

fn organic_loft() -> Solid {
	let sections = [periodic_section(0.0, 1.0, 0.0, 0.0), periodic_section(11.0, 0.76, 1.8, 0.31), periodic_section(27.0, 0.51, 3.2, 0.82)];
	Solid::loft(sections.iter().map(std::iter::once), false).expect("construct representative organic loft")
}

fn options(parallel: bool) -> Tessellation {
	Tessellation { deflection_linear: 0.015, deflection_angular: 0.25, relative_linear: true, include_edges: false, parallel }
}

#[test]
fn organic_surface_refinement_is_schedule_deterministic_and_spatially_graded() {
	let loft = organic_loft();
	let serial = Solid::mesh_chunks([&loft], options(false)).expect("serial organic tessellation");
	let parallel = Solid::mesh_chunks([&loft], options(true)).expect("parallel organic tessellation");

	assert_eq!(parallel, serial, "parallel scheduling changed the representative organic mesh");
	let metrics = mesh_gradation(&serial);
	eprintln!("organic loft gradation: {metrics:?}");

	assert!(metrics.triangle_count <= 50_000, "organic loft exceeded its bounded presentation budget: {metrics:?}");
	assert!(metrics.adjacent_pair_count >= metrics.triangle_count / 2, "gradation audit did not cover enough adjacent triangles: {metrics:?}");
	assert!(metrics.interior_vertex_count > 0, "gradation audit found no interior vertices: {metrics:?}");
	assert!(metrics.p95_adjacent_area_ratio <= 2.5, "organic refinement changes triangle area too abruptly at p95: {metrics:?}");
	assert!(metrics.p99_adjacent_area_ratio <= 4.5, "organic refinement changes triangle area too abruptly at p99: {metrics:?}");
	assert!(metrics.p95_adjacent_edge_ratio <= 1.75, "organic refinement changes edge scale too abruptly at p95: {metrics:?}");
	assert!(metrics.p99_adjacent_edge_ratio <= 2.5, "organic refinement changes edge scale too abruptly at p99: {metrics:?}");
	assert!(metrics.p95_interior_valence <= 8, "organic refinement has widespread high-valence fans: {metrics:?}");
	assert!(metrics.maximum_interior_valence <= 16, "organic refinement contains an extreme high-valence fan: {metrics:?}");
}

fn mesh_gradation(chunks: &MeshChunks) -> GradationMetrics {
	let mut triangle_count = 0;
	let mut adjacent_area_ratios = Vec::new();
	let mut adjacent_edge_ratios = Vec::new();
	let mut interior_valences = Vec::new();

	for face in &chunks.faces {
		let mut triangles = Vec::with_capacity(face.indices.len() / 3);
		let mut edge_uses = BTreeMap::<(u32, u32), Vec<usize>>::new();
		let mut neighbors = vec![BTreeSet::<u32>::new(); face.vertices.len()];
		for triangle in face.indices.chunks_exact(3) {
			let positions = [triangle[0], triangle[1], triangle[2]].map(|index| face.vertices[index as usize]);
			let edges = [positions[1] - positions[0], positions[2] - positions[1], positions[0] - positions[2]];
			let double_area = edges[0].cross(-edges[2]).length();
			let longest_edge = edges.into_iter().map(|edge| edge.length()).fold(0.0_f64, f64::max);
			assert!(double_area.is_finite() && double_area > longest_edge * longest_edge * 1.0e-12, "face {} contains a degenerate triangle: {positions:?}", face.face_index);
			let triangle_index = triangles.len();
			triangles.push(TriangleMetrics { area: double_area * 0.5, longest_edge });
			for [first, second] in [[triangle[0], triangle[1]], [triangle[1], triangle[2]], [triangle[2], triangle[0]]] {
				let edge = if first <= second { (first, second) } else { (second, first) };
				edge_uses.entry(edge).or_default().push(triangle_index);
				neighbors[first as usize].insert(second);
				neighbors[second as usize].insert(first);
			}
		}
		triangle_count += triangles.len();

		let mut boundary_vertices = BTreeSet::new();
		for ((first, second), uses) in edge_uses {
			match uses.as_slice() {
				[first_triangle, second_triangle] => {
					let first = triangles[*first_triangle];
					let second = triangles[*second_triangle];
					adjacent_area_ratios.push(ratio(first.area, second.area));
					adjacent_edge_ratios.push(ratio(first.longest_edge, second.longest_edge));
				}
				[_] => {
					boundary_vertices.insert(first);
					boundary_vertices.insert(second);
				}
				_ => panic!("face {} contains a non-manifold local triangle edge ({first}, {second}) with {} uses", face.face_index, uses.len()),
			}
		}
		interior_valences.extend(neighbors.into_iter().enumerate().filter(|(vertex, neighbors)| !neighbors.is_empty() && !boundary_vertices.contains(&(*vertex as u32))).map(|(_, neighbors)| neighbors.len()));
	}

	adjacent_area_ratios.sort_by(f64::total_cmp);
	adjacent_edge_ratios.sort_by(f64::total_cmp);
	interior_valences.sort_unstable();
	assert!(!adjacent_area_ratios.is_empty());
	assert!(!adjacent_edge_ratios.is_empty());
	assert!(!interior_valences.is_empty());

	GradationMetrics {
		triangle_count,
		adjacent_pair_count: adjacent_area_ratios.len(),
		interior_vertex_count: interior_valences.len(),
		p95_adjacent_area_ratio: percentile(&adjacent_area_ratios, 95),
		p99_adjacent_area_ratio: percentile(&adjacent_area_ratios, 99),
		p95_adjacent_edge_ratio: percentile(&adjacent_edge_ratios, 95),
		p99_adjacent_edge_ratio: percentile(&adjacent_edge_ratios, 99),
		p95_interior_valence: percentile(&interior_valences, 95),
		maximum_interior_valence: *interior_valences.last().expect("checked above"),
	}
}

fn ratio(first: f64, second: f64) -> f64 {
	first.max(second) / first.min(second)
}

fn percentile<T: Copy>(sorted: &[T], percentile: usize) -> T {
	assert!(!sorted.is_empty());
	sorted[(sorted.len() - 1) * percentile / 100]
}
