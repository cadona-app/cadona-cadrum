#![cfg(feature = "test-support")]

use cadrum::{occt::test_support::tessellation_extraction_preflight_with_limits, DVec3, Error, FailureCategory, Solid};

fn assert_resource_limit(error: Error, expected_message: &str) {
	let Error::OperationFailed(failure) = error else {
		panic!("expected structured extraction resource failure");
	};
	assert_eq!(failure.operation, "extract_brep_mesh_source");
	assert_eq!(failure.stage, "resource_limit");
	assert_eq!(failure.category, FailureCategory::ResourceLimit);
	assert!(failure.message.contains(expected_message), "unexpected resource failure: {failure:?}");
}

#[test]
fn topology_preflight_stops_before_unique_shape_limits_are_crossed() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));

	tessellation_extraction_preflight_with_limits(&cube, 6, 12, 8, 0, 0).expect("exact cube topology fits its limits");

	for (limits, message) in [((5, 12, 8), "shape face quota"), ((6, 11, 8), "shape edge quota"), ((6, 12, 7), "shape vertex quota")] {
		let error = tessellation_extraction_preflight_with_limits(&cube, limits.0, limits.1, limits.2, 0, 0).expect_err("topology above its configured preflight limit must fail");
		assert_resource_limit(error, message);
	}
}

#[test]
fn direct_bspline_storage_is_rejected_before_the_extraction_copy() {
	let surface = Solid::bspline(3, 8, false, |u, v| {
		let angle = std::f64::consts::TAU * v as f64 / 8.0;
		DVec3::new(u as f64 * 4.0, angle.cos() * 2.0, angle.sin() * 2.0)
	})
	.expect("construct B-spline fixture");

	let control_point_error = tessellation_extraction_preflight_with_limits(&surface, 100, 200, 200, 1, u32::MAX).expect_err("B-spline control-point storage above the configured limit must fail");
	assert_resource_limit(control_point_error, "surface-copy control-point quota");

	let knot_error = tessellation_extraction_preflight_with_limits(&surface, 100, 200, 200, u32::MAX, 1).expect_err("B-spline knot storage above the configured limit must fail");
	assert_resource_limit(knot_error, "surface-copy knot quota");
}
