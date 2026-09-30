#![cfg(all(feature = "test-support", not(target_arch = "wasm32")))]

use cadrum::{
	occt::test_support::{tessellation_synthetic_aggregate_resource_limit_error, tessellation_synthetic_payload_resource_limit_error, tessellation_synthetic_resource_accounting_totals, tessellation_synthetic_transition_ring_resource_limit_error, tessellation_synthetic_trim_resource_limit_errors},
	Error, FailureCategory,
};

fn resource_failure(error: Error) -> (FailureCategory, String, String) {
	let Error::OperationFailed(failure) = error else {
		panic!("expected a structured resource-limit failure");
	};
	(failure.category, failure.stage, failure.message)
}

#[test]
fn trim_quotas_accept_the_boundary_and_reject_one_item_over() {
	let (vertex_error, loop_error) = tessellation_synthetic_trim_resource_limit_errors().expect("exercise synthetic trim limits");
	let vertex_failure = resource_failure(vertex_error);
	let loop_failure = resource_failure(loop_error);

	assert_eq!(vertex_failure.0, FailureCategory::ResourceLimit);
	assert_eq!(vertex_failure.1, "resource_limit");
	assert!(vertex_failure.2.contains("trim-vertex"));
	assert_eq!(loop_failure.0, FailureCategory::ResourceLimit);
	assert_eq!(loop_failure.1, "resource_limit");
	assert!(loop_failure.2.contains("trim-loop"));
}

#[test]
fn aggregate_quota_failure_is_identical_in_serial_and_parallel_modes() {
	let serial = resource_failure(tessellation_synthetic_aggregate_resource_limit_error(false).expect("serial request must reach its count-only quota"));
	let parallel = resource_failure(tessellation_synthetic_aggregate_resource_limit_error(true).expect("parallel request must reach its count-only quota"));

	assert_eq!(serial, parallel);
	assert_eq!(serial.0, FailureCategory::ResourceLimit);
	assert_eq!(serial.1, "resource_limit");
	assert!(serial.2.contains("aggregate mesh resource limits"));
}

#[test]
fn count_only_accounting_is_exact_across_rayon_pool_sizes() {
	let expected = [4_194_304, 8_388_608, 25_165_824, 369_098_752];
	for worker_count in [1, 2, 4, 8] {
		assert_eq!(tessellation_synthetic_resource_accounting_totals(worker_count).expect("accumulate synthetic request counts"), expected, "resource totals changed with {worker_count} workers");
	}
}

#[test]
fn complete_sixty_four_ring_plan_rejects_before_vertex_allocation() {
	let failure = resource_failure(tessellation_synthetic_transition_ring_resource_limit_error().expect("oversized structured transition plan must fail"));

	assert_eq!(failure.0, FailureCategory::ResourceLimit);
	assert_eq!(failure.1, "resource_limit");
	assert!(failure.2.contains("transition rings"));
}

#[test]
fn output_payload_quota_rejects_one_item_over_without_allocating_it() {
	let failure = resource_failure(tessellation_synthetic_payload_resource_limit_error().expect("oversized output payload plan must fail"));

	assert_eq!(failure.0, FailureCategory::ResourceLimit);
	assert_eq!(failure.1, "resource_limit");
	assert!(failure.2.contains("output payload limit"));
}
