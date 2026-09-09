#![cfg(feature = "test-support")]

use cadrum::{occt::test_support::tessellation_face_cache_statistics, DVec3, Solid, Tessellation};

#[test]
fn exact_face_cache_reuses_detached_geometry_and_rejects_changed_inputs() {
	let options = Tessellation { deflection_linear: 0.05, deflection_angular: 0.5, relative_linear: false, include_edges: true, parallel: false };
	tessellation_face_cache_statistics(true);
	let source = Solid::cube(DVec3::ZERO, DVec3::splat(5.0));
	let first = Solid::mesh_chunks([&source], options).unwrap();
	assert_eq!(tessellation_face_cache_statistics(false)[1], 6);
	let copy = source.presentation_copy().unwrap();
	let cached = Solid::mesh_chunks([&copy], Tessellation { parallel: true, ..options }).unwrap();
	let counts = tessellation_face_cache_statistics(false);
	assert_eq!(&counts[..2], &[6, 6]);
	for (before, after) in first.faces.iter().zip(&cached.faces) {
		assert_eq!(before.vertices, after.vertices);
		assert_eq!(before.normals, after.normals);
		assert_eq!(before.indices, after.indices);
		assert_eq!(before.face_index, after.face_index);
	}
	assert_eq!(first.edges, cached.edges);
	let shifted = source.shared_copy().translate(DVec3::new(7.0, 0.0, 0.0));
	let shifted_mesh = Solid::mesh_chunks([&shifted], options).unwrap();
	assert_eq!(tessellation_face_cache_statistics(false)[1], 12);
	assert!(shifted_mesh.faces.iter().flat_map(|face| &face.vertices).all(|point| point.x >= 7.0 - 1.0e-10));
	Solid::mesh_chunks([&source], Tessellation { deflection_linear: 0.02, ..options }).unwrap();
	assert_eq!(tessellation_face_cache_statistics(false)[1], 18);
	assert!(tessellation_face_cache_statistics(false)[3] <= 64 * 1024 * 1024);
	tessellation_face_cache_statistics(true);
	let rebuilt = Solid::mesh_chunks([&copy], Tessellation { parallel: true, ..options }).unwrap();
	assert_eq!(cached, rebuilt, "a warm cache cannot alter deterministic tessellation");
}
