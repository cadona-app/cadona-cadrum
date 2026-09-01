//! Narrow integration-test support for the custom B-rep tessellator.

use super::{ffi, solid::Solid, tessellation};
use crate::{Error, Tessellation};

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
