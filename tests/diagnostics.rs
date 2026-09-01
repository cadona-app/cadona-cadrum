use cadrum::{DVec3, Error, FailureCategory, ResultTopology, Solid, Tessellation, TopologyKind};

#[test]
fn invalid_tessellation_parameters_are_rejected_before_source_extraction() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let error = Solid::mesh_chunks([&cube], Tessellation { deflection_linear: 0.0, ..Tessellation::default() }).expect_err("zero deflection must fail");

	assert!(matches!(&error, Error::InvalidInput(message) if message.contains("linear deflection")));
	assert_eq!(error.stage(), "validate_input");
	assert_eq!(error.category(), FailureCategory::InvalidInput);
	assert!(error.may_keep_last_valid_result());
}

#[test]
fn ordinary_failures_also_have_stable_recovery_categories() {
	assert_eq!(Error::Cancelled.category(), FailureCategory::Cancelled);
	assert_eq!(Error::ProjectionFailed("face").category(), FailureCategory::NoSolution);
	assert_eq!(Error::TopologyQueryFailed.category(), FailureCategory::InvalidResult);
	assert_eq!(Error::BrepReadFailed.category(), FailureCategory::Io);
}

#[test]
fn invalid_topology_distance_keeps_input_failure_context() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let error = cube.topology_distance(ResultTopology { kind: TopologyKind::Face, index: u32::MAX }, &cube, ResultTopology { kind: TopologyKind::Face, index: 0 }).expect_err("an invalid face ordinal must fail");
	let Error::OperationFailed(failure) = error else { panic!("expected structured input failure") };
	assert_eq!(failure.operation, "topology_distance");
	assert_eq!(failure.stage, "resolve_inputs");
	assert_eq!(failure.category, FailureCategory::InvalidInput);
}
