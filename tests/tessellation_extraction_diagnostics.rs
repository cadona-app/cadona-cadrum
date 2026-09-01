use cadrum::{DVec3, Error, FailureCategory, Solid, Tessellation};

#[test]
fn silent_source_extraction_failures_report_the_active_stage() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let error = cube.mesh_face_chunks(&[u32::MAX], Tessellation::default()).expect_err("out-of-range face extraction must fail");
	let Error::OperationFailed(failure) = error else {
		panic!("source extraction should preserve its native failure context");
	};
	assert_eq!(failure.operation, "extract_brep_mesh_source");
	assert_eq!(failure.stage, "extract_face_surfaces_and_trims");
	assert_eq!(failure.category, FailureCategory::AlgorithmFailed);
	assert!(!failure.message.is_empty());
}

#[test]
fn cancelled_source_extraction_remains_cancellation() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let progress = cadrum::CancellationToken::new();
	progress.cancel();
	let error = cube.mesh_face_chunks_cancelable(&[0], Tessellation::default(), &progress).expect_err("cancelled extraction must stop");
	assert!(matches!(error, Error::Cancelled));
}
