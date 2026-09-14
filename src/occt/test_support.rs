//! Narrow integration-test support for the custom B-rep tessellator.

use super::{ffi, solid::Solid, tessellation};
use crate::{Error, Tessellation};

/// Face-cache hits, misses, entries and retained bytes; optionally clear it after reading.
pub fn tessellation_face_cache_statistics(clear: bool) -> [usize; 4] {
	tessellation::face_cache_statistics(clear)
}

/// Direction of one face-loop edge occurrence relative to the canonical
/// topological edge sample sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryDirection {
	Forward,
	Reversed,
}

/// Provenance retained for one contiguous edge occurrence in a face loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundaryRun {
	pub face_index: u32,
	pub loop_index: u32,
	pub edge_index: u32,
	pub edge_occurrence_index: u32,
	pub direction: BoundaryDirection,
	pub sample_ordinals: Vec<u32>,
}

/// Extract and decode the face-loop boundary provenance consumed by the
/// custom Rust tessellator. This entry point exists only with `test-support`.
pub fn tessellation_boundary_runs(solid: &Solid, options: Tessellation) -> Result<Vec<BoundaryRun>, Error> {
	let source = extract_tessellation_source(solid, options)?;
	tessellation::decode_boundary_run_provenance(source).map(|runs| {
		runs.into_iter()
			.map(|run| BoundaryRun {
				face_index: run.face_index,
				loop_index: run.loop_index,
				edge_index: run.edge_index,
				edge_occurrence_index: run.edge_occurrence_index,
				direction: if run.reversed { BoundaryDirection::Reversed } else { BoundaryDirection::Forward },
				sample_ordinals: run.sample_ordinals,
			})
			.collect()
	})
}

/// Confirm that the production decoder rejects an invalid edge-occurrence
/// direction rather than silently guessing its traversal orientation.
pub fn tessellation_rejects_invalid_boundary_direction(solid: &Solid, options: Tessellation) -> Result<bool, Error> {
	let mut source = extract_tessellation_source(solid, options)?;
	let direction = source.loop_edge_occurrence_directions.first_mut().ok_or(Error::TriangulationFailed)?;
	*direction = u8::MAX;
	Ok(tessellation::decode_boundary_run_provenance(source).is_err())
}

/// Confirm that a vertex shared by more boundary occurrences than the fixed
/// metadata representation can encode fails explicitly instead of dropping
/// provenance.
pub fn tessellation_rejects_boundary_metadata_overflow() -> bool {
	tessellation::boundary_occurrence_metadata_overflow_is_rejected()
}

/// Exercise the exact per-face trim limits without allocating trim payloads.
pub fn tessellation_synthetic_trim_resource_limit_errors() -> Result<(Error, Error), Error> {
	tessellation::synthetic_trim_resource_limit_errors()
}

/// Exercise request-wide resource accounting in serial or bounded parallel mode.
#[cfg(not(target_arch = "wasm32"))]
pub fn tessellation_synthetic_aggregate_resource_limit_error(parallel: bool) -> Result<Error, Error> {
	tessellation::synthetic_aggregate_resource_limit_error(parallel)
}

/// Return count-only request totals accumulated with the requested worker count.
#[cfg(not(target_arch = "wasm32"))]
pub fn tessellation_synthetic_resource_accounting_totals(worker_count: usize) -> Result<[usize; 4], Error> {
	tessellation::synthetic_resource_accounting_totals(worker_count)
}

/// Exercise the complete 64-ring structured-patch vertex preplan.
pub fn tessellation_synthetic_transition_ring_resource_limit_error() -> Result<Error, Error> {
	tessellation::synthetic_transition_ring_resource_limit_error()
}

/// Exercise the exact output-payload byte quota without allocating payloads.
pub fn tessellation_synthetic_payload_resource_limit_error() -> Result<Error, Error> {
	tessellation::synthetic_payload_resource_limit_error()
}

