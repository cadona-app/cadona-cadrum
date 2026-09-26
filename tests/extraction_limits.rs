use std::collections::BTreeMap;

use cadrum::{DVec3, Error, FailureCategory, Solid, Tessellation};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PositionKey([i64; 3]);

impl PositionKey {
	fn new(point: DVec3) -> Self {
		Self(point.to_array().map(|component| (component * 100_000_000.0).round() as i64))
	}
}

#[test]
fn relative_tessellation_is_scale_covariant_below_unit_size() {
	fn mesh_signature(scale: f64) -> (usize, usize) {
		let cylinder = Solid::cylinder(scale, DVec3::Z * (scale * 2.0));
		let mesh = Solid::mesh([&cylinder], Tessellation { deflection_linear: 0.02, deflection_angular: 0.2, relative_linear: true, include_edges: true, parallel: false }).expect("relative cylinder tessellation");
		(mesh.vertices.len(), mesh.indices.len())
	}

	let reference = mesh_signature(1.0);
	for scale in [0.01, 0.1, 10.0, 100.0] {
		let actual = mesh_signature(scale);
		// Adaptive triangulation may choose different diagonals at floating-point ties.
		// A unit-size deflection floor instead causes orders-of-magnitude density changes.
		for (actual, reference) in [(actual.0, reference.0), (actual.1, reference.1)] {
			assert!(actual.abs_diff(reference) <= reference / 100, "relative tessellation density changed at scale {scale}: {actual} versus {reference}");
		}
	}
}

#[test]
fn excessive_face_selection_reports_a_resource_limit() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let face_indices = (0..=65_536_u32).collect::<Vec<_>>();
	let error = cube.mesh_face_chunks(&face_indices, Tessellation::default()).expect_err("selection beyond the extraction face quota must fail");

	let Error::OperationFailed(failure) = error else {
		panic!("expected structured extraction failure");
	};
	assert_eq!(failure.category, FailureCategory::ResourceLimit);
	assert_eq!(failure.stage, "resource_limit");
	assert!(failure.message.contains("face-selection quota"));
}

#[test]
fn tolerance_controlled_extraction_repair_does_not_mutate_the_source_brep() {
	let cylinder = Solid::cylinder(3.0, DVec3::Z * 8.0);
	let mut before = Vec::new();
	Solid::write_brep([&cylinder], &mut before).expect("archive source before tessellation");

	Solid::mesh([&cylinder], Tessellation { deflection_linear: 0.1, relative_linear: false, parallel: false, ..Tessellation::default() }).expect("tessellate a detached extraction copy");

	let mut after = Vec::new();
	Solid::write_brep([&cylinder], &mut after).expect("archive source after tessellation");
	assert_eq!(after, before, "presentation extraction modified the authoritative exact B-rep");
}

#[test]
fn copied_box_face_occurrences_preserve_shell_orientation() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let chunks = Solid::mesh_chunks([&cube], Tessellation { deflection_linear: 0.1, relative_linear: false, parallel: false, ..Tessellation::default() }).expect("tessellate exact box");
	let mut edge_uses = BTreeMap::<(PositionKey, PositionKey), (usize, i32)>::new();

	for face in chunks.faces {
		for triangle in face.indices.chunks_exact(3) {
			for [first, second] in [[triangle[0], triangle[1]], [triangle[1], triangle[2]], [triangle[2], triangle[0]]] {
				let first = PositionKey::new(face.vertices[first as usize]);
				let second = PositionKey::new(face.vertices[second as usize]);
				assert_ne!(first, second, "box tessellation contains a collapsed triangle edge");
				let (key, direction) = if first < second { ((first, second), 1) } else { ((second, first), -1) };
				let use_count = edge_uses.entry(key).or_default();
				use_count.0 += 1;
				use_count.1 += direction;
			}
		}
	}

	let invalid = edge_uses.into_iter().filter(|(_, (count, direction_balance))| *count != 2 || *direction_balance != 0).take(8).collect::<Vec<_>>();
	assert!(invalid.is_empty(), "copied box shell has cracks or same-direction face boundaries: {invalid:?}");
}
