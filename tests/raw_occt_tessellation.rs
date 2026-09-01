#![cfg(feature = "test-support")]

use cadrum::{
	occt::test_support::{tessellation_occt_cache_stats, tessellation_seed_occt_cache},
	CancellationToken, DVec3, Error, MeshChunks, Solid, Tessellation,
};

fn options(include_edges: bool) -> Tessellation {
	Tessellation { deflection_linear: 0.08, deflection_angular: 0.2, relative_linear: false, include_edges, parallel: false }
}

fn assert_valid_chunks(chunks: &MeshChunks) {
	assert!(!chunks.faces.is_empty());
	for face in &chunks.faces {
		assert!(!face.vertices.is_empty());
		assert_eq!(face.vertices.len(), face.normals.len());
		assert!(!face.indices.is_empty());
		assert!(face.indices.len().is_multiple_of(3));
		assert!(face.vertices.iter().all(|point| point.is_finite()));
		assert!(face.normals.iter().all(|normal| normal.is_finite() && (normal.length() - 1.0).abs() < 1.0e-9));
		assert!(face.indices.iter().all(|index| (*index as usize) < face.vertices.len()));
	}
	for edge in &chunks.edges {
		assert!(edge.points.len() >= 2);
		assert!(edge.points.iter().all(|point| point.is_finite()));
	}
}

#[test]
fn raw_occt_chunks_are_valid_and_preserve_source_ordinals() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let face_count = cube.iter_face().count();
	let edge_count = cube.iter_edge().count();
	let chunks = Solid::mesh_chunks_raw_occt([&cube], options(true)).expect("mesh detached cube with raw OCCT");

	assert_valid_chunks(&chunks);
	assert_eq!(chunks.faces.iter().map(|face| face.face_index).collect::<Vec<_>>(), (0..face_count as u32).collect::<Vec<_>>());
	assert_eq!(chunks.edges.iter().map(|edge| edge.edge_index).collect::<Vec<_>>(), (0..edge_count as u32).collect::<Vec<_>>());
}

#[test]
fn custom_remains_default_and_raw_occt_is_an_explicit_engine_choice() {
	let cylinder = Solid::cylinder(4.0, DVec3::Z * 12.0);
	let options = options(true);
	let default_custom = Solid::mesh_chunks([&cylinder], options).expect("mesh with the default custom tessellator");
	let repeated_custom = Solid::mesh_chunks([&cylinder], options).expect("repeat the default custom tessellation");
	let raw_occt = Solid::mesh_chunks_raw_occt([&cylinder], options).expect("mesh with the explicit raw OCCT fallback");

	assert_eq!(default_custom, repeated_custom, "the default custom path must remain deterministic");
	assert_valid_chunks(&raw_occt);
	assert_eq!(default_custom.faces.iter().map(|face| face.face_index).collect::<Vec<_>>(), raw_occt.faces.iter().map(|face| face.face_index).collect::<Vec<_>>());
	assert_eq!(default_custom.edges.iter().map(|edge| edge.edge_index).collect::<Vec<_>>(), raw_occt.edges.iter().map(|edge| edge.edge_index).collect::<Vec<_>>());
	assert_ne!(default_custom, raw_occt, "the explicit raw fallback unexpectedly resolved to the default custom engine");
}

#[test]
fn raw_occt_meshes_a_detached_copy_without_mutating_source_geometry_or_cache() {
	let cylinder = Solid::cylinder(5.0, DVec3::Z * 14.0);
	let coarse = Tessellation { deflection_linear: 1.0, deflection_angular: 0.8, relative_linear: false, include_edges: true, parallel: false };
	tessellation_seed_occt_cache(&cylinder, coarse).expect("seed a coarse authoritative cache for the regression");
	let cache_before = tessellation_occt_cache_stats(&cylinder).expect("inspect seeded OCCT cache");
	assert_eq!(cache_before.face_count, cylinder.iter_face().count() as u32);
	assert!(cache_before.triangulated_face_count > 0);
	assert!(cache_before.node_count > 0);
	assert!(cache_before.triangle_count > 0);
	let mut archive_before = Vec::new();
	Solid::write_brep([&cylinder], &mut archive_before).expect("archive source before raw fallback");

	let fine = Tessellation { deflection_linear: 0.02, deflection_angular: 0.08, ..coarse };
	let chunks = Solid::mesh_chunks_raw_occt([&cylinder], fine).expect("mesh a detached copy at finer tolerances");
	assert_valid_chunks(&chunks);

	let cache_after = tessellation_occt_cache_stats(&cylinder).expect("inspect source cache after raw fallback");
	let mut archive_after = Vec::new();
	Solid::write_brep([&cylinder], &mut archive_after).expect("archive source after raw fallback");
	assert_eq!(cache_after, cache_before, "raw fallback changed the authoritative OCCT triangulation cache");
	assert_eq!(archive_after, archive_before, "raw fallback changed the authoritative B-rep");
}

#[test]
fn raw_occt_fallback_honors_pre_cancelled_requests() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let cancellation = CancellationToken::new();
	cancellation.cancel();
	assert!(matches!(Solid::mesh_chunks_raw_occt_cancelable([&cube], options(false), &cancellation), Err(Error::Cancelled)));
}