/// Confirm that the production snapshot decoder rejects malformed parallel
/// arrays, invalid rational-surface values, broken boundary references, and
/// duplicate face identities before constructing a tessellator domain value.
pub fn tessellation_rejects_corrupted_source_contracts(solid: &Solid, options: Tessellation) -> Result<bool, Error> {
	let mut malformed_offsets = extract_tessellation_source(solid, options)?;
	malformed_offsets.face_pole_offsets.pop();
	let malformed_offsets_rejected = tessellation::decode_boundary_run_provenance(malformed_offsets).is_err();

	let mut non_finite_weight = extract_tessellation_source(solid, options)?;
	*non_finite_weight.weights.first_mut().ok_or(Error::TriangulationFailed)? = f64::NAN;
	let non_finite_weight_rejected = tessellation::decode_boundary_run_provenance(non_finite_weight).is_err();

	let mut invalid_boundary_reference = extract_tessellation_source(solid, options)?;
	*invalid_boundary_reference.loop_edge_sample_indices.first_mut().ok_or(Error::TriangulationFailed)? = u32::MAX;
	let invalid_boundary_reference_rejected = tessellation::decode_boundary_run_provenance(invalid_boundary_reference).is_err();

	let mut excessive_approximation = extract_tessellation_source(solid, options)?;
	*excessive_approximation.face_approximation_errors.first_mut().ok_or(Error::TriangulationFailed)? = excessive_approximation.linear_deflection * 2.0;
	let excessive_approximation_rejected = tessellation::decode_boundary_run_provenance(excessive_approximation).is_err();

	let mut duplicate_face = extract_tessellation_source(solid, options)?;
	if duplicate_face.face_indices.len() < 2 {
		return Err(Error::TriangulationFailed);
	}
	duplicate_face.face_indices[1] = duplicate_face.face_indices[0];
	let duplicate_face_rejected = tessellation::decode_boundary_run_provenance(duplicate_face).is_err();

	Ok(malformed_offsets_rejected && non_finite_weight_rejected && invalid_boundary_reference_rejected && excessive_approximation_rejected && duplicate_face_rejected)
}

/// Exercise the native extraction preflight with fixture-sized limits.
///
/// This keeps the OCCT shape private while allowing integration tests to prove
/// that topology maps and direct B-spline storage stop before crossing their
/// configured quotas.
pub fn tessellation_extraction_preflight_with_limits(solid: &Solid, maximum_faces: u32, maximum_edges: u32, maximum_vertices: u32, maximum_control_points: u32, maximum_knots: u32) -> Result<(), Error> {
	ffi::begin_operation();
	let progress = ffi::CancellationToken::new();
	if ffi::test_brep_extraction_preflight_limits(solid.inner(), maximum_faces, maximum_edges, maximum_vertices, maximum_control_points, maximum_knots, &progress) {
		return Ok(());
	}
	Err(ffi::operation_error(Error::TriangulationFailed, "preflight B-rep tessellation extraction", "preflight"))
}

/// OCCT triangulation cache counts retained on an authoritative test shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OcctTriangulationCacheStats {
	pub face_count: u32,
	pub triangulated_face_count: u32,
	pub node_count: u64,
	pub triangle_count: u64,
}

/// Seed the authoritative shape's OCCT cache for detached-copy regression tests.
pub fn tessellation_seed_occt_cache(solid: &Solid, options: Tessellation) -> Result<(), Error> {
	ffi::begin_operation();
	if ffi::test_seed_occt_triangulation_cache(solid.inner(), options.deflection_linear, options.deflection_angular, options.relative_linear) {
		return Ok(());
	}
	Err(ffi::operation_error(Error::TriangulationFailed, "seed OCCT triangulation cache", "mesh"))
}

/// Inspect the authoritative shape's cache without exposing an OCCT handle.
pub fn tessellation_occt_cache_stats(solid: &Solid) -> Result<OcctTriangulationCacheStats, Error> {
	let stats = ffi::test_occt_triangulation_cache(solid.inner());
	if !stats.success {
		return Err(Error::TriangulationFailed);
	}
	Ok(OcctTriangulationCacheStats { face_count: stats.face_count, triangulated_face_count: stats.triangulated_face_count, node_count: stats.node_count, triangle_count: stats.triangle_count })
}

fn extract_tessellation_source(solid: &Solid, options: Tessellation) -> Result<ffi::BrepMeshSourceData, Error> {
	if !options.deflection_linear.is_finite() || options.deflection_linear <= 0.0 {
		return Err(Error::InvalidInput("tessellation linear deflection must be finite and greater than zero".into()));
	}
	if !options.deflection_angular.is_finite() || options.deflection_angular <= 0.0 {
		return Err(Error::InvalidInput("tessellation angular deflection must be finite and greater than zero".into()));
	}
	ffi::begin_operation();
	let progress = ffi::CancellationToken::new();
	let source = ffi::extract_brep_mesh_source(solid.inner(), &[], options.deflection_linear, options.deflection_angular, options.relative_linear, &progress);
	if !source.success {
		return Err(ffi::operation_error(Error::TriangulationFailed, "extract B-rep tessellation provenance", "extract"));
	}
	Ok(source)
}

/// Exercise a cylinder chart whose normalized conversion fits the source-surface
/// budget but exceeds it when combined with a small canonical edge discrepancy.
pub fn tessellation_chart_mapping_errors() -> Vec<f64> {
	ffi::test_brep_chart_mapping_errors()
}
