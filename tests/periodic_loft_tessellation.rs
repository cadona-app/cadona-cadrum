use std::{collections::BTreeMap, f64::consts::TAU};

use cadrum::{BSplineEnd, DVec3, Edge, Solid, Tessellation};

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

fn periodic_section(z: f64, x_scale: f64, y_scale: f64, x_offset: f64) -> Edge {
	let points = (0..20)
		.map(|index| {
			let angle = TAU * f64::from(index) / 20.0;
			let radius = 10.0 + 1.4 * (angle * 3.0).cos() + 0.7 * (angle * 2.0).sin();
			DVec3::new(x_offset + x_scale * radius * angle.cos(), y_scale * radius * angle.sin(), z)
		})
		.collect::<Vec<_>>();
	Edge::bspline(&points, BSplineEnd::Periodic).expect("periodic B-spline section")
}

#[test]
fn two_section_periodic_bspline_loft_tessellates_watertight_and_well_shaped() {
	let sections = [periodic_section(0.0, 1.0, 1.0, 0.0), periodic_section(24.0, 0.72, 1.18, 3.5)];
	let loft = Solid::loft(sections.iter().map(std::iter::once), false).expect("two-section periodic B-spline loft");
	let mesh = Solid::mesh_chunks([&loft], Tessellation { deflection_linear: 0.004, deflection_angular: 0.25, relative_linear: true, include_edges: true, parallel: true }).expect("tessellate periodic loft");

	assert!(!mesh.faces.is_empty(), "periodic loft has no tessellated faces");
	let mut edge_uses = BTreeMap::<EdgeKey, (usize, i32)>::new();
	let mut aspect_ratios = Vec::new();
	for face in &mesh.faces {
		assert_eq!(face.vertices.len(), face.normals.len());
		assert!(!face.indices.is_empty(), "periodic loft face {} is empty", face.face_index);
		assert!(face.indices.len().is_multiple_of(3));
		assert!(face.vertices.iter().all(|position| position.is_finite()));
		assert!(face.normals.iter().all(|normal| normal.is_finite()));

		for triangle in face.indices.chunks_exact(3) {
			assert!(triangle.iter().all(|index| (*index as usize) < face.vertices.len()));
			let positions = [face.vertices[triangle[0] as usize], face.vertices[triangle[1] as usize], face.vertices[triangle[2] as usize]];
			let edges = [positions[1] - positions[0], positions[2] - positions[1], positions[0] - positions[2]];
			let longest_edge = edges.iter().map(|edge| edge.length()).fold(0.0_f64, f64::max);
			let double_area = edges[0].cross(-edges[2]).length();
			assert!(double_area > longest_edge.powi(2) * 1.0e-12, "periodic loft contains a degenerate triangle: {positions:?}");
			aspect_ratios.push(longest_edge.powi(2) / double_area);

			for (first, second) in [(0, 1), (1, 2), (2, 0)] {
				let (key, orientation) = EdgeKey::new(positions[first], positions[second]);
				assert_ne!(key.first, key.second, "periodic loft contains a collapsed triangle edge");
				let uses = edge_uses.entry(key).or_default();
				uses.0 += 1;
				uses.1 += orientation;
			}
		}
	}

	let invalid_edges = edge_uses.iter().filter(|(_, (count, orientation_balance))| *count != 2 || *orientation_balance != 0).take(8).collect::<Vec<_>>();
	assert!(invalid_edges.is_empty(), "periodic loft is cracked, non-manifold, or inconsistently wound: {invalid_edges:?}");
	aspect_ratios.sort_by(f64::total_cmp);
	let p95_aspect = aspect_ratios[(aspect_ratios.len() - 1) * 95 / 100];
	assert!(p95_aspect <= 30.0, "periodic loft has widespread sliver triangles: p95 aspect {p95_aspect}");
}
