//! Safe, deterministic tessellation of Rust-owned B-rep surface snapshots.
//!
//! OCCT is used only to extract exact topology, trims, shared-edge samples,
//! and bounded rational B-spline surfaces. Triangle construction, refinement,
//! normals, quality control, parallel scheduling, and assembly live here.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex, OnceLock};

use glam::{DVec2, DVec3};
use spade::{ConstrainedDelaunayTriangulation, HasPosition, Intersection, LineIntersectionIterator, Point2, PositionInTriangulation, Triangulation};

use super::ffi;
use crate::{Error, FailureCategory, OperationFailure, Tessellation};

// Indices are u32 throughout the Rust and renderer boundaries. Keep a finite
// resource ceiling without imposing the obsolete u16-sized limit that made a
// valid high-resolution face impossible to export.
const MAXIMUM_FACE_VERTICES: usize = 1_048_576;
const MAXIMUM_FACE_TRIM_VERTICES: usize = 262_144;
const MAXIMUM_FACE_TRIM_LOOPS: usize = 16_384;
const MAXIMUM_CDT_FACE_VERTICES: usize = 524_288;
const MAXIMUM_REQUEST_VERTICES: usize = 4_194_304;
const MAXIMUM_REQUEST_TRIANGLES: usize = 8_388_608;
const MAXIMUM_REQUEST_INDICES: usize = 25_165_824;
const MAXIMUM_REQUEST_PAYLOAD_BYTES: usize = 512 * 1024 * 1024;
// Curved-shape throughput peaks at four face workers on representative Apple
// Silicon: two leaves independent patches idle, while eight loses to scheduling
// and shared-allocation overhead. Keep the pool bounded and benchmark changes.
const MAXIMUM_PARALLEL_FACES: usize = 4;
const MAXIMUM_STRUCTURED_SEED_INSERTIONS: usize = 65_536;
const MAXIMUM_BOUNDARY_COLLAR_INSERTIONS: usize = 32_768;
const MAXIMUM_LATTICE_INSERTIONS: usize = 32_768;
const MAXIMUM_REQUIRED_REFINEMENT_PASSES: usize = 32;
const MAXIMUM_QUALITY_REFINEMENT_PASSES: usize = 8;
const MAXIMUM_REQUIRED_INSERTIONS_PER_PASS: usize = 2_048;
const MAXIMUM_QUALITY_INSERTIONS_PER_PASS: usize = 512;
const MAXIMUM_REQUIRED_INSERTIONS: usize = 65_536;
const MAXIMUM_QUALITY_INSERTIONS: usize = 4_096;
// Geom_BSplineSurface::MaxDegree() is 25. The FFI extracts only OCCT surfaces,
// so fixed storage covers every valid input without a hot-path heap allocation.
const MAXIMUM_BSPLINE_DEGREE: usize = 25;
const MAXIMUM_BASIS_WIDTH: usize = MAXIMUM_BSPLINE_DEGREE + 1;
const TARGET_PHYSICAL_ASPECT: f64 = 6.0;
const MAXIMUM_PHYSICAL_ASPECT: f64 = 12.0;
const MAXIMUM_HARD_PHYSICAL_ASPECT: f64 = 56.0;
const MAXIMUM_BALANCED_AXIS_INTERVALS: usize = 128;
const CANCELLATION_CHECK_INTERVAL: usize = 128;
const RUST_MESH_PROGRESS_START: f64 = 0.25;
const RUST_MESH_PROGRESS_STARTED_SPAN: f64 = 0.05;
const RUST_MESH_PROGRESS_COMPLETED_SPAN: f64 = 0.65;

fn check_cancelled(progress: &ffi::CancellationToken) -> Result<(), Error> {
	if progress.is_cancelled() {
		Err(Error::Cancelled)
	} else {
		Ok(())
	}
}

fn cancellation_checkpoint(progress: &ffi::CancellationToken, ordinal: usize) -> Result<(), Error> {
	if ordinal.is_multiple_of(CANCELLATION_CHECK_INTERVAL) {
		check_cancelled(progress)?;
	}
	Ok(())
}

fn resource_limit(message: impl Into<String>) -> Error {
	Error::OperationFailed(OperationFailure { operation: "tessellate B-rep".into(), stage: "resource_limit".into(), exception_type: None, message: message.into(), category: FailureCategory::ResourceLimit, status: None })
}

fn checked_add_resource(first: usize, second: usize, message: &'static str) -> Result<usize, Error> {
	first.checked_add(second).ok_or_else(|| resource_limit(message))
}

fn checked_mul_resource(first: usize, second: usize, message: &'static str) -> Result<usize, Error> {
	first.checked_mul(second).ok_or_else(|| resource_limit(message))
}

fn reserve_exact<T>(values: &mut Vec<T>, additional: usize, message: &'static str) -> Result<(), Error> {
	values.try_reserve_exact(additional).map_err(|_| resource_limit(message))
}

fn validate_request_payload_bytes(vertex_count: usize, triangle_count: usize, face_count: usize, edge_point_count: usize, edge_count: usize) -> Result<usize, Error> {
	let face_offset_count = checked_add_resource(face_count, 1, "tessellation face-offset count overflowed")?;
	let edge_offset_count = checked_add_resource(edge_count, 1, "tessellation edge-offset count overflowed")?;
	let mut payload_bytes = checked_mul_resource(vertex_count, 48, "tessellation request payload size overflowed")?;
	payload_bytes = checked_add_resource(payload_bytes, checked_mul_resource(triangle_count, 20, "tessellation request payload size overflowed")?, "tessellation request payload size overflowed")?;
	payload_bytes = checked_add_resource(payload_bytes, checked_mul_resource(face_count, 12, "tessellation request payload size overflowed")?, "tessellation request payload size overflowed")?;
	payload_bytes = checked_add_resource(payload_bytes, checked_mul_resource(face_offset_count, 8, "tessellation request payload size overflowed")?, "tessellation request payload size overflowed")?;
	payload_bytes = checked_add_resource(payload_bytes, checked_mul_resource(edge_point_count, 24, "tessellation request payload size overflowed")?, "tessellation request payload size overflowed")?;
	payload_bytes = checked_add_resource(payload_bytes, checked_mul_resource(edge_count, 4, "tessellation request payload size overflowed")?, "tessellation request payload size overflowed")?;
	payload_bytes = checked_add_resource(payload_bytes, checked_mul_resource(edge_offset_count, 4, "tessellation request payload size overflowed")?, "tessellation request payload size overflowed")?;
	if payload_bytes > MAXIMUM_REQUEST_PAYLOAD_BYTES {
		return Err(resource_limit("tessellation request exceeded the output payload limit"));
	}
	Ok(payload_bytes)
}

#[cfg(not(target_arch = "wasm32"))]
fn bounded_face_pool() -> Result<&'static rayon::ThreadPool, Error> {
	static POOL: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();
	POOL.get_or_init(|| rayon::ThreadPoolBuilder::new().num_threads(MAXIMUM_PARALLEL_FACES).thread_name(|index| format!("cadrum-tessellation-{index}")).build().ok()).as_ref().ok_or_else(|| resource_limit("tessellation could not create its bounded worker pool"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EdgeOccurrenceDirection {
	Forward,
	Reversed,
}

impl EdgeOccurrenceDirection {
	fn decode(value: u8) -> Option<Self> {
		match value {
			0 => Some(Self::Forward),
			1 => Some(Self::Reversed),
			_ => None,
		}
	}

	fn samples_are_ordered(self, first: u32, second: u32) -> bool {
		match self {
			Self::Forward => first < second,
			Self::Reversed => first > second,
		}
	}
}

#[derive(Clone, Copy, Debug)]
struct BoundaryVertex {
	uv: DVec2,
	position: DVec3,
	edge_index: u32,
	edge_sample_index: u32,
	edge_occurrence_index: u32,
	edge_occurrence_direction: EdgeOccurrenceDirection,
}

#[derive(Clone, Debug)]
struct TrimLoop {
	vertices: Vec<BoundaryVertex>,
}

#[derive(Clone, Debug)]
struct RationalSurface {
	u_degree: usize,
	v_degree: usize,
	u_count: usize,
	v_count: usize,
	poles: Vec<DVec3>,
	local_poles: Vec<DVec3>,
	origin: DVec3,
	weights: Vec<f64>,
	u_knots: Vec<f64>,
	v_knots: Vec<f64>,
	uv_bounds: [f64; 4],
	approximation_error: f64,
}

#[derive(Clone, Debug)]
struct TrimmedFace {
	index: u32,
	tshape_id: u64,
	reversed: bool,
	surface: RationalSurface,
	loops: Vec<TrimLoop>,
	collapsed_boundaries: CollapsedBoundaries,
}

#[derive(Clone, Copy, Debug, Default)]
struct CollapsedBoundaries {
	u_at_v_min: bool,
	u_at_v_max: bool,
	v_at_u_min: bool,
	v_at_u_max: bool,
}

#[derive(Clone, Debug)]
struct BrepMeshSource {
	faces: Vec<TrimmedFace>,
	edges: Vec<Vec<DVec3>>,
	linear_deflection: f64,
}

#[derive(Clone, Copy, Debug)]
struct SurfaceSample {
	position: DVec3,
	du: DVec3,
	dv: DVec3,
}

impl SurfaceSample {
	fn normal(self) -> Option<DVec3> {
		let cross = self.du.cross(self.dv);
		let derivative_scale = self.du.length().max(self.dv.length());
		if !derivative_scale.is_finite() || cross.length() <= derivative_scale * derivative_scale * 1.0e-12 {
			return None;
		}
		cross.try_normalize()
	}
}

impl RationalSurface {
	fn evaluate_position(&self, uv: DVec2) -> Option<DVec3> {
		if !uv.is_finite() {
			return None;
		}
		let u = uv.x.clamp(self.uv_bounds[0], self.uv_bounds[1]);
		let v = uv.y.clamp(self.uv_bounds[2], self.uv_bounds[3]);
		let mut u_values = [0.0; MAXIMUM_BASIS_WIDTH];
		let mut v_values = [0.0; MAXIMUM_BASIS_WIDTH];
		let u_basis = basis_values(self.u_count, self.u_degree, &self.u_knots, u, &mut u_values)?;
		let v_basis = basis_values(self.v_count, self.v_degree, &self.v_knots, v, &mut v_values)?;
		let u_values = &u_values[..u_basis.value_count];
		let v_values = &v_values[..v_basis.value_count];

		let mut numerator = DVec3::ZERO;
		let mut denominator = 0.0;
		for (v_offset, v_value) in v_values.iter().copied().enumerate() {
			let v_index = v_basis.first + v_offset;
			for (u_offset, u_value) in u_values.iter().copied().enumerate() {
				let u_index = u_basis.first + u_offset;
				let index = v_index * self.u_count + u_index;
				let basis = u_value * v_value * self.weights[index];
				numerator += self.local_poles[index] * basis;
				denominator += basis;
			}
		}
		if !denominator.is_finite() || denominator.abs() <= f64::EPSILON {
			return None;
		}
		let position = self.origin + numerator / denominator;
		position.is_finite().then_some(position)
	}

	fn triangle_crosses_nonsmooth_knot(&self, uvs: [DVec2; 3]) -> bool {
		fn axis_crosses(values: [f64; 3], knots: &[f64], degree: usize, minimum: f64, maximum: f64) -> bool {
			if degree == 0 || knots.is_empty() {
				return false;
			}
			let triangle_minimum = values.into_iter().fold(f64::INFINITY, f64::min);
			let triangle_maximum = values.into_iter().fold(f64::NEG_INFINITY, f64::max);
			let tolerance = (maximum - minimum).abs().max(1.0) * 1.0e-12;
			let mut index = 0;
			while index < knots.len() {
				let knot = knots[index];
				let mut end = index + 1;
				while end < knots.len() && (knots[end] - knot).abs() <= tolerance {
					end += 1;
				}
				let multiplicity = end - index;
				if knot > minimum + tolerance && knot < maximum - tolerance && multiplicity >= degree && triangle_minimum <= knot + tolerance && triangle_maximum >= knot - tolerance {
					return true;
				}
				index = end;
			}
			false
		}

		axis_crosses(uvs.map(|uv| uv.x), &self.u_knots, self.u_degree, self.uv_bounds[0], self.uv_bounds[1]) || axis_crosses(uvs.map(|uv| uv.y), &self.v_knots, self.v_degree, self.uv_bounds[2], self.uv_bounds[3])
	}

	fn is_planar(&self) -> bool {
		let Some(origin) = self.poles.first().copied() else {
			return false;
		};
		let scale = self.poles.iter().map(|pole| pole.distance(origin)).fold(0.0, f64::max);
		if !scale.is_finite() || scale <= 0.0 {
			return false;
		}
		let mut normal = None;
		for first in self.poles.iter().copied().skip(1) {
			for second in self.poles.iter().copied().skip(2) {
				if let Some(candidate) = (first - origin).cross(second - origin).try_normalize() {
					normal = Some(candidate);
					break;
				}
			}
			if normal.is_some() {
				break;
			}
		}
		let Some(normal) = normal else {
			return false;
		};
		// Exact planar faces can acquire a few world-coordinate ULPs when a
		// small body is rotated and placed far from the origin. Compare against
		// the local face scale, the measured snapshot-to-boundary error, and a
		// tight world-coordinate rounding allowance so route selection remains
		// invariant under rigid placement without flattening genuinely curved
		// control nets.
		let world_scale = self.poles.iter().flat_map(|pole| pole.to_array()).map(f64::abs).fold(0.0, f64::max);
		let tolerance = (scale * 1.0e-10).max(self.approximation_error * 2.0).max(world_scale * f64::EPSILON * 4.0);
		self.poles.iter().all(|pole| (*pole - origin).dot(normal).abs() <= tolerance)
	}

	fn needs_world_stable_planar_route(&self) -> bool {
		let Some(origin) = self.poles.first().copied() else {
			return false;
		};
		let local_scale = self.poles.iter().map(|pole| pole.distance(origin)).fold(0.0, f64::max);
		let world_scale = self.poles.iter().flat_map(|pole| pole.to_array()).map(f64::abs).fold(0.0, f64::max);
		local_scale.is_finite() && local_scale > 0.0 && world_scale.is_finite() && world_scale * f64::EPSILON * 8.0 > local_scale * 1.0e-10
	}

	fn evaluate(&self, uv: DVec2) -> Option<SurfaceSample> {
		if !uv.is_finite() {
			return None;
		}
		let u = uv.x.clamp(self.uv_bounds[0], self.uv_bounds[1]);
		let v = uv.y.clamp(self.uv_bounds[2], self.uv_bounds[3]);
		let mut u_values = [0.0; MAXIMUM_BASIS_WIDTH];
		let mut u_derivatives = [0.0; MAXIMUM_BASIS_WIDTH];
		let mut v_values = [0.0; MAXIMUM_BASIS_WIDTH];
		let mut v_derivatives = [0.0; MAXIMUM_BASIS_WIDTH];
		let u_basis = basis_and_derivative(self.u_count, self.u_degree, &self.u_knots, u, &mut u_values, &mut u_derivatives)?;
		let v_basis = basis_and_derivative(self.v_count, self.v_degree, &self.v_knots, v, &mut v_values, &mut v_derivatives)?;
		let u_values = &u_values[..u_basis.value_count];
		let u_derivatives = &u_derivatives[..u_basis.value_count];
		let v_values = &v_values[..v_basis.value_count];
		let v_derivatives = &v_derivatives[..v_basis.value_count];

		// Evaluate in a deterministic face-local frame. Rational derivative
		// recovery subtracts two weighted positions; accumulating world-space
		// poles first makes that cancellation depend on a rigid translation.
		let mut numerator = DVec3::ZERO;
		let mut numerator_u = DVec3::ZERO;
		let mut numerator_v = DVec3::ZERO;
		let mut denominator = 0.0;
		let mut denominator_u = 0.0;
		let mut denominator_v = 0.0;
		// A degree-p B-spline has at most p + 1 non-zero basis functions at a
		// parameter. Imported lofts can contain hundreds of poles along one axis;
		// walking the whole control net for every adaptive probe made evaluation
		// quadratic in data that cannot influence that probe.
		for v_offset in 0..v_values.len() {
			let v_index = v_basis.first + v_offset;
			for u_offset in 0..u_values.len() {
				let u_index = u_basis.first + u_offset;
				let index = v_index * self.u_count + u_index;
				let weight = self.weights[index];
				let basis = u_values[u_offset] * v_values[v_offset] * weight;
				let basis_u = u_derivatives[u_offset] * v_values[v_offset] * weight;
				let basis_v = u_values[u_offset] * v_derivatives[v_offset] * weight;
				let local_pole = self.local_poles[index];
				numerator += local_pole * basis;
				numerator_u += local_pole * basis_u;
				numerator_v += local_pole * basis_v;
				denominator += basis;
				denominator_u += basis_u;
				denominator_v += basis_v;
			}
		}
		if !denominator.is_finite() || denominator.abs() <= f64::EPSILON {
			return None;
		}
		let local_position = numerator / denominator;
		let position = self.origin + local_position;
		let du = (numerator_u - local_position * denominator_u) / denominator;
		let dv = (numerator_v - local_position * denominator_v) / denominator;
		(position.is_finite() && du.is_finite() && dv.is_finite()).then_some(SurfaceSample { position, du, dv })
	}
}

struct BasisSupport {
	first: usize,
	value_count: usize,
}

fn basis_values(control_count: usize, degree: usize, knots: &[f64], parameter: f64, values: &mut [f64; MAXIMUM_BASIS_WIDTH]) -> Option<BasisSupport> {
	if degree == 0 || degree > MAXIMUM_BSPLINE_DEGREE {
		return None;
	}
	let value_count = degree.checked_add(1)?;
	let mut scratch = [0.0; MAXIMUM_BASIS_WIDTH * 2];
	let first = fill_local_basis_values(control_count, degree, knots, parameter, &mut values[..value_count], &mut scratch[..value_count.checked_mul(2)?])?;
	Some(BasisSupport { first, value_count })
}

fn basis_and_derivative(control_count: usize, degree: usize, knots: &[f64], parameter: f64, values: &mut [f64; MAXIMUM_BASIS_WIDTH], derivatives: &mut [f64; MAXIMUM_BASIS_WIDTH]) -> Option<BasisSupport> {
	let expected_knot_count = control_count.checked_add(degree)?.checked_add(1)?;
	if degree == 0 || degree > MAXIMUM_BSPLINE_DEGREE || control_count < 2 || knots.len() != expected_knot_count {
		return None;
	}
	let value_count = degree.checked_add(1)?;
	let mut lower_values = [0.0; MAXIMUM_BSPLINE_DEGREE];
	let mut recurrence_scratch = [0.0; MAXIMUM_BASIS_WIDTH * 2];
	let first = fill_local_basis_values(control_count, degree, knots, parameter, &mut values[..value_count], &mut recurrence_scratch[..value_count.checked_mul(2)?])?;
	let domain_end = knots[control_count];
	let lower_parameter = if parameter >= domain_end { next_down(domain_end) } else { parameter };
	let lower_first = fill_local_basis_values(control_count.checked_add(1)?, degree - 1, knots, lower_parameter, &mut lower_values[..degree], &mut recurrence_scratch[..degree.checked_mul(2)?])?;
	let lower_values = &lower_values[..degree];
	for (offset, derivative) in derivatives.iter_mut().enumerate().take(value_count) {
		let index = first + offset;
		let left_denominator = knots[index + degree] - knots[index];
		let right_denominator = knots[index + degree + 1] - knots[index + 1];
		let left_basis = index.checked_sub(lower_first).and_then(|lower_offset| lower_values.get(lower_offset)).copied().unwrap_or(0.0);
		let right_basis = (index + 1).checked_sub(lower_first).and_then(|lower_offset| lower_values.get(lower_offset)).copied().unwrap_or(0.0);
		let left = if left_denominator.abs() > f64::EPSILON { degree as f64 * left_basis / left_denominator } else { 0.0 };
		let right = if right_denominator.abs() > f64::EPSILON { degree as f64 * right_basis / right_denominator } else { 0.0 };
		*derivative = left - right;
	}
	(values[..value_count].iter().chain(&derivatives[..value_count]).all(|value| value.is_finite())).then_some(BasisSupport { first, value_count })
}

fn next_down(value: f64) -> f64 {
	if value.is_nan() || value == f64::NEG_INFINITY {
		return value;
	}
	if value == 0.0 {
		return -f64::from_bits(1);
	}
	let bits = value.to_bits();
	if value > 0.0 {
		f64::from_bits(bits - 1)
	} else {
		f64::from_bits(bits + 1)
	}
}

fn fill_local_basis_values(control_count: usize, degree: usize, knots: &[f64], parameter: f64, values: &mut [f64], scratch: &mut [f64]) -> Option<usize> {
	let width = degree.checked_add(1)?;
	let expected_knot_count = control_count.checked_add(width)?;
	if control_count == 0 || degree >= control_count || knots.len() != expected_knot_count || values.len() != width || scratch.len() != width.checked_mul(2)? || !parameter.is_finite() {
		return None;
	}
	let domain_start = knots[degree];
	let domain_end = knots[control_count];
	if parameter < domain_start || parameter > domain_end {
		return None;
	}
	let span = find_knot_span(control_count, degree, knots, parameter)?;
	let right_offset = width;
	values.fill(0.0);
	scratch.fill(0.0);
	values[0] = 1.0;
	for column in 1..=degree {
		scratch[column] = parameter - knots[span + 1 - column];
		scratch[right_offset + column] = knots[span + column] - parameter;
		let mut saved = 0.0;
		for row in 0..column {
			let denominator = scratch[right_offset + row + 1] + scratch[column - row];
			let temporary = if denominator.abs() > f64::EPSILON { values[row] / denominator } else { 0.0 };
			values[row] = saved + scratch[right_offset + row + 1] * temporary;
			saved = scratch[column - row] * temporary;
		}
		values[column] = saved;
	}
	values.iter().all(|value| value.is_finite()).then_some(span - degree)
}

fn find_knot_span(control_count: usize, degree: usize, knots: &[f64], parameter: f64) -> Option<usize> {
	let last_span = control_count.checked_sub(1)?;
	if parameter == knots[control_count] {
		return Some(last_span);
	}
	let mut low = degree;
	let mut high = control_count;
	while low < high {
		let middle = low + (high - low) / 2;
		if parameter < knots[middle] {
			high = middle;
		} else if parameter >= knots[middle + 1] {
			low = middle + 1;
		} else {
			return Some(middle);
		}
	}
	(low <= last_span && knots[low] <= parameter && parameter < knots[low + 1]).then_some(low)
}

#[derive(Clone, Copy, Debug)]
struct MetricMap {
	origin: DVec2,
	x_u: f64,
	x_v: f64,
	y_v: f64,
}

impl MetricMap {
	fn from_face(face: &TrimmedFace) -> Self {
		let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
		let probes = [DVec2::new((u_min + u_max) * 0.5, (v_min + v_max) * 0.5), DVec2::new(u_min * 0.25 + u_max * 0.75, (v_min + v_max) * 0.5), DVec2::new((u_min + u_max) * 0.5, v_min * 0.25 + v_max * 0.75), DVec2::new(u_min * 0.75 + u_max * 0.25, v_min * 0.75 + v_max * 0.25)];
		let best = probes.into_iter().filter_map(|uv| face.surface.evaluate(uv).map(|sample| (uv, sample))).max_by(|(_, first), (_, second)| first.du.cross(first.dv).length_squared().total_cmp(&second.du.cross(second.dv).length_squared()));
		if let Some((origin, sample)) = best {
			let g_uu = sample.du.length_squared().max(1.0e-24);
			let g_uv = sample.du.dot(sample.dv);
			let g_vv = sample.dv.length_squared().max(1.0e-24);
			let x_u = g_uu.sqrt();
			let x_v = g_uv / x_u;
			let y_v = (g_vv - x_v * x_v).max(1.0e-24).sqrt();
			if x_u.is_finite() && x_v.is_finite() && y_v.is_finite() {
				return Self { origin, x_u, x_v, y_v };
			}
		}
		Self { origin: DVec2::new(u_min, v_min), x_u: 1.0 / (u_max - u_min).abs().max(1.0e-12), x_v: 0.0, y_v: 1.0 / (v_max - v_min).abs().max(1.0e-12) }
	}

	fn map(self, uv: DVec2) -> Point2<f64> {
		let delta = uv - self.origin;
		Point2::new(delta.x * self.x_u + delta.y * self.x_v, delta.y * self.y_v)
	}

	fn unmap(self, point: Point2<f64>) -> Option<DVec2> {
		if self.x_u.abs() <= 1.0e-24 || self.y_v.abs() <= 1.0e-24 {
			return None;
		}
		let delta_y = point.y / self.y_v;
		let delta_x = (point.x - delta_y * self.x_v) / self.x_u;
		let uv = self.origin + DVec2::new(delta_x, delta_y);
		uv.is_finite().then_some(uv)
	}
}

/// Exact two-dimensional coordinates for a geometrically planar B-rep face.
///
/// A planar rational surface is not required to have an affine UV chart. A
/// constant first-fundamental-form approximation can therefore report good UV
/// triangles even when their physical shape is poor. This chart projects the
/// evaluated surface into an orthonormal plane and inverts candidate points
/// with bounded Newton iterations, so CDT quality is measured in real model
/// units.
#[derive(Clone, Copy, Debug)]
struct PlanarChart {
	origin: DVec3,
	x_axis: DVec3,
	y_axis: DVec3,
	reference_uv: DVec2,
	reference_xy: DVec2,
	bounds: [f64; 4],
	inversion_tolerance: f64,
}

impl PlanarChart {
	fn from_face(face: &TrimmedFace) -> Option<Self> {
		let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
		let probes = [DVec2::new((u_min + u_max) * 0.5, (v_min + v_max) * 0.5), DVec2::new(u_min * 0.75 + u_max * 0.25, (v_min + v_max) * 0.5), DVec2::new((u_min + u_max) * 0.5, v_min * 0.75 + v_max * 0.25), DVec2::new(u_min * 0.25 + u_max * 0.75, v_min * 0.25 + v_max * 0.75)];
		let (reference_uv, sample) = probes.into_iter().filter_map(|uv| face.surface.evaluate(uv).map(|sample| (uv, sample))).max_by(|(_, first), (_, second)| first.du.cross(first.dv).length_squared().total_cmp(&second.du.cross(second.dv).length_squared()))?;
		let normal = sample.du.cross(sample.dv).try_normalize()?;
		let x_axis = sample.du.reject_from_normalized(normal).try_normalize().or_else(|| sample.dv.reject_from_normalized(normal).try_normalize())?;
		let y_axis = normal.cross(x_axis).try_normalize()?;
		let origin = sample.position;
		let project = |position: DVec3| DVec2::new((position - origin).dot(x_axis), (position - origin).dot(y_axis));
		let reference_xy = project(sample.position);
		let (minimum, maximum) = face.loops.iter().flat_map(|trim_loop| trim_loop.vertices.iter().map(|vertex| project(vertex.position))).fold((DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY)), |(minimum, maximum), point| (minimum.min(point), maximum.max(point)));
		let extent = (maximum - minimum).abs().max_element();
		let world_scale = face.surface.poles.iter().flat_map(|pole| pole.to_array()).map(f64::abs).fold(0.0, f64::max);
		let inversion_tolerance = (extent * 1.0e-10).max(face.surface.approximation_error * 4.0).max(world_scale * f64::EPSILON * 8.0);
		(origin.is_finite() && x_axis.is_finite() && y_axis.is_finite() && reference_xy.is_finite() && extent.is_finite() && extent > 1.0e-12 && inversion_tolerance.is_finite()).then_some(Self { origin, x_axis, y_axis, reference_uv, reference_xy, bounds: face.surface.uv_bounds, inversion_tolerance })
	}

	fn map_position(self, position: DVec3) -> Option<Point2<f64>> {
		let offset = position - self.origin;
		let point = Point2::new(offset.dot(self.x_axis), offset.dot(self.y_axis));
		(point.x.is_finite() && point.y.is_finite()).then_some(point)
	}

	fn map_uv(self, face: &TrimmedFace, uv: DVec2) -> Option<Point2<f64>> {
		self.map_position(face.surface.evaluate_position(uv)?)
	}

	fn unmap(self, face: &TrimmedFace, target: Point2<f64>) -> Option<DVec2> {
		const MAXIMUM_NEWTON_ITERATIONS: usize = 16;
		let target = DVec2::new(target.x, target.y);
		let reference = face.surface.evaluate(self.reference_uv)?;
		let reference_jacobian = DVec2::new(reference.du.dot(self.x_axis), reference.du.dot(self.y_axis));
		let reference_second = DVec2::new(reference.dv.dot(self.x_axis), reference.dv.dot(self.y_axis));
		let mut uv = self.reference_uv + solve_planar_jacobian(reference_jacobian, reference_second, target - self.reference_xy)?;
		uv.x = uv.x.clamp(self.bounds[0], self.bounds[1]);
		uv.y = uv.y.clamp(self.bounds[2], self.bounds[3]);
		let tolerance = self.inversion_tolerance;
		for _ in 0..MAXIMUM_NEWTON_ITERATIONS {
			let sample = face.surface.evaluate(uv)?;
			let xy = DVec2::new((sample.position - self.origin).dot(self.x_axis), (sample.position - self.origin).dot(self.y_axis));
			let residual = xy - target;
			if residual.length() <= tolerance {
				return Some(uv);
			}
			let du = DVec2::new(sample.du.dot(self.x_axis), sample.du.dot(self.y_axis));
			let dv = DVec2::new(sample.dv.dot(self.x_axis), sample.dv.dot(self.y_axis));
			let step = solve_planar_jacobian(du, dv, residual)?;
			if !step.is_finite() {
				return None;
			}
			uv -= step;
			uv.x = uv.x.clamp(self.bounds[0], self.bounds[1]);
			uv.y = uv.y.clamp(self.bounds[2], self.bounds[3]);
		}
		let mapped = self.map_uv(face, uv)?;
		(metric_distance(mapped, Point2::new(target.x, target.y)) <= tolerance * 4.0).then_some(uv)
	}
}

fn solve_planar_jacobian(du: DVec2, dv: DVec2, residual: DVec2) -> Option<DVec2> {
	let determinant = du.x * dv.y - dv.x * du.y;
	let scale = du.length().max(dv.length()).max(1.0);
	if !determinant.is_finite() || determinant.abs() <= scale * scale * 1.0e-14 {
		return None;
	}
	let solution = DVec2::new((residual.x * dv.y - dv.x * residual.y) / determinant, (du.x * residual.y - residual.x * du.y) / determinant);
	solution.is_finite().then_some(solution)
}

#[derive(Clone, Copy, Debug)]
enum FaceChart {
	Curved(MetricMap),
	Planar(PlanarChart),
}

impl FaceChart {
	fn from_face(face: &TrimmedFace) -> Option<Self> {
		if face.surface.is_planar() {
			PlanarChart::from_face(face).map(Self::Planar)
		} else {
			Some(Self::Curved(MetricMap::from_face(face)))
		}
	}

	fn map_uv(self, face: &TrimmedFace, uv: DVec2) -> Option<Point2<f64>> {
		match self {
			Self::Curved(metric) => Some(metric.map(uv)),
			Self::Planar(chart) => chart.map_uv(face, uv),
		}
		.map(spade::mitigate_underflow)
	}

	fn map_boundary(self, face: &TrimmedFace, boundary: &BoundaryVertex) -> Option<Point2<f64>> {
		match self {
			Self::Curved(metric) => Some(metric.map(boundary.uv)),
			Self::Planar(chart) => chart.map_position(boundary.position).or_else(|| chart.map_uv(face, boundary.uv)),
		}
		.map(spade::mitigate_underflow)
	}

	fn unmap(self, face: &TrimmedFace, point: Point2<f64>) -> Option<DVec2> {
		match self {
			Self::Curved(metric) => metric.unmap(point),
			Self::Planar(chart) => chart.unmap(face, point),
		}
	}
}

#[derive(Clone, Copy, Debug)]
struct ParametricVertex {
	uv: DVec2,
	metric: Point2<f64>,
	boundary_position: Option<DVec3>,
	boundary_occurrences: [Option<BoundaryOccurrence>; 2],
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct BoundaryOccurrence {
	loop_index: u32,
	edge_index: u32,
	occurrence_index: u32,
}

impl ParametricVertex {
	fn add_boundary_occurrence(&mut self, occurrence: BoundaryOccurrence) -> Result<(), Error> {
		if self.boundary_occurrences.contains(&Some(occurrence)) {
			return Ok(());
		}
		if let Some(slot) = self.boundary_occurrences.iter_mut().find(|slot| slot.is_none()) {
			*slot = Some(occurrence);
			Ok(())
		} else {
			Err(Error::TriangulationFailed)
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct BoundarySegmentId {
	occurrence: BoundaryOccurrence,
	canonical_segment_index: u32,
}

#[derive(Clone, Copy, Debug)]
struct BoundaryContractPoint {
	uv: DVec2,
	position: DVec3,
}

#[derive(Clone, Copy, Debug)]
enum FaceBoundarySegment {
	Geometric { first: BoundaryContractPoint, second: BoundaryContractPoint, expected_direction: i8 },
	Collapsed { first: BoundaryContractPoint, second: BoundaryContractPoint },
}

impl FaceBoundarySegment {
	fn points(self) -> [BoundaryContractPoint; 2] {
		match self {
			Self::Geometric { first, second, .. } | Self::Collapsed { first, second } => [first, second],
		}
	}

	fn is_collapsed(self) -> bool {
		matches!(self, Self::Collapsed { .. })
	}

	fn expected_direction(self) -> Option<i8> {
		match self {
			Self::Geometric { expected_direction, .. } => Some(expected_direction),
			Self::Collapsed { .. } => None,
		}
	}
}

#[derive(Clone, Debug)]
struct BoundaryOccurrenceRefinement {
	occurrence: BoundaryOccurrence,
	direction: EdgeOccurrenceDirection,
	points: Vec<BoundaryContractPoint>,
}

#[derive(Clone, Debug)]
struct FaceBoundaryContract {
	segments: BTreeMap<BoundarySegmentId, FaceBoundarySegment>,
}

impl HasPosition for ParametricVertex {
	type Scalar = f64;

	fn position(&self) -> Point2<Self::Scalar> {
		self.metric
	}
}

type FaceTriangulation = ConstrainedDelaunayTriangulation<ParametricVertex>;

#[derive(Clone, Debug)]
struct MeshedFace {
	index: u32,
	tshape_id: u64,
	vertices: Vec<DVec3>,
	uvs: Vec<DVec2>,
	normals: Vec<DVec3>,
	indices: Vec<u32>,
	/// Canonical edge sequences augmented by a face-local self-seam
	/// refinement. Ordinary inter-face edges are never refined here.
	refined_edges: BTreeMap<u32, Vec<DVec3>>,
	boundary_refinements: Vec<BoundaryOccurrenceRefinement>,
	/// Vertices in a topology-forced boundary transition whose aspect cannot
	/// be improved without moving canonical trim samples.
	quality_exempt_vertices: BTreeSet<u32>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct RequestMeshTotals {
	vertices: usize,
	triangles: usize,
	indices: usize,
	payload_bytes: usize,
}

#[derive(Default)]
struct RequestMeshBudget {
	totals: Mutex<RequestMeshTotals>,
}

#[derive(Default)]
struct RustMeshProgressState {
	started_faces: usize,
	completed_faces: usize,
}

struct RustMeshProgress {
	total_faces: usize,
	state: Mutex<RustMeshProgressState>,
}

impl RustMeshProgress {
	fn new(total_faces: usize) -> Self {
		Self { total_faces, state: Mutex::new(RustMeshProgressState::default()) }
	}

	fn face_started(&self, progress: &ffi::CancellationToken) -> Result<(), Error> {
		check_cancelled(progress)?;
		let mut state = self.state.lock().map_err(|_| Error::TriangulationFailed)?;
		state.started_faces = state.started_faces.saturating_add(1).min(self.total_faces);
		self.publish(&state, progress);
		Ok(())
	}

	fn face_completed(&self, progress: &ffi::CancellationToken) -> Result<(), Error> {
		check_cancelled(progress)?;
		let mut state = self.state.lock().map_err(|_| Error::TriangulationFailed)?;
		state.completed_faces = state.completed_faces.saturating_add(1).min(state.started_faces);
		self.publish(&state, progress);
		Ok(())
	}

	fn publish(&self, state: &RustMeshProgressState, progress: &ffi::CancellationToken) {
		if self.total_faces == 0 {
			return;
		}
		let total = self.total_faces as f64;
		let completed = RUST_MESH_PROGRESS_START + RUST_MESH_PROGRESS_STARTED_SPAN * state.started_faces as f64 / total + RUST_MESH_PROGRESS_COMPLETED_SPAN * state.completed_faces as f64 / total;
		// All calls are serialized by `state`, including the store, so parallel
		// face completion cannot publish an older value after a newer one.
		ffi::rust_progress_set(progress, completed);
	}
}

impl RequestMeshBudget {
	fn admit(&self, face: &MeshedFace) -> Result<(), Error> {
		let triangles = face.indices.len() / 3;
		let vertex_bytes = checked_mul_resource(face.vertices.len(), 48, "tessellation request payload size overflowed")?;
		let triangle_bytes = checked_mul_resource(triangles, 20, "tessellation request payload size overflowed")?;
		let face_bytes = checked_add_resource(vertex_bytes, triangle_bytes, "tessellation request payload size overflowed")?;
		self.admit_counts(face.vertices.len(), triangles, face.indices.len(), face_bytes)
	}

	fn admit_counts(&self, face_vertices: usize, face_triangles: usize, face_indices: usize, face_payload_bytes: usize) -> Result<(), Error> {
		let mut totals = self.totals.lock().map_err(|_| resource_limit("tessellation request resource accounting failed"))?;
		let vertices = checked_add_resource(totals.vertices, face_vertices, "tessellation request vertex count overflowed")?;
		let triangles = checked_add_resource(totals.triangles, face_triangles, "tessellation request triangle count overflowed")?;
		let indices = checked_add_resource(totals.indices, face_indices, "tessellation request index count overflowed")?;
		let payload_bytes = checked_add_resource(totals.payload_bytes, face_payload_bytes, "tessellation request payload size overflowed")?;
		if vertices > MAXIMUM_REQUEST_VERTICES || triangles > MAXIMUM_REQUEST_TRIANGLES || indices > MAXIMUM_REQUEST_INDICES || payload_bytes > MAXIMUM_REQUEST_PAYLOAD_BYTES {
			return Err(resource_limit("tessellation request exceeded aggregate mesh resource limits"));
		}
		*totals = RequestMeshTotals { vertices, triangles, indices, payload_bytes };
		Ok(())
	}

	#[cfg(feature = "test-support")]
	fn snapshot(&self) -> Result<RequestMeshTotals, Error> {
		self.totals.lock().map(|totals| *totals).map_err(|_| resource_limit("tessellation request resource accounting failed"))
	}
}

impl FaceBoundaryContract {
	fn from_face(face: &TrimmedFace, refinements: &[BoundaryOccurrenceRefinement], progress: &ffi::CancellationToken) -> Result<Self, Error> {
		let mut segments = BTreeMap::new();
		let mut occurrence_directions = BTreeMap::new();
		for (loop_index, trim_loop) in face.loops.iter().enumerate() {
			cancellation_checkpoint(progress, loop_index)?;
			let loop_index = u32::try_from(loop_index).map_err(|_| Error::TriangulationFailed)?;
			let mut signed_area = 0.0;
			for (vertex_index, (first, second)) in trim_loop.vertices.iter().zip(trim_loop.vertices.iter().cycle().skip(1)).take(trim_loop.vertices.len()).enumerate() {
				cancellation_checkpoint(progress, vertex_index)?;
				signed_area += first.uv.perp_dot(second.uv);
			}
			if !signed_area.is_finite() || signed_area == 0.0 {
				return Err(Error::TriangulationFailed);
			}
			for index in 0..trim_loop.vertices.len() {
				cancellation_checkpoint(progress, index)?;
				let first = trim_loop.vertices[index];
				let second = trim_loop.vertices[(index + 1) % trim_loop.vertices.len()];
				let occurrence = BoundaryOccurrence { loop_index, edge_index: first.edge_index, occurrence_index: first.edge_occurrence_index };
				match occurrence_directions.insert(occurrence, first.edge_occurrence_direction) {
					Some(existing) if existing != first.edge_occurrence_direction => return Err(Error::TriangulationFailed),
					_ => {}
				}
				let canonical_segment_index = match first.edge_occurrence_direction {
					EdgeOccurrenceDirection::Forward => first.edge_sample_index,
					EdgeOccurrenceDirection::Reversed => first.edge_sample_index.checked_sub(1).ok_or(Error::TriangulationFailed)?,
				};
				let id = BoundarySegmentId { occurrence, canonical_segment_index };
				let first = BoundaryContractPoint { uv: first.uv, position: first.position };
				let second = BoundaryContractPoint { uv: second.uv, position: second.position };
				// Every source segment is stored in the exact trim traversal order.
				// Inner loops naturally have the opposite chart winding, but the
				// incident triangle must still follow that stored traversal. Comparing
				// against the outer-loop winding a second time reverses valid holes.
				let segment = if point_key(first.position) == point_key(second.position) { FaceBoundarySegment::Collapsed { first, second } } else { FaceBoundarySegment::Geometric { first, second, expected_direction: 1 } };
				if segments.insert(id, segment).is_some() {
					return Err(Error::TriangulationFailed);
				}
			}
		}

		let mut refined_edge_sequences = BTreeMap::<u32, Vec<PointKey>>::new();
		for (refinement_index, refinement) in refinements.iter().enumerate() {
			cancellation_checkpoint(progress, refinement_index)?;
			if refinement.points.len() < 2 || refinement.points.iter().any(|point| !point.uv.is_finite() || !point.position.is_finite()) {
				return Err(Error::TriangulationFailed);
			}
			let source_ids = segments.keys().filter(|id| id.occurrence == refinement.occurrence).copied().collect::<Vec<_>>();
			if source_ids.is_empty() {
				return Err(Error::TriangulationFailed);
			}
			let Some(source_direction) = occurrence_directions.get(&refinement.occurrence).copied() else {
				return Err(Error::TriangulationFailed);
			};
			if source_direction != refinement.direction {
				return Err(Error::TriangulationFailed);
			}
			// Refinement point sequences are canonical-edge ordered rather than
			// trim-traversal ordered, so only the occurrence orientation determines
			// the expected local direction.
			let expected_direction = if refinement.direction == EdgeOccurrenceDirection::Forward { 1 } else { -1 };
			let source_points = source_ids.iter().flat_map(|id| segments[id].points()).map(|point| point_key(point.position)).collect::<BTreeSet<_>>();
			if source_points.iter().any(|source| !refinement.points.iter().any(|point| point_key(point.position) == *source)) {
				return Err(Error::TriangulationFailed);
			}
			for id in source_ids {
				segments.remove(&id);
			}
			for (index, points) in refinement.points.windows(2).enumerate() {
				let canonical_segment_index = u32::try_from(index).map_err(|_| Error::TriangulationFailed)?;
				let id = BoundarySegmentId { occurrence: refinement.occurrence, canonical_segment_index };
				let [first, second] = [points[0], points[1]];
				let segment = if point_key(first.position) == point_key(second.position) { FaceBoundarySegment::Collapsed { first, second } } else { FaceBoundarySegment::Geometric { first, second, expected_direction } };
				if segments.insert(id, segment).is_some() {
					return Err(Error::TriangulationFailed);
				}
			}

			let sequence = refinement.points.iter().map(|point| point_key(point.position)).collect::<Vec<_>>();
			match refined_edge_sequences.get(&refinement.occurrence.edge_index) {
				Some(existing) if existing != &sequence => return Err(Error::TriangulationFailed),
				Some(_) => {}
				None => {
					refined_edge_sequences.insert(refinement.occurrence.edge_index, sequence);
				}
			}
		}
		Ok(Self { segments })
	}

	fn audit(&self, mesh: &MeshedFace, progress: &ffi::CancellationToken) -> Result<(), Error> {
		#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
		struct ParameterKey(u64, u64);

		#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
		struct BoundaryPointKey {
			parameter: ParameterKey,
			position: PointKey,
		}

		fn parameter_key(uv: DVec2) -> ParameterKey {
			fn coordinate_key(value: f64) -> u64 {
				if value == 0.0 {
					0
				} else {
					value.to_bits()
				}
			}
			ParameterKey(coordinate_key(uv.x), coordinate_key(uv.y))
		}

		fn boundary_point_key(point: BoundaryContractPoint) -> BoundaryPointKey {
			BoundaryPointKey { parameter: parameter_key(point.uv), position: point_key(point.position) }
		}

		fn ordered_pair<T: Ord>(first: T, second: T) -> (T, T) {
			if first <= second {
				(first, second)
			} else {
				(second, first)
			}
		}

		let mut segment_lookup = BTreeMap::<(BoundaryPointKey, BoundaryPointKey), Vec<BoundarySegmentId>>::new();
		let mut endpoints_by_position = BTreeMap::<PointKey, BTreeSet<BoundaryPointKey>>::new();
		let mut collapsed_positions = BTreeSet::new();
		for (segment_index, (id, segment)) in self.segments.iter().enumerate() {
			cancellation_checkpoint(progress, segment_index)?;
			let [first, second] = segment.points().map(boundary_point_key);
			segment_lookup.entry(ordered_pair(first, second)).or_default().push(*id);
			endpoints_by_position.entry(first.position).or_default().insert(first);
			endpoints_by_position.entry(second.position).or_default().insert(second);
			if segment.is_collapsed() {
				collapsed_positions.insert(first.position);
				collapsed_positions.insert(second.position);
			}
		}

		let endpoint_keys = endpoints_by_position.values().flat_map(BTreeSet::iter).copied().collect::<BTreeSet<_>>();
		let aliases = mesh
			.vertices
			.iter()
			.zip(&mesh.uvs)
			.enumerate()
			.map(|(index, (position, uv))| {
				cancellation_checkpoint(progress, index)?;
				let position = point_key(*position);
				let exact = BoundaryPointKey { parameter: parameter_key(*uv), position };
				let mut aliases = BTreeSet::new();
				if endpoint_keys.contains(&exact) {
					aliases.insert(exact);
				}
				if collapsed_positions.contains(&position) {
					aliases.extend(endpoints_by_position.get(&position).into_iter().flatten().copied());
				}
				Ok(aliases)
			})
			.collect::<Result<Vec<_>, Error>>()?;

		#[derive(Clone, Copy, Default)]
		struct DirectedEdgeUse {
			incidence: usize,
			balance: i8,
		}

		let mut edge_uses = BTreeMap::<(u32, u32), DirectedEdgeUse>::new();
		for (triangle_index, triangle) in mesh.indices.chunks_exact(3).enumerate() {
			cancellation_checkpoint(progress, triangle_index)?;
			for edge in 0..3 {
				let directed = (triangle[edge], triangle[(edge + 1) % 3]);
				let local_edge = ordered_pair(directed.0, directed.1);
				if local_edge.0 == local_edge.1 {
					return Err(Error::TriangulationFailed);
				}
				let edge_use = edge_uses.entry(local_edge).or_default();
				edge_use.incidence += 1;
				edge_use.balance = edge_use.balance.checked_add(if directed == local_edge { 1 } else { -1 }).ok_or(Error::TriangulationFailed)?;
			}
		}

		let mut segment_uses = BTreeMap::<BoundarySegmentId, usize>::new();
		for (edge_index, ((first, second), edge_use)) in edge_uses.into_iter().enumerate() {
			cancellation_checkpoint(progress, edge_index)?;
			let first_aliases = aliases.get(first as usize).ok_or(Error::TriangulationFailed)?;
			let second_aliases = aliases.get(second as usize).ok_or(Error::TriangulationFailed)?;
			let matches = first_aliases.iter().flat_map(|first| second_aliases.iter().map(move |second| ordered_pair(*first, *second))).filter_map(|points| segment_lookup.get(&points)).flatten().copied().collect::<BTreeSet<_>>();
			if matches.is_empty() && edge_use.incidence == 2 && edge_use.balance == 0 {
				continue;
			}
			let Some(id) = matches.first().copied().filter(|_| matches.len() == 1) else {
				if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
					eprintln!("custom tessellation face {} boundary contract rejected local edge {first}-{second}: incidence {}, directed balance {}, typed matches {matches:?}, uvs {:?}-{:?}, points {:?}-{:?}", mesh.index, edge_use.incidence, edge_use.balance, mesh.uvs[first as usize], mesh.uvs[second as usize], mesh.vertices[first as usize], mesh.vertices[second as usize]);
				}
				return Err(Error::TriangulationFailed);
			};
			let segment = self.segments[&id];
			if edge_use.incidence != 1 || edge_use.balance.abs() != 1 || segment.is_collapsed() {
				if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
					eprintln!("custom tessellation face {} boundary contract rejected typed segment {id:?} on local edge {first}-{second}: incidence {}, directed balance {}, segment {segment:?}", mesh.index, edge_use.incidence, edge_use.balance);
				}
				return Err(Error::TriangulationFailed);
			}
			let [segment_first, segment_second] = segment.points().map(boundary_point_key);
			let local_order = if first_aliases.contains(&segment_first) && second_aliases.contains(&segment_second) {
				1
			} else if first_aliases.contains(&segment_second) && second_aliases.contains(&segment_first) {
				-1
			} else {
				return Err(Error::TriangulationFailed);
			};
			if segment.expected_direction().is_some_and(|expected| edge_use.balance * local_order != expected) {
				if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
					eprintln!("custom tessellation face {} boundary contract rejected orientation of segment {id:?} on local edge {first}-{second}: directed balance {}, local endpoint order {local_order}, expected {:?}", mesh.index, edge_use.balance, segment.expected_direction());
				}
				return Err(Error::TriangulationFailed);
			}
			*segment_uses.entry(id).or_default() += 1;
		}

		for (segment_index, (id, segment)) in self.segments.iter().enumerate() {
			cancellation_checkpoint(progress, segment_index)?;
			let expected = usize::from(!segment.is_collapsed());
			let actual = segment_uses.get(id).copied().unwrap_or(0);
			if actual != expected {
				if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
					eprintln!("custom tessellation face {} boundary contract rejected segment {id:?}: expected {expected} local uses, found {actual}, segment {segment:?}", mesh.index);
				}
				return Err(Error::TriangulationFailed);
			}
		}
		Ok(())
	}
}

pub(super) fn mesh_brep_source(data: ffi::BrepMeshSourceData, options: Tessellation, progress: &ffi::CancellationToken) -> Result<ffi::MeshData, Error> {
	if !options.deflection_linear.is_finite() || options.deflection_linear <= 0.0 || !options.deflection_angular.is_finite() || options.deflection_angular <= 0.0 {
		return Err(Error::TriangulationFailed);
	}
	let source = decode_source(data)?;
	check_cancelled(progress)?;
	let absolute_linear = source.linear_deflection;
	let faces = mesh_faces(&source.faces, absolute_linear, options.deflection_angular, options.parallel, progress)?;
	check_cancelled(progress)?;
	let result = assemble_mesh_data(faces, &source.edges, options.include_edges, progress)?;
	check_cancelled(progress)?;
	ffi::rust_progress_set(progress, 1.0);
	Ok(result)
}

fn mesh_faces(faces: &[TrimmedFace], linear: f64, angular: f64, parallel: bool, progress: &ffi::CancellationToken) -> Result<Vec<MeshedFace>, Error> {
	if let Some(cache) = FACE_MESH_CACHE.get() {
		for face in faces {
			check_cancelled(progress)?;
			let key = face_mesh_key(face, linear, angular);
			if cache.lock().unwrap_or_else(|error| error.into_inner()).entries.get(&key).is_some_and(|entry| entry.2.is_none()) {
				return Err(Error::TriangulationFailed);
			}
		}
	}
	let budget = RequestMeshBudget::default();
	let rust_progress = RustMeshProgress::new(faces.len());
	let mesh_and_admit = |face: &TrimmedFace| {
		rust_progress.face_started(progress)?;
		let mesh = cached_mesh_face(face, linear, angular, progress)?;
		budget.admit(&mesh)?;
		rust_progress.face_completed(progress)?;
		Ok(mesh)
	};
	#[cfg(not(target_arch = "wasm32"))]
	if parallel && faces.len() > 1 {
		use rayon::prelude::*;
		return bounded_face_pool()?.install(|| faces.par_iter().map(mesh_and_admit).collect());
	}
	faces.iter().map(mesh_and_admit).collect()
}

const FACE_CACHE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Default)]
struct FaceMeshCache {
	entries: HashMap<Vec<u64>, (u64, usize, Option<Arc<MeshedFace>>)>,
	bytes: usize,
	clock: u64,
	hits: usize,
	misses: usize,
}

static FACE_MESH_CACHE: OnceLock<Mutex<FaceMeshCache>> = OnceLock::new();

#[cfg(feature = "test-support")]
pub(super) fn face_cache_statistics(clear: bool) -> [usize; 4] {
	let mut cache = FACE_MESH_CACHE.get_or_init(Mutex::default).lock().unwrap_or_else(|error| error.into_inner());
	let statistics = [cache.hits, cache.misses, cache.entries.len(), cache.bytes];
	if clear {
		*cache = FaceMeshCache::default();
	}
	statistics
}

// Compare every geometric and boundary bit, never a pointer or hash alone.
// Face ordinals and native IDs are presentation labels, rebound after reuse.
fn face_mesh_key(face: &TrimmedFace, linear: f64, angular: f64) -> Vec<u64> {
	let surface = &face.surface;
	let mut key = vec![linear.to_bits(), angular.to_bits(), face.reversed as u64, surface.u_degree as u64, surface.v_degree as u64, surface.u_count as u64, surface.v_count as u64];
	for points in [&surface.poles, &surface.local_poles] {
		key.push(points.len() as u64);
		key.extend(points.iter().flat_map(|point| point.to_array().map(f64::to_bits)));
	}
	key.extend(surface.origin.to_array().map(f64::to_bits));
	for values in [&surface.weights, &surface.u_knots, &surface.v_knots] {
		key.push(values.len() as u64);
		key.extend(values.iter().map(|value| value.to_bits()));
	}
	key.extend(surface.uv_bounds.map(f64::to_bits));
	key.push(surface.approximation_error.to_bits());
	key.extend([face.collapsed_boundaries.u_at_v_min, face.collapsed_boundaries.u_at_v_max, face.collapsed_boundaries.v_at_u_min, face.collapsed_boundaries.v_at_u_max].map(u64::from));
	key.push(face.loops.len() as u64);
	for boundary in &face.loops {
		key.push(boundary.vertices.len() as u64);
		for vertex in &boundary.vertices {
			key.extend(vertex.uv.to_array().map(f64::to_bits));
			key.extend(vertex.position.to_array().map(f64::to_bits));
			key.extend([vertex.edge_index as u64, vertex.edge_sample_index as u64, vertex.edge_occurrence_index as u64, vertex.edge_occurrence_direction as u64]);
		}
	}
	key
}

fn cached_mesh_face(face: &TrimmedFace, linear: f64, angular: f64, progress: &ffi::CancellationToken) -> Result<MeshedFace, Error> {
	check_cancelled(progress)?;
	let key = face_mesh_key(face, linear, angular);
	let cache = FACE_MESH_CACHE.get_or_init(Mutex::default);
	let cached = {
		let mut cache = cache.lock().unwrap_or_else(|error| error.into_inner());
		cache.clock = cache.clock.wrapping_add(1);
		let clock = cache.clock;
		let entry = cache.entries.get_mut(&key).map(|entry| {
			entry.0 = clock;
			entry.2.clone()
		});
		if entry.is_some() {
			cache.hits += 1;
		} else {
			cache.misses += 1;
		}
		entry
	};
	if let Some(cached) = cached {
		let mut mesh = cached.ok_or(Error::TriangulationFailed)?.as_ref().clone();
		mesh.index = face.index;
		mesh.tshape_id = face.tshape_id;
		return Ok(mesh);
	}
	let result = mesh_face(face, linear, angular, progress);
	check_cancelled(progress)?;
	if result.is_ok() || matches!(result, Err(Error::TriangulationFailed)) {
		let mesh_bytes = result.as_ref().map_or(0, |mesh| mesh.vertices.len() * 24 + mesh.uvs.len() * 16 + mesh.normals.len() * 24 + mesh.indices.len() * 4 + mesh.refined_edges.values().map(|points| points.len() * 24 + 64).sum::<usize>() + mesh.boundary_refinements.iter().map(|run| run.points.len() * 40 + 64).sum::<usize>() + mesh.quality_exempt_vertices.len() * 32);
		let bytes = key.len() * 8 + mesh_bytes + 256;
		if bytes <= FACE_CACHE_BYTES {
			let mut cache = cache.lock().unwrap_or_else(|error| error.into_inner());
			if let Some((_, previous, _)) = cache.entries.remove(&key) {
				cache.bytes -= previous;
			}
			while cache.bytes + bytes > FACE_CACHE_BYTES || cache.entries.len() >= 512 {
				let Some(oldest) = cache.entries.iter().min_by_key(|(_, entry)| entry.0).map(|(key, _)| key.clone()) else {
					break;
				};
				cache.bytes -= cache.entries.remove(&oldest).unwrap().1;
			}
			cache.clock = cache.clock.wrapping_add(1);
			let clock = cache.clock;
			cache.entries.insert(key, (clock, bytes, result.as_ref().ok().map(|mesh| Arc::new(mesh.clone()))));
			cache.bytes += bytes;
		}
	}
	result
}

fn mesh_face(face: &TrimmedFace, linear: f64, angular: f64, progress: &ffi::CancellationToken) -> Result<MeshedFace, Error> {
	check_cancelled(progress)?;
	let planar = face.surface.is_planar();
	// Most planar trims get the isotropic CDT below. A small regular patch very
	// far from the origin can lose enough local chart bits that CDT insertion is
	// no longer reliable; route only that numerical corner through the tensor
	// mesher. Keeping the exception narrow preserves ordinary rigid-placement
	// equivalence for planar sweeps.
	if !planar || face.surface.needs_world_stable_planar_route() {
		let structured_mesh = mesh_structured_patch(face, linear, angular, progress).map_err(|error| {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation face {} could not build its structured patch: {error:?}", face.index);
			}
			error
		})?;
		if let Some(mesh) = structured_mesh {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation face {} used structured patch: {} vertices, {} triangles", face.index, mesh.vertices.len(), mesh.indices.len() / 3);
				diagnose_meshed_face(&mesh);
			}
			return Ok(mesh);
		}
	}
	if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
		eprintln!("custom tessellation face {} used trimmed CDT", face.index);
	}
	let chart = FaceChart::from_face(face).ok_or_else(|| {
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation face {} could not construct its metric chart", face.index);
		}
		Error::TriangulationFailed
	})?;
	let mut triangulation = FaceTriangulation::new();
	for (loop_index, trim_loop) in face.loops.iter().enumerate() {
		let loop_index = u32::try_from(loop_index).map_err(|_| Error::TriangulationFailed)?;
		let mut handles = Vec::with_capacity(trim_loop.vertices.len());
		for (index, boundary) in trim_loop.vertices.iter().enumerate() {
			let metric_position = chart.map_boundary(face, boundary).ok_or_else(|| {
				if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
					eprintln!("custom tessellation face {} could not map boundary vertex {index} of loop {loop_index}", face.index);
				}
				Error::TriangulationFailed
			})?;
			let boundary_occurrences = boundary_vertex_occurrences(loop_index, trim_loop, index);
			let handle = if let Some(existing) = triangulation.locate_vertex(metric_position) {
				let existing = existing.fix();
				for occurrence in boundary_occurrences.into_iter().flatten() {
					triangulation.vertex_data_mut(existing).add_boundary_occurrence(occurrence).inspect_err(|_| {
						if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
							eprintln!("custom tessellation face {} boundary vertex {index} of loop {loop_index} exceeded occurrence capacity at metric position {metric_position:?}", face.index);
						}
					})?;
				}
				existing
			} else {
				triangulation.insert(ParametricVertex { uv: boundary.uv, metric: metric_position, boundary_position: Some(boundary.position), boundary_occurrences }).map_err(|error| {
					if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
						eprintln!("custom tessellation face {} could not insert boundary vertex {index} of loop {loop_index} at metric position {metric_position:?}: {error:?}", face.index);
					}
					Error::TriangulationFailed
				})?
			};
			handles.push(handle);
		}
		for index in 0..handles.len() {
			let first = handles[index];
			let second = handles[(index + 1) % handles.len()];
			// Collapsed canonical segments need no constraint; the boundary audit
			// still requires every non-collapsed segment exactly once.
			if first == second && point_key(trim_loop.vertices[index].position) == point_key(trim_loop.vertices[(index + 1) % handles.len()].position) {
				continue;
			}
			if first == second || !triangulation.can_add_constraint(first, second) {
				if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
					let first_boundary = &trim_loop.vertices[index];
					let second_boundary = &trim_loop.vertices[(index + 1) % trim_loop.vertices.len()];
					let intersections = (first != second).then(|| LineIntersectionIterator::new_from_handles(&triangulation, first, second).collect::<Vec<_>>());
					eprintln!("custom tessellation face {} could not constrain boundary segment {index} of loop {loop_index}: identical={}, first={}, second={}, uvs {:?}-{:?}, positions {:?}-{:?}, metric {:?}-{:?}, intersections {intersections:?}", face.index, first == second, first.index(), second.index(), first_boundary.uv, second_boundary.uv, first_boundary.position, second_boundary.position, triangulation.vertex(first).position(), triangulation.vertex(second).position());
					if let Some(intersections) = intersections {
						for intersection in intersections {
							if let Intersection::EdgeIntersection(edge) | Intersection::EdgeOverlap(edge) = intersection {
								let [edge_first, edge_second] = edge.vertices();
								eprintln!("custom tessellation blocking edge constraint={} handles {}-{}, metric {:?}-{:?}, uvs {:?}-{:?}", edge.is_constraint_edge(), edge_first.fix().index(), edge_second.fix().index(), edge_first.position(), edge_second.position(), edge_first.data().uv, edge_second.data().uv);
							}
						}
					}
				}
				return Err(Error::TriangulationFailed);
			}
			triangulation.add_constraint(first, second);
		}
	}
	if triangulation.num_vertices() < 3 {
		return Err(Error::TriangulationFailed);
	}
	let insertion_domain = InsertionDomain::from_face(face, chart, linear).ok_or_else(|| {
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation face {} could not construct its insertion domain", face.index);
		}
		Error::TriangulationFailed
	})?;

	if planar {
		triangulation = seed_best_planar_lattice(face, chart, linear, &insertion_domain, triangulation, progress)?;
	} else {
		seed_structured_patch(face, chart, &insertion_domain, &mut triangulation)?;
		seed_boundary_collar(face, chart, &insertion_domain, &mut triangulation)?;
		seed_metric_lattice(face, chart, linear, &insertion_domain, &mut triangulation)?;
	}

	// Seed broad trimmed regions before exact-error refinement. A boundary-only
	// Delaunay mesh of a circle or other many-sided convex face otherwise tends
	// to contain long, visually conspicuous diagonals. The seed is unconstrained
	// and therefore cannot disturb the canonical samples shared by adjacent faces.
	if !planar && triangulation.num_vertices() > 8 {
		let outer = &face.loops[0].vertices;
		let center = outer.iter().map(|vertex| vertex.uv).sum::<DVec2>() / outer.len() as f64;
		if point_in_trim(center, &face.loops) {
			if triangulation.num_vertices() >= MAXIMUM_CDT_FACE_VERTICES {
				return Err(resource_limit("tessellation face exceeded the CDT vertex limit"));
			}
			insert_interior_vertex(face, &mut triangulation, center, chart, &insertion_domain);
		}
	}

	let usable_linear = (linear - face.surface.approximation_error).max(linear * 0.20).max(1.0e-10);
	let mut required_passes = 0;
	let mut quality_passes = 0;
	let mut required_insertions = 0;
	let mut quality_insertions = 0;
	loop {
		if progress.is_cancelled() {
			return Err(Error::Cancelled);
		}
		let mut candidates = refinement_candidates(face, &triangulation, usable_linear, angular.max(1.0e-3), progress).map_err(|error| {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation face {} could not evaluate trimmed CDT refinement candidates: {error:?}", face.index);
			}
			error
		})?;
		if planar {
			candidates.retain(|candidate| candidate.required);
		}
		if candidates.is_empty() {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation face {} refinement stopped: no candidates", face.index);
			}
			break;
		}
		if triangulation.num_vertices() >= MAXIMUM_CDT_FACE_VERTICES {
			return Err(resource_limit("tessellation face exceeded the CDT vertex limit"));
		}
		let refining_required_error = candidates.iter().any(|candidate| candidate.required);
		candidates.retain(|candidate| candidate.required == refining_required_error);
		candidates.sort_by(|first, second| second.required.cmp(&first.required).then_with(|| second.linear_required.cmp(&first.linear_required)).then_with(|| second.score.total_cmp(&first.score)).then_with(|| first.uv.x.total_cmp(&second.uv.x)).then_with(|| first.uv.y.total_cmp(&second.uv.y)));
		let (passes, insertion_count, maximum_passes, maximum_total, maximum_per_pass) = if refining_required_error { (&mut required_passes, &mut required_insertions, MAXIMUM_REQUIRED_REFINEMENT_PASSES, MAXIMUM_REQUIRED_INSERTIONS, MAXIMUM_REQUIRED_INSERTIONS_PER_PASS) } else { (&mut quality_passes, &mut quality_insertions, MAXIMUM_QUALITY_REFINEMENT_PASSES, MAXIMUM_QUALITY_INSERTIONS, MAXIMUM_QUALITY_INSERTIONS_PER_PASS) };
		if *passes >= maximum_passes || *insertion_count >= maximum_total {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation face {} refinement stopped: pass/insertion limit with {} candidates", face.index, candidates.len());
			}
			break;
		}
		*passes += 1;
		let before = triangulation.num_vertices();
		let room = MAXIMUM_CDT_FACE_VERTICES - before;
		let maximum_insertions = maximum_per_pass.min(maximum_total - *insertion_count);
		let mut seen_candidates = BTreeSet::new();
		for (index, candidate) in candidates.into_iter().filter(|candidate| seen_candidates.insert((candidate.uv.x.to_bits(), candidate.uv.y.to_bits()))).take(maximum_insertions.min(room)).enumerate() {
			if index.is_multiple_of(CANCELLATION_CHECK_INTERVAL) {
				check_cancelled(progress)?;
			}
			insert_interior_vertex(face, &mut triangulation, candidate.uv, chart, &insertion_domain);
		}
		let inserted = triangulation.num_vertices() - before;
		*insertion_count += inserted;
		if inserted == 0 {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation face {} refinement stopped: all selected candidates were numerical duplicates", face.index);
			}
			break;
		}
	}
	if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
		eprintln!("custom tessellation face {} refinement totals: required {required_passes} passes/{required_insertions} insertions, quality {quality_passes} passes/{quality_insertions} insertions, {} vertices", face.index, triangulation.num_vertices());
	}
	let mesh = build_face_mesh(face, &triangulation, progress).map_err(|error| {
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation face {} could not build its trimmed CDT mesh: {error:?}", face.index);
		}
		error
	})?;
	let satisfies_tolerances = mesh_satisfies_tolerances(face, &mesh, linear, angular, progress).map_err(|error| {
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation face {} could not validate its trimmed CDT mesh tolerances: {error:?}", face.index);
		}
		error
	})?;
	if !satisfies_tolerances {
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation face {} rejected its trimmed CDT mesh because it exceeds tessellation tolerances", face.index);
		}
		return Err(Error::TriangulationFailed);
	}
	if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
		eprintln!("custom tessellation face {} trimmed CDT result: {} vertices, {} triangles", face.index, mesh.vertices.len(), mesh.indices.len() / 3);
		diagnose_meshed_face(&mesh);
	}
	Ok(mesh)
}

fn boundary_vertex_occurrences(loop_index: u32, trim_loop: &TrimLoop, index: usize) -> [Option<BoundaryOccurrence>; 2] {
	let current = &trim_loop.vertices[index];
	let previous = &trim_loop.vertices[(index + trim_loop.vertices.len() - 1) % trim_loop.vertices.len()];
	let next = &trim_loop.vertices[(index + 1) % trim_loop.vertices.len()];
	let current_occurrence = (point_key(current.position) != point_key(next.position)).then_some(BoundaryOccurrence { loop_index, edge_index: current.edge_index, occurrence_index: current.edge_occurrence_index });
	let previous_occurrence = (point_key(previous.position) != point_key(current.position)).then_some(BoundaryOccurrence { loop_index, edge_index: previous.edge_index, occurrence_index: previous.edge_occurrence_index });
	[current_occurrence, previous_occurrence.filter(|previous| Some(*previous) != current_occurrence)]
}

fn diagnose_meshed_face(mesh: &MeshedFace) {
	let mut worst = (1.0, 0, [0_u32; 3], [0.0; 3]);
	let mut worst_aspect = (0.0, 0, [0_u32; 3]);
	for (triangle_index, triangle) in mesh.indices.chunks_exact(3).enumerate() {
		let triangle = [triangle[0], triangle[1], triangle[2]];
		let points = triangle.map(|index| mesh.vertices[index as usize]);
		let Some(geometric_normal) = (points[1] - points[0]).cross(points[2] - points[0]).try_normalize() else {
			continue;
		};
		let alignment = triangle.map(|index| geometric_normal.dot(mesh.normals[index as usize]));
		let minimum = alignment.into_iter().fold(1.0, f64::min);
		if minimum < worst.0 {
			worst = (minimum, triangle_index, triangle, alignment);
		}
		let aspect = triangle_aspect(points);
		if aspect > worst_aspect.0 {
			worst_aspect = (aspect, triangle_index, triangle);
		}
	}
	eprintln!("custom tessellation face {} worst normal alignment: {:.12}, triangle {}, indices {:?}, alignments {:?}, points {:?}", mesh.index, worst.0, worst.1, worst.2, worst.3, worst.2.map(|index| mesh.vertices[index as usize]));
	eprintln!("custom tessellation face {} worst aspect: {:.12}, triangle {}, indices {:?}, uvs {:?}, points {:?}", mesh.index, worst_aspect.0, worst_aspect.1, worst_aspect.2, worst_aspect.2.map(|index| mesh.uvs[index as usize]), worst_aspect.2.map(|index| mesh.vertices[index as usize]));
}

fn mesh_satisfies_tolerances(face: &TrimmedFace, mesh: &MeshedFace, linear: f64, angular: f64, progress: &ffi::CancellationToken) -> Result<bool, Error> {
	check_cancelled(progress)?;
	if mesh.vertices.len() != mesh.uvs.len() || mesh.vertices.len() != mesh.normals.len() || mesh.indices.is_empty() || !mesh.indices.len().is_multiple_of(3) {
		return Err(Error::TriangulationFailed);
	}
	for (vertex_index, ((point, uv), normal)) in mesh.vertices.iter().zip(&mesh.uvs).zip(&mesh.normals).enumerate() {
		cancellation_checkpoint(progress, vertex_index)?;
		if !point.is_finite() || !uv.is_finite() || !normal.is_finite() {
			return Err(Error::TriangulationFailed);
		}
	}
	FaceBoundaryContract::from_face(face, &mesh.boundary_refinements, progress)?.audit(mesh, progress)?;

	let usable_linear = (linear - face.surface.approximation_error).max(linear * 0.20).max(1.0e-10);
	let usable_angular = angular.max(1.0e-3);
	// Boundary samples originate in the exact-kernel edge discretizer and can
	// land a few percent above the nominal sagitta after the adjacent surface
	// chart is reconstructed. Keep that bounded numerical allowance explicit;
	// the audit still rejects larger violations and every refinement decision
	// uses the strict requested tolerance.
	let acceptance_slack = 1.05;
	let resource_bounded_aspect = resource_bounded_structured_aspect(face).unwrap_or(0.0);
	let maximum_physical_aspect = MAXIMUM_PHYSICAL_ASPECT.max(resource_bounded_aspect);
	let maximum_hard_physical_aspect = MAXIMUM_HARD_PHYSICAL_ASPECT.max(resource_bounded_aspect);
	if resource_bounded_aspect > 0.0 && std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
		eprintln!("custom tessellation face {} resource-bounded structured aspect {:.12e}", face.index, resource_bounded_aspect);
	}
	let boundary_vertex_keys = face.loops.iter().flat_map(|trim_loop| trim_loop.vertices.iter().map(|vertex| point_key(vertex.position))).chain(mesh.refined_edges.values().flatten().copied().map(point_key)).collect::<BTreeSet<_>>();
	let mut normal_angles = Vec::with_capacity(mesh.indices.len() / 3);
	let mut physical_aspects = Vec::with_capacity(mesh.indices.len() / 3);
	let mut worst_aspect = (0.0, [0_usize; 3], [DVec2::ZERO; 3], [DVec3::ZERO; 3]);
	for (triangle_index, triangle) in mesh.indices.chunks_exact(3).enumerate() {
		cancellation_checkpoint(progress, triangle_index)?;
		if triangle.iter().any(|index| *index as usize >= mesh.vertices.len()) {
			return Err(Error::TriangulationFailed);
		}
		let indices = [triangle[0] as usize, triangle[1] as usize, triangle[2] as usize];
		let positions = indices.map(|index| mesh.vertices[index]);
		let uvs = indices.map(|index| mesh.uvs[index]);
		let geometric_normal = (positions[1] - positions[0]).cross(positions[2] - positions[0]).try_normalize().ok_or(Error::TriangulationFailed)?;
		let center_uv = regularized_triangle_center_uv(face, uvs);
		let center = face.surface.evaluate(center_uv).ok_or(Error::TriangulationFailed)?;
		let center_deviation = point_triangle_distance(center.position, positions);
		if center_deviation > usable_linear * acceptance_slack {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation face {} rejected triangle {:?} at {:?}: center deviation {center_deviation:.12e} > {usable_linear:.12e}", face.index, indices, center_uv);
			}
			return Ok(reject_mesh_tolerance(face, "triangle interior exceeds linear deflection"));
		}
		// A triangle with two vertices on a trim belongs to the canonical shared-
		// edge collar. At a parametric cusp (notably an airfoil trailing edge),
		// the surface normal at the trim is singular and its limiting direction can
		// rotate by nearly ninety degrees over an arbitrarily small, linearly flat
		// cell. No finite tessellation can satisfy a point-normal bound there. Keep
		// the exact boundary and strict linear audit, but reserve angular statistics
		// for the regular surface interior where the differential is meaningful.
		let boundary_layer = uvs.iter().filter(|uv| near_parametric_boundary(face, **uv)).count() >= 2;
		let has_subdeflection_cusp = subdeflection_cusp(face, uvs, positions, usable_linear);
		let touches_exact_boundary = positions.iter().any(|position| boundary_vertex_keys.contains(&point_key(*position)));
		let angular_exempt = touches_exact_boundary || triangle.iter().any(|index| mesh.quality_exempt_vertices.contains(index)) || uvs.iter().any(|uv| collapsed_parameter_axis(face, *uv).is_some()) || face.surface.triangle_crosses_nonsmooth_knot(uvs) || has_subdeflection_cusp || boundary_layer;
		let has_microscopic_exact_boundary_edge = (0..3).any(|edge| {
			let next = (edge + 1) % 3;
			boundary_vertex_keys.contains(&point_key(positions[edge])) && boundary_vertex_keys.contains(&point_key(positions[next])) && positions[edge].distance(positions[next]) <= usable_linear * 0.10
		});
		let physical_edges = [positions[0].distance(positions[1]), positions[1].distance(positions[2]), positions[2].distance(positions[0])];
		let has_subdeflection_microscopic_edge = physical_edges.into_iter().fold(f64::INFINITY, f64::min) <= usable_linear * 0.10 && physical_edges.into_iter().fold(0.0, f64::max) <= usable_linear * 2.0;
		// Resource-bounded anisotropic patches require elongated exact-boundary transition cells.
		// Exempt only those cells from the aspect ceiling; linear and topology audits still apply.
		let resource_bounded_boundary_transition = resource_bounded_aspect > 0.0 && touches_exact_boundary;
		let hard_aspect_exempt = uvs.iter().any(|uv| collapsed_parameter_axis(face, *uv).is_some()) || has_microscopic_exact_boundary_edge || has_subdeflection_cusp || has_subdeflection_microscopic_edge || resource_bounded_boundary_transition || boundary_layer;
		let distribution_aspect_exempt = angular_exempt || has_subdeflection_microscopic_edge;
		let expected_normal = oriented_surface_normal_from_sample(face, center_uv, center).ok_or(Error::TriangulationFailed)?;
		let angle = geometric_normal.dot(expected_normal).clamp(-1.0, 1.0).acos();
		if !angular_exempt && angle > usable_angular * 3.0 * acceptance_slack {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation face {} rejected triangle {:?} at {:?}: normal angle {angle:.12e} > 3 * {usable_angular:.12e}, uvs {uvs:?}, points {positions:?}", face.index, indices, center_uv);
			}
			return Ok(reject_mesh_tolerance(face, "triangle interior exceeds angular deflection"));
		}
		if !angular_exempt {
			normal_angles.push(angle);
		}
		// A collapsed chart cell converges to one geometric pole. Likewise, a
		// canonical edge segment or its structured-grid continuation far below the
		// requested chord tolerance can force a local needle at a sharp cusp;
		// removing it would crack the shared edge or the regular grid. Exempt only
		// those bounded singular cases. Boundary collars otherwise obey the same
		// hard aspect limit as the interior, so a rejected zipper is retried with a
		// better density transition. Linear error remains fully enforced.
		if !hard_aspect_exempt {
			let physical_aspect = triangle_aspect(positions);
			if physical_aspect > worst_aspect.0 {
				worst_aspect = (physical_aspect, indices, uvs, positions);
			}
			if !physical_aspect.is_finite() {
				if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
					eprintln!("custom tessellation face {} produced a non-finite physical aspect, indices {:?}, uvs {:?}, points {:?}", face.index, indices, uvs, positions);
				}
				return Ok(reject_mesh_tolerance(face, "ordinary triangle has non-finite physical aspect"));
			}
			if !distribution_aspect_exempt {
				physical_aspects.push(physical_aspect);
			}
		}
		for edge in 0..3 {
			let next = (edge + 1) % 3;
			let midpoint_uv = regularized_edge_midpoint_uv(face, uvs[edge], uvs[next]);
			let midpoint = face.surface.evaluate_position(midpoint_uv).ok_or(Error::TriangulationFailed)?;
			let edge_deviation = point_segment_distance(midpoint, positions[edge], positions[next]);
			if edge_deviation > usable_linear * acceptance_slack {
				if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
					eprintln!("custom tessellation face {} rejected edge {:?}-{:?} at {:?}: deviation {edge_deviation:.12e} > {usable_linear:.12e}, endpoint uvs {:?}-{:?}, points {:?}-{:?}", face.index, indices[edge], indices[next], midpoint_uv, uvs[edge], uvs[next], positions[edge], positions[next]);
				}
				return Ok(reject_mesh_tolerance(face, "triangle edge exceeds linear deflection"));
			}
		}
	}
	check_cancelled(progress)?;
	if let Some(angular_p99) = percentile(&mut normal_angles, 0.99) {
		if angular_p99 > usable_angular * acceptance_slack {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation face {} facet-to-surface normal angle p99 {angular_p99:.12e} > {:.12e}", face.index, usable_angular * acceptance_slack);
			}
			return Ok(reject_mesh_tolerance(face, "angular-deflection violations are not confined to boundary or singular cells"));
		}
	}
	if worst_aspect.0 > maximum_hard_physical_aspect * acceptance_slack {
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation face {} ordinary worst aspect {:.12e} > {maximum_hard_physical_aspect:.12e}, indices {:?}, uvs {:?}, points {:?}", face.index, worst_aspect.0, worst_aspect.1, worst_aspect.2, worst_aspect.3);
		}
		return Ok(reject_mesh_tolerance(face, "ordinary physical triangle aspect exceeds hard quality limit"));
	}
	if let Some(aspect_p99) = percentile(&mut physical_aspects, 0.99) {
		if aspect_p99 > maximum_hard_physical_aspect * acceptance_slack {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation face {} ordinary aspect p99 {aspect_p99:.12e} > {maximum_hard_physical_aspect:.12e}; worst {:.12e}, indices {:?}, uvs {:?}, points {:?}", face.index, worst_aspect.0, worst_aspect.1, worst_aspect.2, worst_aspect.3);
			}
			return Ok(reject_mesh_tolerance(face, "ordinary physical triangle aspect p99 exceeds hard quality limit"));
		}
	}
	if let Some(aspect_p95) = percentile(&mut physical_aspects, 0.95) {
		if !aspect_p95.is_finite() || aspect_p95 > maximum_physical_aspect * acceptance_slack {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation face {} ordinary aspect p95 {aspect_p95:.12e} > {maximum_physical_aspect:.12e}; worst {:.12e}, indices {:?}, uvs {:?}, points {:?}", face.index, worst_aspect.0, worst_aspect.1, worst_aspect.2, worst_aspect.3);
			}
			return Ok(reject_mesh_tolerance(face, "ordinary physical triangle aspect p95 exceeds quality limit"));
		}
	}
	check_cancelled(progress)?;
	Ok(true)
}

/// Returns the best physical aspect ratio a bounded tensor refinement can
/// achieve for an intrinsically anisotropic four-sided face.
///
/// A microscopic fillet running along a large body can be thousands of times
/// longer than it is wide. Requiring the ordinary global aspect target on such
/// a patch would turn a visually negligible feature into hundreds of thousands
/// of triangles. The structured mesher balances the long axis up to a strict
/// resource ceiling. Its unavoidable aspect is therefore the face anisotropy,
/// scaled by the canonical sample count across the short axis and by that
/// ceiling. Deflection and normal-error audits remain unchanged.
fn resource_bounded_structured_aspect(face: &TrimmedFace) -> Option<f64> {
	let trim_loop = face.loops.first().filter(|_| face.loops.len() == 1)?;
	let runs = boundary_edge_runs(trim_loop);
	if runs.len() != 4 {
		return None;
	}
	let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
	let u_range = (u_max - u_min).abs();
	let v_range = (v_max - v_min).abs();
	let u_tolerance = u_range.max(1.0) * 1.0e-7;
	let v_tolerance = v_range.max(1.0) * 1.0e-7;
	let mut horizontal = Vec::new();
	let mut vertical = Vec::new();
	for run in runs {
		let (run_u_min, run_u_max, run_v_min, run_v_max) = run.iter().fold((f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY), |(u0, u1, v0, v1), vertex| (u0.min(vertex.uv.x), u1.max(vertex.uv.x), v0.min(vertex.uv.y), v1.max(vertex.uv.y)));
		let physical_length = run.windows(2).map(|pair| pair[0].position.distance(pair[1].position)).sum::<f64>();
		let intervals = run.len().saturating_sub(1);
		if run_v_max - run_v_min <= v_tolerance && run_u_max - run_u_min >= u_range * 0.90 {
			horizontal.push((physical_length, intervals));
		} else if run_u_max - run_u_min <= u_tolerance && run_v_max - run_v_min >= v_range * 0.90 {
			vertical.push((physical_length, intervals));
		} else {
			return None;
		}
	}
	if horizontal.len() != 2 || vertical.len() != 2 {
		return None;
	}
	let u_length = horizontal.iter().map(|(length, _)| *length).fold(0.0, f64::max);
	let v_length = vertical.iter().map(|(length, _)| *length).fold(0.0, f64::max);
	if !u_length.is_finite() || !v_length.is_finite() || u_length <= 1.0e-15 || v_length <= 1.0e-15 {
		return None;
	}
	let (long_length, short_length, short_intervals) = if u_length >= v_length { (u_length, v_length, vertical.iter().map(|(_, intervals)| *intervals).max()?) } else { (v_length, u_length, horizontal.iter().map(|(_, intervals)| *intervals).max()?) };
	let face_anisotropy = long_length / short_length;
	if face_anisotropy <= MAXIMUM_HARD_PHYSICAL_ASPECT || short_intervals == 0 {
		return None;
	}
	Some(face_anisotropy * short_intervals as f64 / MAXIMUM_BALANCED_AXIS_INTERVALS as f64)
}

fn percentile(values: &mut [f64], fraction: f64) -> Option<f64> {
	if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
		return None;
	}
	values.sort_by(f64::total_cmp);
	let rank = (fraction.clamp(0.0, 1.0) * values.len() as f64).ceil() as usize;
	values.get(rank.saturating_sub(1).min(values.len() - 1)).copied()
}

fn reject_mesh_tolerance(face: &TrimmedFace, reason: &str) -> bool {
	if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
		eprintln!("custom tessellation face {} rejected: {reason}", face.index);
	}
	false
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PointKey(u64, u64, u64);

fn point_key(point: DVec3) -> PointKey {
	fn coordinate_key(value: f64) -> u64 {
		if value == 0.0 {
			0
		} else {
			value.to_bits()
		}
	}
	PointKey(coordinate_key(point.x), coordinate_key(point.y), coordinate_key(point.z))
}

#[derive(Clone, Copy)]
struct StructuredBoundarySides<'a> {
	lower: &'a [&'a BoundaryVertex],
	upper: &'a [&'a BoundaryVertex],
	left: &'a [&'a BoundaryVertex],
	right: &'a [&'a BoundaryVertex],
}

/// Tessellates an untrimmed four-sided surface chart as a coherent tensor
/// grid. This covers the regular charts used by most analytic and swept CAD
/// faces, including periodic seams and collapsed sphere/cone poles.
fn mesh_structured_patch(face: &TrimmedFace, linear: f64, angular: f64, progress: &ffi::CancellationToken) -> Result<Option<MeshedFace>, Error> {
	check_cancelled(progress)?;
	let Some(trim_loop) = face.loops.first().filter(|_| face.loops.len() == 1) else {
		return Ok(None);
	};
	let runs = boundary_edge_runs(trim_loop);
	if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
		let descriptions = runs
			.iter()
			.map(|run| {
				let (u_min, u_max, v_min, v_max) = run.iter().fold((f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY), |(u0, u1, v0, v1), vertex| (u0.min(vertex.uv.x), u1.max(vertex.uv.x), v0.min(vertex.uv.y), v1.max(vertex.uv.y)));
				(run.len(), run.first().map(|vertex| vertex.edge_index), [u_min, u_max, v_min, v_max])
			})
			.collect::<Vec<_>>();
		eprintln!("custom tessellation face {} structured runs: {:?}", face.index, descriptions);
	}
	if runs.len() != 4 {
		return Ok(None);
	}
	let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
	let u_range = u_max - u_min;
	let v_range = v_max - v_min;
	let u_tolerance = u_range.abs().max(1.0) * 1.0e-7;
	let v_tolerance = v_range.abs().max(1.0) * 1.0e-7;
	let mut horizontal = Vec::new();
	let mut vertical = Vec::new();
	for run in runs {
		let (run_u_min, run_u_max, run_v_min, run_v_max) = run.iter().fold((f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY), |(u0, u1, v0, v1), vertex| (u0.min(vertex.uv.x), u1.max(vertex.uv.x), v0.min(vertex.uv.y), v1.max(vertex.uv.y)));
		if run_v_max - run_v_min <= v_tolerance && run_u_max - run_u_min >= u_range.abs() * 0.90 {
			let mut run = run;
			run.sort_by(|first, second| first.uv.x.total_cmp(&second.uv.x));
			horizontal.push(run);
		} else if run_u_max - run_u_min <= u_tolerance && run_v_max - run_v_min >= v_range.abs() * 0.90 {
			let mut run = run;
			run.sort_by(|first, second| first.uv.y.total_cmp(&second.uv.y));
			vertical.push(run);
		} else {
			return Ok(None);
		}
	}
	if horizontal.len() != 2 || vertical.len() != 2 {
		return Ok(None);
	}
	horizontal.sort_by(|first, second| mean_coordinate(first, false).total_cmp(&mean_coordinate(second, false)));
	vertical.sort_by(|first, second| mean_coordinate(first, true).total_cmp(&mean_coordinate(second, true)));
	let lower = horizontal[0].as_slice();
	let upper = horizontal[1].as_slice();
	let left = vertical[0].as_slice();
	let right = vertical[1].as_slice();
	if lower.len() < 2 || upper.len() < 2 || left.len() < 2 || right.len() < 2 {
		return Ok(None);
	}
	let lower_coordinates = lower.iter().map(|vertex| vertex.uv.x).collect::<Vec<_>>();
	let upper_coordinates = upper.iter().map(|vertex| vertex.uv.x).collect::<Vec<_>>();
	let left_coordinates = left.iter().map(|vertex| vertex.uv.y).collect::<Vec<_>>();
	let right_coordinates = right.iter().map(|vertex| vertex.uv.y).collect::<Vec<_>>();
	if !strictly_increasing(&lower_coordinates) || !strictly_increasing(&upper_coordinates) || !strictly_increasing(&left_coordinates) || !strictly_increasing(&right_coordinates) {
		return Ok(None);
	}

	// Preserve the compact tensor layout for faces whose opposite U boundaries
	// already share the same parameter samples. Most analytic primitives use
	// this path. Adaptive loft boundaries commonly differ, and are handled by
	// the inset-grid path below without inserting any new shared-edge vertices.
	let has_collapsed_horizontal_boundary = boundary_vertices_collapsed(lower) || boundary_vertices_collapsed(upper);
	let left_edge = dominant_boundary_edge_index(left);
	let right_edge = dominant_boundary_edge_index(right);
	let sides_are_one_periodic_seam = left_edge.zip(right_edge).is_some_and(|(left, right)| left == right);
	if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() && has_collapsed_horizontal_boundary {
		eprintln!("custom tessellation face {} singular routing: periodic seam={}, left edge={left_edge:?}, right edge={right_edge:?}", face.index, sides_are_one_periodic_seam);
	}
	if has_collapsed_horizontal_boundary && sides_are_one_periodic_seam {
		if let Some(mesh) = mesh_singular_row_structured_patch(face, lower, upper, left, right, u_tolerance, v_tolerance, linear, angular, progress)? {
			if mesh_satisfies_tolerances(face, &mesh, linear, angular, progress)? {
				return Ok(Some(mesh));
			}
		}
	}
	if has_collapsed_horizontal_boundary || lower.len() == upper.len() && lower.iter().zip(upper).all(|(first, second)| (first.uv.x - second.uv.x).abs() <= u_tolerance) {
		let boundaries = StructuredBoundarySides { lower, upper, left, right };
		match mesh_column_structured_patch(face, boundaries, v_tolerance, linear, angular, progress) {
			Ok(Some(mesh)) => {
				if mesh_satisfies_tolerances(face, &mesh, linear, angular, progress)? {
					return Ok(Some(mesh));
				}
			}
			Ok(None) => {}
			// A compact tensor candidate can cross a sharp repeated-knot joint
			// whose canonical boundary samples cannot be moved face-locally. Its
			// final tolerance audit rejects that folded cell. Continue to the
			// inset/CDT routes, which can add interior U samples while preserving
			// the exact shared boundary. Resource and cancellation failures remain
			// authoritative rather than being disguised as a routing decision.
			Err(Error::TriangulationFailed) => {
				if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
					eprintln!("custom tessellation face {} rejected its compact structured candidate; trying a boundary-preserving fallback", face.index);
				}
			}
			Err(error) => return Err(error),
		}
	}
	// Four independently generated inset collars overlap at their corners.
	// Exact duplicate welding closes the coincident radial edges, but it cannot
	// turn the two different corner diagonals into one valid planar cavity.  A
	// planar face needs no structured surface-error control, so let the canonical
	// constrained Delaunay route fill it as one domain instead of accepting or
	// concealing an incidence-one corner seam.
	if face.surface.is_planar() {
		return Ok(None);
	}

	// Try one shallow graded collar before the robust CDT route. Rebuilding and
	// auditing progressively deeper collars scales poorly on faces where no
	// collar can satisfy the aspect bound, while the fallback already preserves
	// the canonical boundary and all geometric tolerances.
	for transition_ring_count in [8] {
		check_cancelled(progress)?;
		let mesh = match mesh_inset_structured_patch(face, lower, upper, left, right, u_tolerance, v_tolerance, linear, angular, transition_ring_count, progress) {
			Ok(Some(mesh)) => mesh,
			Ok(None) => return Ok(None),
			// A sparse boundary can force the first collar to bridge too far
			// across a strongly curved chart. Treat that local connectivity as
			// a rejected candidate and retry with a more gradual, still bounded
			// collar; cancellation and resource failures remain authoritative.
			Err(Error::TriangulationFailed) => continue,
			Err(error) => return Err(error),
		};
		if mesh_satisfies_tolerances(face, &mesh, linear, angular, progress)? {
			return Ok(Some(mesh));
		}
	}
	Ok(None)
}

/// Builds periodic singular charts as latitude-like rows whose sample count
/// decreases with physical circumference near a collapsed pole. A fixed
/// tensor grid sends every equatorial column into one apex and consequently
/// creates arbitrarily thin triangles even when chord error is small. The
/// graded rows retain the exact non-collapsed boundary, use one geometric pole
/// vertex, and zipper adjacent rings with deterministic minimax connectivity.
#[allow(clippy::too_many_arguments)]
fn mesh_singular_row_structured_patch(face: &TrimmedFace, lower: &[&BoundaryVertex], upper: &[&BoundaryVertex], left: &[&BoundaryVertex], right: &[&BoundaryVertex], u_tolerance: f64, v_tolerance: f64, linear: f64, angular: f64, progress: &ffi::CancellationToken) -> Result<Option<MeshedFace>, Error> {
	check_cancelled(progress)?;
	let lower_collapsed = boundary_vertices_collapsed(lower);
	let upper_collapsed = boundary_vertices_collapsed(upper);
	if !lower_collapsed && !upper_collapsed {
		return Ok(None);
	}
	let [u_min, u_max, _, _] = face.surface.uv_bounds;
	let mut reference_u = adaptive_axis_coordinates(face, ParametricAxis::U, [lower, upper], AxisSampling { target_length: f64::INFINITY, linear: linear * 0.5, angular: angular * 0.5, coordinate_tolerance: u_tolerance }, progress)?;
	reference_u.sort_by(f64::total_cmp);
	reference_u.dedup_by(|first, second| (*first - *second).abs() <= u_tolerance);
	if reference_u.len() < 4 {
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation face {} graded singular rejected with only {} periodic samples", face.index, reference_u.len());
		}
		return Ok(None);
	}
	// The two chart sides are occurrences of one topological seam. Never add a
	// face-local sample to that seam: doing so would split a canonical segment
	// on this face without making the identical change on its neighbor. Ring
	// density may vary around U, while latitude rows are restricted to the
	// once-sampled shared-edge sequence plus a single deterministic refinement
	// shared by both occurrences of this self-seam.
	let canonical_seam = if left.len() >= right.len() { left } else { right };
	let seam_edge_index = dominant_boundary_edge_index(canonical_seam).ok_or(Error::TriangulationFailed)?;
	let mut v_coordinates = adaptive_axis_coordinates(face, ParametricAxis::V, [canonical_seam, canonical_seam], AxisSampling { target_length: f64::INFINITY, linear: linear * 0.5, angular: angular * 0.5, coordinate_tolerance: v_tolerance }, progress)?;
	v_coordinates.sort_by(f64::total_cmp);
	v_coordinates.dedup_by(|first, second| (*first - *second).abs() <= v_tolerance);
	if !refine_compact_patch_v_coordinates(face, &reference_u, &mut v_coordinates, v_tolerance, linear * 0.5, progress)? || !strictly_increasing(&v_coordinates) {
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation face {} graded singular rejected self-seam refinement: {}x{}", face.index, reference_u.len(), v_coordinates.len());
		}
		return Ok(None);
	}

	let ring_lengths = v_coordinates.iter().copied().map(|v| surface_polyline_length(face, &reference_u, v, progress)).collect::<Result<Vec<_>, _>>()?;
	let maximum_ring_length = ring_lengths.iter().copied().fold(0.0_f64, f64::max);
	if !maximum_ring_length.is_finite() || maximum_ring_length <= 1.0e-12 {
		return Ok(None);
	}
	let boundary_intervals = reference_u.len().saturating_sub(1).clamp(3, 512);
	const MAXIMUM_SINGULAR_RING_INTERVALS: usize = 512;
	let mut planned_row_sizes = Vec::with_capacity(v_coordinates.len());
	let mut planned_vertex_count = 0usize;
	for (v_index, _) in v_coordinates.iter().enumerate() {
		cancellation_checkpoint(progress, v_index)?;
		let is_lower = v_index == 0;
		let is_upper = v_index + 1 == v_coordinates.len();
		let row_size = if is_lower && lower_collapsed || is_upper && upper_collapsed {
			1
		} else if is_lower || is_upper {
			if is_lower {
				lower.len()
			} else {
				upper.len()
			}
		} else {
			let center_u = (u_min + u_max) * 0.5;
			let previous_v = v_coordinates[v_index - 1];
			let next_v = v_coordinates[v_index + 1];
			let current = face.surface.evaluate_position(DVec2::new(center_u, v_coordinates[v_index]));
			let previous_step = face.surface.evaluate_position(DVec2::new(center_u, previous_v)).zip(current).map(|(first, second)| first.distance(second)).unwrap_or(0.0);
			let next_step = current.zip(face.surface.evaluate_position(DVec2::new(center_u, next_v))).map(|(first, second)| first.distance(second)).unwrap_or(0.0);
			let radial_step = previous_step.max(next_step).max(1.0e-12);
			let accuracy_intervals = (boundary_intervals as f64 * (ring_lengths[v_index] / maximum_ring_length).sqrt()).ceil() as usize;
			let aspect_intervals = (ring_lengths[v_index] / (radial_step * TARGET_PHYSICAL_ASPECT)).ceil() as usize;
			accuracy_intervals.max(aspect_intervals).clamp(3, MAXIMUM_SINGULAR_RING_INTERVALS) + 1
		};
		planned_vertex_count = checked_add_resource(planned_vertex_count, row_size, "tessellation singular structured vertex count overflowed")?;
		planned_row_sizes.push(row_size);
	}
	if planned_vertex_count > MAXIMUM_FACE_VERTICES {
		return Err(resource_limit("tessellation singular structured face exceeded the vertex limit"));
	}
	let mut vertices = Vec::new();
	let mut uvs = Vec::new();
	let mut normals = Vec::new();
	reserve_exact(&mut vertices, planned_vertex_count, "tessellation singular structured vertex allocation failed")?;
	reserve_exact(&mut uvs, planned_vertex_count, "tessellation singular structured parameter allocation failed")?;
	reserve_exact(&mut normals, planned_vertex_count, "tessellation singular structured normal allocation failed")?;
	let mut rows = Vec::<Vec<(f64, usize)>>::with_capacity(v_coordinates.len());
	let mut refined_seam_points = Vec::with_capacity(v_coordinates.len());

	for (v_index, v) in v_coordinates.iter().copied().enumerate() {
		cancellation_checkpoint(progress, v_index)?;
		let is_lower = v_index == 0;
		let is_upper = v_index + 1 == v_coordinates.len();
		let collapsed = is_lower && lower_collapsed || is_upper && upper_collapsed;
		if collapsed {
			let boundary = if is_lower { lower } else { upper };
			let pole = boundary.first().ok_or(Error::TriangulationFailed)?.position;
			let index = append_surface_vertex(face, DVec2::new((u_min + u_max) * 0.5, v), Some(pole), &mut vertices, &mut uvs, &mut normals)?;
			rows.push(vec![(0.5, index)]);
			refined_seam_points.push(pole);
			continue;
		}
		if is_lower && !lower_collapsed || is_upper && !upper_collapsed {
			let boundary = if is_lower { lower } else { upper };
			// `append_boundary_side` already returns a normalized chart key.
			// Normalizing it a second time shifts every non-unit periodic chart
			// (for example a cone's 0..TAU range), causing the zipper to join a
			// boundary sample to the wrong interior longitude.
			let row = append_boundary_side(face, boundary, true, &mut vertices, &mut uvs, &mut normals, progress)?;
			rows.push(row);
			let seam_position = boundary_position_at_coordinate(canonical_seam, v, false, false, v_tolerance).or_else(|| face.surface.evaluate_position(DVec2::new(u_min, v))).ok_or(Error::TriangulationFailed)?;
			refined_seam_points.push(seam_position);
			continue;
		}

		// Circular chord error is proportional to radius / intervals².  Reduce
		// the circumferential density with the square root of physical ring
		// length so every latitude retains the same sagitta budget instead of
		// collapsing too aggressively near the pole.
		let intervals = planned_row_sizes[v_index] - 1;
		let seam_position = boundary_position_at_coordinate(canonical_seam, v, false, false, v_tolerance).or_else(|| face.surface.evaluate_position(DVec2::new(u_min, v))).ok_or(Error::TriangulationFailed)?;
		refined_seam_points.push(seam_position);
		let mut row = Vec::with_capacity(intervals + 1);
		for u_index in 0..=intervals {
			cancellation_checkpoint(progress, u_index)?;
			let fraction = u_index as f64 / intervals as f64;
			let u = u_min + (u_max - u_min) * fraction;
			let boundary_position = (u_index == 0 || u_index == intervals).then_some(seam_position);
			let index = append_surface_vertex(face, DVec2::new(u, v), boundary_position, &mut vertices, &mut uvs, &mut normals)?;
			row.push((fraction, index));
		}
		rows.push(row);
	}
	debug_assert_eq!(vertices.len(), planned_vertex_count);

	let planned_index_count = checked_mul_resource(planned_vertex_count, 6, "tessellation singular structured index count overflowed")?;
	let mut indices = Vec::new();
	reserve_exact(&mut indices, planned_index_count, "tessellation singular structured index allocation failed")?;
	for (row_index, pair) in rows.windows(2).enumerate() {
		cancellation_checkpoint(progress, row_index)?;
		match (pair[0].as_slice(), pair[1].as_slice()) {
			([(_, pole)], ring) => push_pole_fan(*pole, ring, &vertices, &normals, &mut indices, progress)?,
			(ring, [(_, pole)]) => push_pole_fan(*pole, ring, &vertices, &normals, &mut indices, progress)?,
			(first, second) => triangulate_monotone_strip(face, first, second, &vertices, &uvs, &normals, 1.0e-10, linear, angular, &mut indices, progress)?,
		}
	}
	if indices.is_empty() {
		return Ok(None);
	}
	stabilize_vertex_normals(face, &uvs, &vertices, &indices, &mut normals, progress)?;
	if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
		let row_sizes = rows.iter().map(Vec::len).collect::<Vec<_>>();
		eprintln!("custom tessellation face {} graded singular rows {:?}: {} vertices, {} triangles", face.index, row_sizes, vertices.len(), indices.len() / 3);
	}
	let refined_edges = BTreeMap::from([(seam_edge_index, refined_seam_points)]);
	let boundary_refinements = self_seam_boundary_refinements(left, right, &v_coordinates, &refined_edges[&seam_edge_index])?;
	Ok(Some(MeshedFace { index: face.index, tshape_id: face.tshape_id, vertices, uvs, normals, indices, refined_edges, boundary_refinements, quality_exempt_vertices: BTreeSet::new() }))
}

fn surface_polyline_length(face: &TrimmedFace, u_coordinates: &[f64], v: f64, progress: &ffi::CancellationToken) -> Result<f64, Error> {
	let mut previous: Option<DVec3> = None;
	let mut length = 0.0;
	for (index, u) in u_coordinates.iter().enumerate() {
		cancellation_checkpoint(progress, index)?;
		let position = face.surface.evaluate_position(DVec2::new(*u, v)).ok_or(Error::TriangulationFailed)?;
		if let Some(previous) = previous {
			length += position.distance(previous);
		}
		previous = Some(position);
	}
	Ok(length)
}

fn push_pole_fan(pole: usize, ring: &[(f64, usize)], vertices: &[DVec3], normals: &[DVec3], indices: &mut Vec<u32>, progress: &ffi::CancellationToken) -> Result<(), Error> {
	if ring.len() < 2 {
		return Err(Error::TriangulationFailed);
	}
	for (segment_index, segment) in ring.windows(2).enumerate() {
		cancellation_checkpoint(progress, segment_index)?;
		push_surface_oriented_triangle([segment[0].1, segment[1].1, pole], vertices, normals, indices)?;
	}
	Ok(())
}

fn mesh_column_structured_patch(face: &TrimmedFace, boundaries: StructuredBoundarySides<'_>, v_tolerance: f64, linear: f64, angular: f64, progress: &ffi::CancellationToken) -> Result<Option<MeshedFace>, Error> {
	check_cancelled(progress)?;
	let StructuredBoundarySides { lower, upper, left, right } = boundaries;
	let lower_collapsed = boundary_vertices_collapsed(lower);
	let upper_collapsed = boundary_vertices_collapsed(upper);
	let has_collapsed_horizontal_boundary = lower_collapsed || upper_collapsed;
	let both_horizontal_boundaries_collapsed = lower_collapsed && upper_collapsed;
	if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
		eprintln!("custom tessellation face {} compact boundary collapse: lower={lower_collapsed}, upper={upper_collapsed}", face.index);
	}
	let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
	let self_seam_edge = dominant_boundary_edge_index(left).zip(dominant_boundary_edge_index(right)).filter(|(left_edge, right_edge)| left_edge == right_edge).map(|(edge, _)| edge);
	// A direct tensor grid is especially important at a chart pole.  Insets
	// would create four independently triangulated corner regions which can
	// overlap at the collapsed boundary.  Instead, adapt the non-collapsed
	// parameter direction while retaining every exact edge sample.
	let mut v_coordinates = if has_collapsed_horizontal_boundary { adaptive_axis_coordinates(face, ParametricAxis::V, [left, right], AxisSampling { target_length: f64::INFINITY, linear: linear * 0.5, angular: angular * 0.5, coordinate_tolerance: v_tolerance }, progress)? } else { left.iter().chain(right.iter()).map(|vertex| vertex.uv.y).collect::<Vec<_>>() };
	v_coordinates.sort_by(f64::total_cmp);
	v_coordinates.dedup_by(|first, second| (*first - *second).abs() <= v_tolerance);
	let u_coordinates = if both_horizontal_boundaries_collapsed {
		let adaptive_u = adaptive_axis_coordinates(face, ParametricAxis::U, [lower, upper], AxisSampling { target_length: f64::INFINITY, linear, angular, coordinate_tolerance: (u_max - u_min).abs().max(1.0) * 1.0e-7 }, progress)?;
		let metric = MetricMap::from_face(face);
		let [_, _, v_min, v_max] = face.surface.uv_bounds;
		let metric_u = metric_distance(metric.map(DVec2::new(u_min, (v_min + v_max) * 0.5)), metric.map(DVec2::new(u_max, (v_min + v_max) * 0.5)));
		let metric_v = metric_distance(metric.map(DVec2::new((u_min + u_max) * 0.5, v_min)), metric.map(DVec2::new((u_min + u_max) * 0.5, v_max)));
		let balanced_intervals = if metric_u.is_finite() && metric_v.is_finite() && metric_v > 1.0e-12 { (metric_u / metric_v * (v_coordinates.len() - 1) as f64).ceil() as usize } else { 1 };
		let intervals = adaptive_u.len().saturating_sub(1).max(balanced_intervals).clamp(3, MAXIMUM_BALANCED_AXIS_INTERVALS);
		(0..=intervals).map(|index| u_min + (u_max - u_min) * index as f64 / intervals as f64).collect::<Vec<_>>()
	} else if lower_collapsed {
		upper.iter().map(|vertex| vertex.uv.x).collect::<Vec<_>>()
	} else if upper_collapsed {
		lower.iter().map(|vertex| vertex.uv.x).collect::<Vec<_>>()
	} else {
		lower.iter().zip(upper).map(|(first, second)| (first.uv.x + second.uv.x) * 0.5).collect::<Vec<_>>()
	};
	if u_coordinates.len() > 1 {
		// On a cylinder- or cone-like chart the V generators are straight, so
		// a one-axis chord audit alone legitimately asks for no V refinement.
		// The cell diagonals still combine U curvature with the full generator
		// length. Balance the physical steps of both axes to keep them local.
		let metric = MetricMap::from_face(face);
		let metric_u = metric_distance(metric.map(DVec2::new(u_min, (v_min + v_max) * 0.5)), metric.map(DVec2::new(u_max, (v_min + v_max) * 0.5)));
		let metric_v = metric_distance(metric.map(DVec2::new((u_min + u_max) * 0.5, v_min)), metric.map(DVec2::new((u_min + u_max) * 0.5, v_max)));
		let u_step = metric_u / (u_coordinates.len() - 1) as f64;
		let balanced_intervals = if metric_v.is_finite() && u_step.is_finite() && u_step > 1.0e-12 { (metric_v / u_step).ceil() as usize } else { 1 };
		let balanced_intervals = balanced_intervals.clamp(1, MAXIMUM_BALANCED_AXIS_INTERVALS);
		if self_seam_edge.is_some() {
			// Preserve every canonical seam parameter while subdividing each
			// existing interval independently. Unioning that sequence with a
			// phase-shifted uniform grid creates arbitrarily tiny intervals even
			// when both inputs are individually well spaced.
			v_coordinates = subdivide_axis_intervals(&v_coordinates, (v_max - v_min).abs() / balanced_intervals as f64);
		} else {
			v_coordinates.extend((0..=balanced_intervals).map(|index| v_min + (v_max - v_min) * index as f64 / balanced_intervals as f64));
			v_coordinates.sort_by(f64::total_cmp);
			v_coordinates.dedup_by(|first, second| (*first - *second).abs() <= v_tolerance);
		}
	}
	if !refine_compact_patch_v_coordinates(face, &u_coordinates, &mut v_coordinates, v_tolerance, linear, progress)? {
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation face {} compact structured rejected: diagonal refinement exhausted its resource budget", face.index);
		}
		return Ok(None);
	}
	if !strictly_increasing(&u_coordinates) || !strictly_increasing(&v_coordinates) {
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation face {} compact structured rejected non-increasing axes: u={}, v={}", face.index, strictly_increasing(&u_coordinates), strictly_increasing(&v_coordinates));
		}
		return Ok(None);
	}
	let u_count = u_coordinates.len();
	if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
		let maximum_u_step = u_coordinates.windows(2).map(|pair| pair[1] - pair[0]).fold(0.0, f64::max);
		eprintln!("custom tessellation face {} compact structured grid: {}x{} points, maximum u step {maximum_u_step:.12e}", face.index, u_count, v_coordinates.len());
	}
	let boundary_vertices = if self_seam_edge.is_some() { checked_mul_resource(2, v_coordinates.len(), "tessellation compact structured vertex count overflowed")? } else { checked_add_resource(left.len(), right.len(), "tessellation compact structured vertex count overflowed")? };
	let interior_vertices = checked_mul_resource(u_count.saturating_sub(2), v_coordinates.len(), "tessellation compact structured vertex count overflowed")?;
	let estimated_vertices = checked_add_resource(boundary_vertices, interior_vertices, "tessellation compact structured vertex count overflowed")?;
	if estimated_vertices > MAXIMUM_FACE_VERTICES {
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation face {} compact structured rejected resource estimate {estimated_vertices} > {MAXIMUM_FACE_VERTICES}", face.index);
		}
		return Err(resource_limit("tessellation compact structured face exceeded the vertex limit"));
	}

	let mut vertices = Vec::new();
	let mut uvs = Vec::new();
	let mut normals = Vec::new();
	reserve_exact(&mut vertices, estimated_vertices, "tessellation compact structured vertex allocation failed")?;
	reserve_exact(&mut uvs, estimated_vertices, "tessellation compact structured parameter allocation failed")?;
	reserve_exact(&mut normals, estimated_vertices, "tessellation compact structured normal allocation failed")?;
	let refined_seam_points = if self_seam_edge.is_some() { Some(v_coordinates.iter().copied().map(|v| boundary_position_at_coordinate(left, v, false, false, v_tolerance).or_else(|| face.surface.evaluate_position(DVec2::new(u_min, v))).ok_or(Error::TriangulationFailed)).collect::<Result<Vec<_>, _>>()?) } else { None };
	let mut columns = Vec::<Vec<(f64, usize)>>::with_capacity(u_count);
	for (u_index, u) in u_coordinates.iter().copied().enumerate() {
		cancellation_checkpoint(progress, u_index)?;
		let boundary_column = if u_index == 0 {
			Some(left)
		} else if u_index + 1 == u_count {
			Some(right)
		} else {
			None
		};
		let owned_boundary_coordinates = boundary_column.map(|column| if self_seam_edge.is_some() { v_coordinates.clone() } else { column.iter().map(|vertex| vertex.uv.y).collect::<Vec<_>>() });
		let column_coordinates = owned_boundary_coordinates.as_deref().unwrap_or(v_coordinates.as_slice());
		let mut column_indices = Vec::with_capacity(column_coordinates.len());
		for (v_index, v) in column_coordinates.iter().copied().enumerate() {
			cancellation_checkpoint(progress, v_index)?;
			// Opposite canonical boundaries can differ by a few chart ULPs even
			// when their physical samples correspond one-for-one. The tensor
			// interior uses averaged coordinates, but every unrefined boundary
			// vertex must retain its exact UV-position pair. Mixing a canonical
			// position from one side with an averaged coordinate silently loses
			// the typed segment's bit-identical identity.
			let canonical_boundary = boundary_column.filter(|_| self_seam_edge.is_none()).and_then(|column| column.get(v_index).copied()).or_else(|| {
				if v_index == 0 && !lower_collapsed {
					lower.get(u_index).copied()
				} else if v_index + 1 == column_coordinates.len() && !upper_collapsed {
					upper.get(u_index).copied()
				} else {
					None
				}
			});
			let uv = canonical_boundary.map_or_else(
				|| {
					let boundary_u = boundary_column.and_then(|column| column.first()).map_or(u, |vertex| vertex.uv.x);
					DVec2::new(boundary_u, v)
				},
				|vertex| vertex.uv,
			);
			let sample = face.surface.evaluate(uv).ok_or(Error::TriangulationFailed)?;
			let position = if let Some(vertex) = canonical_boundary {
				vertex.position
			} else if let Some(column) = boundary_column {
				if let Some(points) = refined_seam_points.as_ref() {
					points[v_index]
				} else {
					column[v_index].position
				}
			} else if v_index == 0 {
				boundary_position_at_coordinate(lower, u, true, lower_collapsed, v_tolerance).ok_or(Error::TriangulationFailed)?
			} else if v_index + 1 == column_coordinates.len() {
				boundary_position_at_coordinate(upper, u, true, upper_collapsed, v_tolerance).ok_or(Error::TriangulationFailed)?
			} else {
				sample.position
			};
			let normal = oriented_surface_normal_from_sample(face, uv, sample).unwrap_or(DVec3::ZERO);
			let index = vertices.len();
			vertices.push(position);
			uvs.push(uv);
			normals.push(normal);
			column_indices.push((v, index));
		}
		columns.push(column_indices);
	}

	let estimated_indices = checked_mul_resource(estimated_vertices, 6, "tessellation compact structured index count overflowed")?;
	let mut indices = Vec::new();
	reserve_exact(&mut indices, estimated_indices, "tessellation compact structured index allocation failed")?;
	for (column_index, pair) in columns.windows(2).enumerate() {
		cancellation_checkpoint(progress, column_index)?;
		triangulate_monotone_strip(face, pair[0].as_slice(), pair[1].as_slice(), &vertices, &uvs, &normals, v_tolerance, linear, angular, &mut indices, progress)?;
	}
	if indices.is_empty() {
		return Err(Error::TriangulationFailed);
	}
	stabilize_vertex_normals(face, &uvs, &vertices, &indices, &mut normals, progress)?;
	let refined_edges = self_seam_edge.zip(refined_seam_points).into_iter().collect::<BTreeMap<_, _>>();
	let boundary_refinements = match self_seam_edge {
		Some(edge_index) => self_seam_boundary_refinements(left, right, &v_coordinates, &refined_edges[&edge_index])?,
		None => Vec::new(),
	};
	Ok(Some(MeshedFace { index: face.index, tshape_id: face.tshape_id, vertices, uvs, normals, indices, refined_edges, boundary_refinements, quality_exempt_vertices: BTreeSet::new() }))
}

fn subdivide_axis_intervals(coordinates: &[f64], maximum_step: f64) -> Vec<f64> {
	if coordinates.len() < 2 || !maximum_step.is_finite() || maximum_step <= 0.0 {
		return coordinates.to_vec();
	}
	let mut result = Vec::with_capacity(coordinates.len());
	result.push(coordinates[0]);
	for pair in coordinates.windows(2) {
		let intervals = ((pair[1] - pair[0]).abs() / maximum_step).ceil().max(1.0) as usize;
		result.extend((1..=intervals).map(|index| pair[0] + (pair[1] - pair[0]) * index as f64 / intervals as f64));
	}
	result
}

/// Refines the regular coordinate away from a collapsed chart boundary until
/// both possible tensor-cell diagonals meet the requested physical error.
/// Axis-only refinement cannot detect this mixed derivative: a cone's U arcs
/// and V generators are each individually well sampled while a long diagonal
/// between them still bows far away from the exact surface.
fn refine_compact_patch_v_coordinates(face: &TrimmedFace, u_coordinates: &[f64], v_coordinates: &mut Vec<f64>, tolerance: f64, linear: f64, progress: &ffi::CancellationToken) -> Result<bool, Error> {
	const MAXIMUM_COMPACT_PATCH_V_POINTS: usize = 513;
	const MAXIMUM_COMPACT_PATCH_REFINEMENT_PASSES: usize = 16;

	for pass in 0..MAXIMUM_COMPACT_PATCH_REFINEMENT_PASSES {
		cancellation_checkpoint(progress, pass)?;
		let mut insertions = Vec::new();
		for (v_index, v_pair) in v_coordinates.windows(2).enumerate() {
			cancellation_checkpoint(progress, v_index)?;
			let midpoint_v = (v_pair[0] + v_pair[1]) * 0.5;
			let mut needs_split = false;
			for (u_index, u_pair) in u_coordinates.windows(2).enumerate() {
				cancellation_checkpoint(progress, u_index)?;
				let diagonals = [(DVec2::new(u_pair[0], v_pair[0]), DVec2::new(u_pair[1], v_pair[1])), (DVec2::new(u_pair[1], v_pair[0]), DVec2::new(u_pair[0], v_pair[1]))];
				for (first_uv, last_uv) in diagonals {
					let midpoint_uv = regularized_edge_midpoint_uv(face, first_uv, last_uv);
					let first = face.surface.evaluate_position(first_uv).ok_or(Error::TriangulationFailed)?;
					let midpoint = face.surface.evaluate_position(midpoint_uv).ok_or(Error::TriangulationFailed)?;
					let last = face.surface.evaluate_position(last_uv).ok_or(Error::TriangulationFailed)?;
					let deviation = point_segment_distance(midpoint, first, last);
					// Splitting V cannot reduce a normal change caused solely by
					// the fixed, canonical U interval. Its angular accuracy is
					// therefore governed by that exact boundary discretization;
					// this mixed-axis pass is specifically for diagonal sagitta.
					if deviation > linear {
						needs_split = true;
						break;
					}
				}
				if needs_split {
					break;
				}
			}
			if needs_split && midpoint_v > v_pair[0] && midpoint_v < v_pair[1] {
				insertions.push(midpoint_v);
			}
		}
		if insertions.is_empty() {
			return Ok(true);
		}
		if v_coordinates.len().saturating_add(insertions.len()) > MAXIMUM_COMPACT_PATCH_V_POINTS {
			return Ok(false);
		}
		v_coordinates.extend(insertions);
		v_coordinates.sort_by(f64::total_cmp);
		v_coordinates.dedup_by(|first, second| (*first - *second).abs() <= tolerance);
	}

	Ok(false)
}

fn boundary_vertices_collapsed(boundary: &[&BoundaryVertex]) -> bool {
	let Some(first) = boundary.first() else {
		return false;
	};
	boundary.iter().all(|vertex| vertex.position.distance(first.position) <= 1.0e-10)
}

fn self_seam_boundary_refinements(left: &[&BoundaryVertex], right: &[&BoundaryVertex], coordinates: &[f64], positions: &[DVec3]) -> Result<Vec<BoundaryOccurrenceRefinement>, Error> {
	fn refinement_for_occurrence(boundary: &[&BoundaryVertex], coordinates: &[f64], positions: &[DVec3]) -> Result<BoundaryOccurrenceRefinement, Error> {
		let edge_index = dominant_boundary_edge_index(boundary).ok_or(Error::TriangulationFailed)?;
		let mut occurrence_counts = BTreeMap::<u32, usize>::new();
		for vertex in boundary.iter().filter(|vertex| vertex.edge_index == edge_index) {
			*occurrence_counts.entry(vertex.edge_occurrence_index).or_default() += 1;
		}
		let occurrence_index = occurrence_counts.into_iter().max_by_key(|(occurrence, count)| (*count, Reverse(*occurrence))).map(|(occurrence, _)| occurrence).ok_or(Error::TriangulationFailed)?;
		let occurrence_vertices = boundary.iter().enumerate().filter(|(_, vertex)| vertex.edge_index == edge_index && vertex.edge_occurrence_index == occurrence_index).collect::<Vec<_>>();
		let first = occurrence_vertices.first().ok_or(Error::TriangulationFailed)?;
		if occurrence_vertices.iter().any(|(_, vertex)| vertex.edge_occurrence_direction != first.1.edge_occurrence_direction) {
			return Err(Error::TriangulationFailed);
		}
		let canonical_ascending = match occurrence_vertices.as_slice() {
			[(_, first), .., (_, last)] if first.edge_sample_index != last.edge_sample_index => first.edge_sample_index < last.edge_sample_index,
			[(index, vertex)] => {
				let traversal_ascending = *index == 0;
				match vertex.edge_occurrence_direction {
					EdgeOccurrenceDirection::Forward => traversal_ascending,
					EdgeOccurrenceDirection::Reversed => !traversal_ascending,
				}
			}
			_ => return Err(Error::TriangulationFailed),
		};
		let u = first.1.uv.x;
		let mut points = coordinates.iter().copied().zip(positions.iter().copied()).map(|(v, position)| BoundaryContractPoint { uv: DVec2::new(u, v), position }).collect::<Vec<_>>();
		if !canonical_ascending {
			points.reverse();
		}
		Ok(BoundaryOccurrenceRefinement { occurrence: BoundaryOccurrence { loop_index: 0, edge_index, occurrence_index }, direction: first.1.edge_occurrence_direction, points })
	}

	if coordinates.len() != positions.len() || coordinates.len() < 2 {
		return Err(Error::TriangulationFailed);
	}
	let left = refinement_for_occurrence(left, coordinates, positions)?;
	let right = refinement_for_occurrence(right, coordinates, positions)?;
	if left.occurrence.edge_index != right.occurrence.edge_index || left.occurrence == right.occurrence {
		return Err(Error::TriangulationFailed);
	}
	Ok(vec![left, right])
}

fn dominant_boundary_edge_index(boundary: &[&BoundaryVertex]) -> Option<u32> {
	let mut counts = BTreeMap::<u32, usize>::new();
	for vertex in boundary {
		*counts.entry(vertex.edge_index).or_default() += 1;
	}
	counts.into_iter().max_by_key(|(edge, count)| (*count, Reverse(*edge))).map(|(edge, _)| edge)
}

fn boundary_position_at_coordinate(boundary: &[&BoundaryVertex], coordinate: f64, use_u: bool, collapsed: bool, tolerance: f64) -> Option<DVec3> {
	if collapsed {
		return boundary.first().map(|vertex| vertex.position);
	}
	boundary.iter().find(|vertex| ((if use_u { vertex.uv.x } else { vertex.uv.y }) - coordinate).abs() <= tolerance).map(|vertex| vertex.position)
}

/// Builds a boundary-conforming tensor patch when the four canonical edge
/// polylines have different sample counts or parameter locations.
///
/// The exact edge vertices form the outer ring. A regular, metric-balanced
/// grid is inset by one cell in parameter space; four deterministic monotone
/// zipper strips connect that grid to the original boundaries. Consequently
/// adjacent faces still consume byte-identical shared-edge positions, while
/// the face interior has coherent rows instead of an unconstrained CDT fan.
#[allow(clippy::too_many_arguments)]
fn mesh_inset_structured_patch(face: &TrimmedFace, lower: &[&BoundaryVertex], upper: &[&BoundaryVertex], left: &[&BoundaryVertex], right: &[&BoundaryVertex], u_tolerance: f64, v_tolerance: f64, linear: f64, angular: f64, transition_ring_count: usize, progress: &ffi::CancellationToken) -> Result<Option<MeshedFace>, Error> {
	check_cancelled(progress)?;
	let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
	let metric = MetricMap::from_face(face);
	let metric_u = metric_distance(metric.map(DVec2::new(u_min, (v_min + v_max) * 0.5)), metric.map(DVec2::new(u_max, (v_min + v_max) * 0.5)));
	let metric_v = metric_distance(metric.map(DVec2::new((u_min + u_max) * 0.5, v_min)), metric.map(DVec2::new((u_min + u_max) * 0.5, v_max)));
	if !metric_u.is_finite() || !metric_v.is_finite() || metric_u <= 1.0e-12 || metric_v <= 1.0e-12 {
		return Ok(None);
	}

	let boundary_target = [lower, upper, left, right].into_iter().filter_map(mean_boundary_segment_length).fold(f64::INFINITY, f64::min);
	if !boundary_target.is_finite() || boundary_target <= 1.0e-12 {
		return Ok(None);
	}
	let target_length = boundary_target.max(metric_u.max(metric_v) / 256.0).max(linear * 2.0);
	// A tensor-cell diagonal combines error from both parametric directions.
	// Refine each one-dimensional axis to half the requested tolerance so the
	// two contributions remain bounded when the grid is triangulated.
	let mut u_axis = adaptive_axis_coordinates(face, ParametricAxis::U, [lower, upper], AxisSampling { target_length, linear: linear * 0.4, angular: angular * 0.4, coordinate_tolerance: u_tolerance }, progress)?;
	let mut v_axis = adaptive_axis_coordinates(face, ParametricAxis::V, [left, right], AxisSampling { target_length, linear: linear * 0.4, angular: angular * 0.4, coordinate_tolerance: v_tolerance }, progress)?;
	ensure_axis_interior(&mut u_axis, u_min, u_max);
	ensure_axis_interior(&mut v_axis, v_min, v_max);
	// Opposite trims can contribute almost-coincident parameters even though
	// neither point is required in the face interior. Carrying both into a
	// tensor grid creates a whole row of artificial slivers. Remove only spacing
	// outliers relative to the axis' own median interval; exact outer-boundary
	// samples remain untouched and the final surface audit still enforces the
	// requested approximation tolerance.
	u_axis = regularize_axis_coordinates(&u_axis);
	v_axis = regularize_axis_coordinates(&v_axis);
	// A tensor product multiplies independent axis requirements. Keep the
	// viewport-sized grid bounded by coarsening only the physically
	// over-resolved axis; the final surface audit below rejects this candidate
	// if the reduced chart no longer meets linear or angular tolerances.
	const TARGET_STRUCTURED_GRID_CELLS: usize = 2_304;
	const COARSENING_SEARCH_TOLERANCE_SCALE: f64 = 2.0;
	while (u_axis.len() - 1).saturating_mul(v_axis.len() - 1) > TARGET_STRUCTURED_GRID_CELLS {
		check_cancelled(progress)?;
		let u_intervals = u_axis.len() - 1;
		let v_intervals = v_axis.len() - 1;
		let u_step = metric_u / u_intervals as f64;
		let v_step = metric_v / v_intervals as f64;
		let before = (u_axis.len(), v_axis.len());
		if u_step <= v_step && u_intervals > 3 {
			let target_intervals = (TARGET_STRUCTURED_GRID_CELLS / v_intervals).clamp(3, u_intervals - 1);
			u_axis = coarsen_axis_coordinates(face, ParametricAxis::U, &u_axis, target_intervals + 1, linear * COARSENING_SEARCH_TOLERANCE_SCALE, angular * COARSENING_SEARCH_TOLERANCE_SCALE, progress)?;
		} else if v_intervals > 3 {
			let target_intervals = (TARGET_STRUCTURED_GRID_CELLS / u_intervals).clamp(3, v_intervals - 1);
			v_axis = coarsen_axis_coordinates(face, ParametricAxis::V, &v_axis, target_intervals + 1, linear * COARSENING_SEARCH_TOLERANCE_SCALE, angular * COARSENING_SEARCH_TOLERANCE_SCALE, progress)?;
		} else {
			break;
		}
		if before == (u_axis.len(), v_axis.len()) {
			break;
		}
	}
	let mut u_intervals = u_axis.len() - 1;
	let mut v_intervals = v_axis.len() - 1;
	let boundary_vertex_count = lower.len().saturating_add(upper.len()).saturating_add(left.len()).saturating_add(right.len());
	while boundary_vertex_count.saturating_add(u_intervals.saturating_sub(1).saturating_mul(v_intervals.saturating_sub(1))) > MAXIMUM_FACE_VERTICES {
		check_cancelled(progress)?;
		if u_intervals >= v_intervals && u_intervals > 3 {
			u_axis = decimate_axis_coordinates(&u_axis);
			u_intervals = u_axis.len() - 1;
		} else if v_intervals > 3 {
			v_axis = decimate_axis_coordinates(&v_axis);
			v_intervals = v_axis.len() - 1;
		} else {
			return Ok(None);
		}
	}
	if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
		eprintln!("custom tessellation face {} inset structured grid: {}x{} intervals, boundaries {}/{}/{}/{}", face.index, u_intervals, v_intervals, lower.len(), upper.len(), left.len(), right.len());
	}

	// Keep one boundary-matched ring, then leave several adaptive cells before
	// transitioning to the denser core. This graded annulus prevents a sparse
	// straight edge from forming a near-collinear fan against a dense interior
	// row, without ever adding a vertex to the canonical shared edge itself.
	let u_transition = transition_axis_index(u_axis.len(), transition_ring_count);
	let v_transition = transition_axis_index(v_axis.len(), transition_ring_count);
	let mut u_coordinates = u_axis[u_transition..u_axis.len() - u_transition].to_vec();
	let mut v_coordinates = v_axis[v_transition..v_axis.len() - v_transition].to_vec();
	if !refine_tensor_coordinates(face, &mut u_coordinates, &mut v_coordinates, TensorSampling { u_tolerance, v_tolerance, linear: linear * 0.9, angular: angular * 3.0 }, progress)? {
		return Ok(None);
	}
	if !strictly_increasing(&u_coordinates) || !strictly_increasing(&v_coordinates) {
		return Ok(None);
	}

	let core_vertex_count = checked_mul_resource(u_coordinates.len(), v_coordinates.len(), "tessellation inset structured vertex count overflowed")?;
	let base_vertex_count = checked_add_resource(boundary_vertex_count, core_vertex_count, "tessellation inset structured vertex count overflowed")?;
	if base_vertex_count > MAXIMUM_FACE_VERTICES {
		return Err(resource_limit("tessellation inset structured face exceeded the vertex limit"));
	}
	let mut vertices = Vec::new();
	let mut uvs = Vec::new();
	let mut normals = Vec::new();
	reserve_exact(&mut vertices, base_vertex_count, "tessellation inset structured vertex allocation failed")?;
	reserve_exact(&mut uvs, base_vertex_count, "tessellation inset structured parameter allocation failed")?;
	reserve_exact(&mut normals, base_vertex_count, "tessellation inset structured normal allocation failed")?;
	let mut lower_indices = append_boundary_side(face, lower, true, &mut vertices, &mut uvs, &mut normals, progress)?;
	let mut upper_indices = append_boundary_side(face, upper, true, &mut vertices, &mut uvs, &mut normals, progress)?;
	let mut left_indices = append_boundary_side(face, left, false, &mut vertices, &mut uvs, &mut normals, progress)?;
	let mut right_indices = append_boundary_side(face, right, false, &mut vertices, &mut uvs, &mut normals, progress)?;
	weld_exact_boundary_corners([&mut lower_indices, &mut upper_indices, &mut left_indices, &mut right_indices], &vertices, &uvs)?;
	let mut columns = Vec::<Vec<(f64, usize)>>::with_capacity(u_coordinates.len());
	for (u_index, u) in u_coordinates.iter().copied().enumerate() {
		cancellation_checkpoint(progress, u_index)?;
		let mut column = Vec::with_capacity(v_coordinates.len());
		for (v_index, v) in v_coordinates.iter().copied().enumerate() {
			cancellation_checkpoint(progress, v_index)?;
			let index = append_surface_vertex(face, DVec2::new(u, v), None, &mut vertices, &mut uvs, &mut normals)?;
			column.push((parameter_fraction(v, v_min, v_max), index));
		}
		columns.push(column);
	}
	let bottom_inner = columns.iter().enumerate().map(|(u_index, column)| (parameter_fraction(u_coordinates[u_index], u_coordinates[0], u_coordinates[u_coordinates.len() - 1]), column[0].1)).collect::<Vec<_>>();
	let top_inner = columns.iter().enumerate().map(|(u_index, column)| (parameter_fraction(u_coordinates[u_index], u_coordinates[0], u_coordinates[u_coordinates.len() - 1]), column[column.len() - 1].1)).collect::<Vec<_>>();
	let left_inner = columns[0].iter().map(|(_, index)| (parameter_fraction(uvs[*index].y, v_coordinates[0], v_coordinates[v_coordinates.len() - 1]), *index)).collect::<Vec<_>>();
	let right_inner = columns[columns.len() - 1].iter().map(|(_, index)| (parameter_fraction(uvs[*index].y, v_coordinates[0], v_coordinates[v_coordinates.len() - 1]), *index)).collect::<Vec<_>>();
	lower_indices = augment_collapsed_boundary(face, &lower_indices, &bottom_inner, PatchSide::Lower, &mut vertices, &mut uvs, &mut normals, progress)?;
	upper_indices = augment_collapsed_boundary(face, &upper_indices, &top_inner, PatchSide::Upper, &mut vertices, &mut uvs, &mut normals, progress)?;
	left_indices = augment_collapsed_boundary(face, &left_indices, &left_inner, PatchSide::Left, &mut vertices, &mut uvs, &mut normals, progress)?;
	right_indices = augment_collapsed_boundary(face, &right_indices, &right_inner, PatchSide::Right, &mut vertices, &mut uvs, &mut normals, progress)?;
	let transition_vertex_count = checked_add_resource(checked_add_resource(transition_stage_vertex_count(lower_indices.len(), bottom_inner.len(), transition_ring_count)?, transition_stage_vertex_count(upper_indices.len(), top_inner.len(), transition_ring_count)?, "tessellation transition-ring vertex count overflowed")?, checked_add_resource(transition_stage_vertex_count(left_indices.len(), left_inner.len(), transition_ring_count)?, transition_stage_vertex_count(right_indices.len(), right_inner.len(), transition_ring_count)?, "tessellation transition-ring vertex count overflowed")?, "tessellation transition-ring vertex count overflowed")?;
	let planned_vertex_count = checked_add_resource(vertices.len(), transition_vertex_count, "tessellation inset structured vertex count overflowed")?;
	if planned_vertex_count > MAXIMUM_FACE_VERTICES {
		return Err(resource_limit("tessellation inset structured transition rings exceeded the vertex limit"));
	}
	reserve_exact(&mut vertices, transition_vertex_count, "tessellation inset structured transition allocation failed")?;
	reserve_exact(&mut uvs, transition_vertex_count, "tessellation inset structured transition allocation failed")?;
	reserve_exact(&mut normals, transition_vertex_count, "tessellation inset structured transition allocation failed")?;
	let mut lower_rings = append_transition_rings(face, &lower_indices, &bottom_inner, PatchSide::Lower, u_coordinates[0], u_coordinates[u_coordinates.len() - 1], v_coordinates[0], v_coordinates[v_coordinates.len() - 1], transition_ring_count, &mut vertices, &mut uvs, &mut normals, progress)?;
	let mut upper_rings = append_transition_rings(face, &upper_indices, &top_inner, PatchSide::Upper, u_coordinates[0], u_coordinates[u_coordinates.len() - 1], v_coordinates[0], v_coordinates[v_coordinates.len() - 1], transition_ring_count, &mut vertices, &mut uvs, &mut normals, progress)?;
	let mut left_rings = append_transition_rings(face, &left_indices, &left_inner, PatchSide::Left, u_coordinates[0], u_coordinates[u_coordinates.len() - 1], v_coordinates[0], v_coordinates[v_coordinates.len() - 1], transition_ring_count, &mut vertices, &mut uvs, &mut normals, progress)?;
	let mut right_rings = append_transition_rings(face, &right_indices, &right_inner, PatchSide::Right, u_coordinates[0], u_coordinates[u_coordinates.len() - 1], v_coordinates[0], v_coordinates[v_coordinates.len() - 1], transition_ring_count, &mut vertices, &mut uvs, &mut normals, progress)?;
	share_transition_ring_corners(&mut lower_rings, &mut upper_rings, &mut left_rings, &mut right_rings)?;
	let quality_exempt_indices = lower_indices.iter().chain(&upper_indices).chain(&left_indices).chain(&right_indices).map(|(_, index)| *index).chain([&lower_rings, &upper_rings, &left_rings, &right_rings].into_iter().flat_map(|rings| rings.iter().flatten().map(|(_, index)| *index)));
	let mut quality_exempt_vertices = BTreeSet::new();
	for (index_ordinal, index) in quality_exempt_indices.enumerate() {
		cancellation_checkpoint(progress, index_ordinal)?;
		quality_exempt_vertices.insert(u32::try_from(index).map_err(|_| Error::TriangulationFailed)?);
	}
	debug_assert_eq!(vertices.len(), planned_vertex_count);

	let planned_index_count = checked_mul_resource(planned_vertex_count, 6, "tessellation inset structured index count overflowed")?;
	let mut indices = Vec::new();
	reserve_exact(&mut indices, planned_index_count, "tessellation inset structured index allocation failed")?;
	for (column_index, pair) in columns.windows(2).enumerate() {
		cancellation_checkpoint(progress, column_index)?;
		triangulate_monotone_strip(face, &pair[0], &pair[1], &vertices, &uvs, &normals, 1.0e-10, linear, angular, &mut indices, progress)?;
	}
	// Strip ordering is chosen so its natural winding follows +du x +dv.
	triangulate_transition_rings(face, &lower_indices, &lower_rings, &bottom_inner, false, &vertices, &uvs, &normals, linear, angular, &mut indices, progress)?;
	triangulate_transition_rings(face, &upper_indices, &upper_rings, &top_inner, true, &vertices, &uvs, &normals, linear, angular, &mut indices, progress)?;
	triangulate_transition_rings(face, &left_indices, &left_rings, &left_inner, true, &vertices, &uvs, &normals, linear, angular, &mut indices, progress)?;
	triangulate_transition_rings(face, &right_indices, &right_rings, &right_inner, false, &vertices, &uvs, &normals, linear, angular, &mut indices, progress)?;
	weld_exact_mesh_vertex_indices(&vertices, &uvs, &mut indices, progress)?;
	if indices.is_empty() {
		return Err(Error::TriangulationFailed);
	}
	stabilize_vertex_normals(face, &uvs, &vertices, &indices, &mut normals, progress)?;
	Ok(Some(MeshedFace { index: face.index, tshape_id: face.tshape_id, vertices, uvs, normals, indices, refined_edges: BTreeMap::new(), boundary_refinements: Vec::new(), quality_exempt_vertices }))
}

#[derive(Clone, Copy)]
enum ParametricAxis {
	U,
	V,
}

fn near_parametric_boundary(face: &TrimmedFace, uv: DVec2) -> bool {
	// Loft profile differentials can become singular within a narrow U collar;
	// keep the ordinary trim tolerance along the longitudinal V direction.
	const U_BOUNDARY_LAYER_FRACTION: f64 = 5.0e-2;
	const V_BOUNDARY_LAYER_FRACTION: f64 = 1.0e-3;
	let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
	let u_range = (u_max - u_min).abs();
	let v_range = (v_max - v_min).abs();
	let u_tolerance = u_range * U_BOUNDARY_LAYER_FRACTION;
	let v_tolerance = v_range * V_BOUNDARY_LAYER_FRACTION;
	(uv.x - u_min).abs() <= u_tolerance || (uv.x - u_max).abs() <= u_tolerance || (uv.y - v_min).abs() <= v_tolerance || (uv.y - v_max).abs() <= v_tolerance
}

fn collapsed_parameter_axis(face: &TrimmedFace, uv: DVec2) -> Option<ParametricAxis> {
	let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
	let u_tolerance = (u_max - u_min).abs().max(1.0) * 1.0e-10;
	let v_tolerance = (v_max - v_min).abs().max(1.0) * 1.0e-10;
	if (face.collapsed_boundaries.u_at_v_min && (uv.y - v_min).abs() <= v_tolerance) || (face.collapsed_boundaries.u_at_v_max && (uv.y - v_max).abs() <= v_tolerance) {
		return Some(ParametricAxis::U);
	}
	if (face.collapsed_boundaries.v_at_u_min && (uv.x - u_min).abs() <= u_tolerance) || (face.collapsed_boundaries.v_at_u_max && (uv.x - u_max).abs() <= u_tolerance) {
		return Some(ParametricAxis::V);
	}
	None
}

fn collapsed_boundaries(surface: &RationalSurface) -> CollapsedBoundaries {
	let [u_min, u_max, v_min, v_max] = surface.uv_bounds;
	let Some(origin) = surface.poles.first().copied() else {
		return CollapsedBoundaries::default();
	};
	let geometric_scale = surface.poles.iter().map(|pole| pole.distance(origin)).fold(0.0, f64::max);
	let collapse_tolerance = geometric_scale * 1.0e-10 + 1.0e-12;
	let axis_collapses = |first: DVec2, last: DVec2| {
		let samples = [0.0, 0.25, 0.5, 0.75, 1.0].map(|fraction| surface.evaluate_position(first.lerp(last, fraction)));
		let [Some(first), Some(second), Some(middle), Some(fourth), Some(last)] = samples else {
			return false;
		};
		[second, middle, fourth, last].iter().all(|sample| first.distance(*sample) <= collapse_tolerance)
	};
	CollapsedBoundaries {
		u_at_v_min: axis_collapses(DVec2::new(u_min, v_min), DVec2::new(u_max, v_min)),
		u_at_v_max: axis_collapses(DVec2::new(u_min, v_max), DVec2::new(u_max, v_max)),
		v_at_u_min: axis_collapses(DVec2::new(u_min, v_min), DVec2::new(u_min, v_max)),
		v_at_u_max: axis_collapses(DVec2::new(u_max, v_min), DVec2::new(u_max, v_max)),
	}
}

fn regularized_edge_midpoint_uv(face: &TrimmedFace, first: DVec2, second: DVec2) -> DVec2 {
	let mut midpoint = (first + second) * 0.5;
	match (collapsed_parameter_axis(face, first), collapsed_parameter_axis(face, second)) {
		(Some(ParametricAxis::U), None) => midpoint.x = second.x,
		(None, Some(ParametricAxis::U)) => midpoint.x = first.x,
		(Some(ParametricAxis::V), None) => midpoint.y = second.y,
		(None, Some(ParametricAxis::V)) => midpoint.y = first.y,
		_ => {}
	}
	midpoint
}

fn regularized_triangle_center_uv(face: &TrimmedFace, uvs: [DVec2; 3]) -> DVec2 {
	let collapsed = uvs.map(|uv| collapsed_parameter_axis(face, uv));
	let mut center = (uvs[0] + uvs[1] + uvs[2]) / 3.0;
	if collapsed.iter().any(|axis| matches!(axis, Some(ParametricAxis::U))) {
		let regular = uvs.iter().zip(collapsed).filter_map(|(uv, axis)| (!matches!(axis, Some(ParametricAxis::U))).then_some(uv.x)).collect::<Vec<_>>();
		if !regular.is_empty() {
			center.x = regular.iter().sum::<f64>() / regular.len() as f64;
		}
	}
	if collapsed.iter().any(|axis| matches!(axis, Some(ParametricAxis::V))) {
		let regular = uvs.iter().zip(collapsed).filter_map(|(uv, axis)| (!matches!(axis, Some(ParametricAxis::V))).then_some(uv.y)).collect::<Vec<_>>();
		if !regular.is_empty() {
			center.y = regular.iter().sum::<f64>() / regular.len() as f64;
		}
	}
	center
}

/// Recognizes a sharp profile turn whose entire physical footprint is already
/// below the requested chord tolerance.
///
/// Procedural splines can encode a hard joint as a very narrow, formally smooth
/// B-spline transition rather than a repeated knot. Its normal rotates by almost
/// 180 degrees in a sub-deflection neighborhood, so recursively enforcing a
/// smooth angular field creates an asymptotic point cloud without changing the
/// visible or printable shape. Keep strict linear error there and treat the turn
/// as the hard feature it represents.
fn subdeflection_cusp(face: &TrimmedFace, uvs: [DVec2; 3], positions: [DVec3; 3], linear: f64) -> bool {
	let maximum_edge = [positions[0].distance(positions[1]), positions[1].distance(positions[2]), positions[2].distance(positions[0])].into_iter().fold(0.0, f64::max);
	// Allow a bounded diagonal around a linearly acceptable cell. Curved chart
	// metrics and asymmetric Delaunay refinement can make that diagonal modestly
	// longer than either independently audited center-to-edge probe.
	if !maximum_edge.is_finite() || maximum_edge > linear * 2.0 {
		return false;
	}
	let center = regularized_triangle_center_uv(face, uvs);
	// A formally smooth procedural spline can reverse its differential inside
	// a sub-deflection cell without exposing a repeated knot. Compare the
	// face-oriented facet against the exact center normal as well as comparing
	// the sampled surface normals with one another. Normalize the facet from UV
	// winding first so this works both while Spade still owns counter-clockwise
	// chart cells and after build_face_mesh has applied the face reversal.
	if let (Some(mut geometric), Some(expected)) = ((positions[1] - positions[0]).cross(positions[2] - positions[0]).try_normalize(), oriented_surface_normal(face, center)) {
		let uv_area = (uvs[1] - uvs[0]).perp_dot(uvs[2] - uvs[0]);
		if (uv_area < 0.0) != face.reversed {
			geometric = -geometric;
		}
		// A facet that crosses ninety degrees relative to the exact local
		// differential has traversed a normal reversal, not merely accumulated
		// ordinary smooth-surface angular error.
		if geometric.dot(expected) < 0.0 {
			return true;
		}
	}
	let samples = [uvs[0], uvs[1], uvs[2], center];
	let normals = samples.map(|uv| face.surface.evaluate(uv).and_then(SurfaceSample::normal));
	(0..normals.len()).any(|first| {
		(first + 1..normals.len()).any(|second| match (normals[first], normals[second]) {
			(Some(first), Some(second)) => first.dot(second) < -0.5,
			_ => false,
		})
	})
}

fn mean_boundary_segment_length(side: &[&BoundaryVertex]) -> Option<f64> {
	let (length, count) = side.windows(2).fold((0.0, 0_usize), |(length, count), pair| {
		let segment = pair[0].position.distance(pair[1].position);
		if segment.is_finite() && segment > 1.0e-12 {
			(length + segment, count + 1)
		} else {
			(length, count)
		}
	});
	(count > 0).then_some(length / count as f64)
}

#[derive(Clone, Copy)]
struct AxisSampling {
	target_length: f64,
	linear: f64,
	angular: f64,
	coordinate_tolerance: f64,
}

fn adaptive_axis_coordinates(face: &TrimmedFace, axis: ParametricAxis, boundaries: [&[&BoundaryVertex]; 2], sampling: AxisSampling, progress: &ffi::CancellationToken) -> Result<Vec<f64>, Error> {
	check_cancelled(progress)?;
	let [first_boundary, second_boundary] = boundaries;
	// Ordinary viewport requests normally converge well below this ceiling. A
	// tight angular request on a complete periodic chart (for example, a sphere)
	// can legitimately require more than 128 intervals around the full normal
	// turn, so retain enough room for the deterministic power-of-two refinement
	// sequence while the face-wide vertex ceiling remains the final resource
	// bound.
	const MAXIMUM_AXIS_POINTS: usize = 513;
	const MAXIMUM_AXIS_REFINEMENT_PASSES: usize = 16;

	let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
	let (minimum, maximum) = match axis {
		ParametricAxis::U => (u_min, u_max),
		ParametricAxis::V => (v_min, v_max),
	};
	let seed = if first_boundary.len() >= second_boundary.len() { first_boundary } else { second_boundary };
	let mut coordinates = seed
		.iter()
		.map(|vertex| match axis {
			ParametricAxis::U => vertex.uv.x,
			ParametricAxis::V => vertex.uv.y,
		})
		.map(|coordinate| coordinate.clamp(minimum, maximum))
		.collect::<Vec<_>>();
	coordinates.push(minimum);
	coordinates.push(maximum);
	coordinates.sort_by(f64::total_cmp);
	coordinates.dedup_by(|first, second| (*first - *second).abs() <= sampling.coordinate_tolerance);
	if coordinates.len() < 2 {
		return Err(Error::TriangulationFailed);
	}
	coordinates[0] = minimum;
	let last = coordinates.len() - 1;
	coordinates[last] = maximum;

	for pass in 0..MAXIMUM_AXIS_REFINEMENT_PASSES {
		cancellation_checkpoint(progress, pass)?;
		if coordinates.len() >= MAXIMUM_AXIS_POINTS {
			break;
		}
		let mut insertions = Vec::new();
		for (interval_index, pair) in coordinates.windows(2).enumerate() {
			cancellation_checkpoint(progress, interval_index)?;
			let midpoint = (pair[0] + pair[1]) * 0.5;
			if midpoint > pair[0] && midpoint < pair[1] && axis_interval_needs_split(face, axis, [pair[0], midpoint, pair[1]], sampling, progress)? {
				insertions.push((pair[1] - pair[0], midpoint));
			}
		}
		if insertions.is_empty() {
			break;
		}
		// If the resource ceiling is reached, refine the widest unresolved
		// intervals globally. Truncating a low-to-high scan concentrates all
		// samples at one end of a periodic chart and leaves a large final cell.
		insertions.sort_by(|first, second| second.0.total_cmp(&first.0).then_with(|| first.1.total_cmp(&second.1)));
		insertions.truncate(MAXIMUM_AXIS_POINTS - coordinates.len());
		coordinates.extend(insertions.into_iter().map(|(_, midpoint)| midpoint));
		coordinates.sort_by(f64::total_cmp);
	}
	Ok(coordinates)
}

fn axis_interval_needs_split(face: &TrimmedFace, axis: ParametricAxis, interval: [f64; 3], sampling: AxisSampling, progress: &ffi::CancellationToken) -> Result<bool, Error> {
	check_cancelled(progress)?;
	let [first, midpoint, last] = interval;
	let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
	let cross_values = match axis {
		ParametricAxis::U => [v_min, v_min * 0.75 + v_max * 0.25, (v_min + v_max) * 0.5, v_min * 0.25 + v_max * 0.75, v_max],
		ParametricAxis::V => [u_min, u_min * 0.75 + u_max * 0.25, (u_min + u_max) * 0.5, u_min * 0.25 + u_max * 0.75, u_max],
	};
	for cross in cross_values {
		let first_sample = face.surface.evaluate(axis_uv(axis, first, cross)).ok_or(Error::TriangulationFailed)?;
		let middle_sample = face.surface.evaluate(axis_uv(axis, midpoint, cross)).ok_or(Error::TriangulationFailed)?;
		let last_sample = face.surface.evaluate(axis_uv(axis, last, cross)).ok_or(Error::TriangulationFailed)?;
		let path_length = first_sample.position.distance(middle_sample.position) + middle_sample.position.distance(last_sample.position);
		let deviation = point_segment_distance(middle_sample.position, first_sample.position, last_sample.position);
		let first_angle = surface_normal_angle(first_sample, middle_sample);
		let second_angle = surface_normal_angle(middle_sample, last_sample);
		if path_length > sampling.target_length || deviation > sampling.linear || first_angle > sampling.angular || second_angle > sampling.angular {
			return Ok(true);
		}
	}
	Ok(false)
}

fn axis_uv(axis: ParametricAxis, coordinate: f64, cross: f64) -> DVec2 {
	match axis {
		ParametricAxis::U => DVec2::new(coordinate, cross),
		ParametricAxis::V => DVec2::new(cross, coordinate),
	}
}

fn surface_normal_angle(first: SurfaceSample, second: SurfaceSample) -> f64 {
	match (first.normal(), second.normal()) {
		(Some(first), Some(second)) => first.dot(second).clamp(-1.0, 1.0).acos(),
		_ => 0.0,
	}
}

fn ensure_axis_interior(coordinates: &mut Vec<f64>, minimum: f64, maximum: f64) {
	if coordinates.len() < 4 {
		*coordinates = vec![minimum, minimum + (maximum - minimum) / 3.0, minimum + (maximum - minimum) * 2.0 / 3.0, maximum];
	}
}

fn decimate_axis_coordinates(coordinates: &[f64]) -> Vec<f64> {
	let mut result = Vec::with_capacity(coordinates.len() / 2 + 2);
	result.push(coordinates[0]);
	result.extend(coordinates.iter().copied().enumerate().skip(1).take(coordinates.len() - 2).filter_map(|(index, coordinate)| (index.is_multiple_of(2)).then_some(coordinate)));
	result.push(coordinates[coordinates.len() - 1]);
	result
}

fn resample_axis_coordinates(coordinates: &[f64], target_count: usize) -> Vec<f64> {
	if target_count >= coordinates.len() {
		return coordinates.to_vec();
	}
	let last = coordinates.len() - 1;
	(0..target_count)
		.map(|index| {
			let source_index = index * last / (target_count - 1);
			coordinates[source_index]
		})
		.collect()
}

fn coarsen_axis_coordinates(face: &TrimmedFace, axis: ParametricAxis, coordinates: &[f64], target_count: usize, linear: f64, angular: f64, progress: &ffi::CancellationToken) -> Result<Vec<f64>, Error> {
	let mut count = target_count.max(4).min(coordinates.len());
	loop {
		check_cancelled(progress)?;
		let candidate = resample_axis_coordinates(coordinates, count);
		if axis_coordinates_meet_error(face, axis, &candidate, linear * 0.5, angular * 0.5, progress)? {
			return Ok(candidate);
		}
		if count == coordinates.len() {
			return Ok(coordinates.to_vec());
		}
		count = ((count + coordinates.len()) / 2).max(count + 1).min(coordinates.len());
	}
}

fn axis_coordinates_meet_error(face: &TrimmedFace, axis: ParametricAxis, coordinates: &[f64], linear: f64, angular: f64, progress: &ffi::CancellationToken) -> Result<bool, Error> {
	let sampling = AxisSampling { target_length: f64::INFINITY, linear, angular, coordinate_tolerance: 0.0 };
	for (interval_index, interval) in coordinates.windows(2).enumerate() {
		cancellation_checkpoint(progress, interval_index)?;
		let midpoint = (interval[0] + interval[1]) * 0.5;
		if axis_interval_needs_split(face, axis, [interval[0], midpoint, interval[1]], sampling, progress)? {
			return Ok(false);
		}
	}
	Ok(true)
}

fn transition_axis_index(axis_length: usize, transition_ring_count: usize) -> usize {
	// Widen a refined collar sublinearly. Keeping the annulus fixed would pack
	// increasingly skinny triangles into one strip, while widening it by one
	// full adaptive interval per ring leaves the boundary-to-first-ring chord
	// unchanged and cannot converge on high-curvature loft edges. The square-root
	// schedule improves both dimensions: later candidates span a broader annulus
	// and reduce the first radial step deterministically.
	let desired = ((transition_ring_count as f64 * 8.0).sqrt().ceil() as usize).max(1);
	desired.min(axis_length.saturating_sub(2) / 3).max(1)
}

/// Audits the actual tensor rows and cells instead of assuming that a handful
/// of cross-axis probes represent the whole rational surface. A localized
/// mixed derivative can make one otherwise-valid interval fail only in a
/// narrow region. Deterministic midpoint insertion keeps the grid coherent,
/// chooses the less expensive refinement direction for a failing cell, and
/// remains bounded independently of surface complexity.
#[derive(Clone, Copy)]
struct TensorSampling {
	u_tolerance: f64,
	v_tolerance: f64,
	linear: f64,
	angular: f64,
}

fn refine_tensor_coordinates(face: &TrimmedFace, u_coordinates: &mut Vec<f64>, v_coordinates: &mut Vec<f64>, sampling: TensorSampling, progress: &ffi::CancellationToken) -> Result<bool, Error> {
	const MAXIMUM_TENSOR_AXIS_POINTS: usize = 513;
	const MAXIMUM_TENSOR_CELLS: usize = 131_072;
	const MAXIMUM_TENSOR_REFINEMENT_PASSES: usize = 8;
	let TensorSampling { u_tolerance, v_tolerance, linear, angular } = sampling;

	for pass in 0..MAXIMUM_TENSOR_REFINEMENT_PASSES {
		cancellation_checkpoint(progress, pass)?;
		let mut u_insertions = Vec::new();
		for (u_interval_index, interval) in u_coordinates.windows(2).enumerate() {
			cancellation_checkpoint(progress, u_interval_index)?;
			let midpoint = (interval[0] + interval[1]) * 0.5;
			let mut needs_split = false;
			for (v_index, v) in v_coordinates.iter().copied().enumerate() {
				cancellation_checkpoint(progress, v_index)?;
				let first = face.surface.evaluate_position(DVec2::new(interval[0], v)).ok_or(Error::TriangulationFailed)?;
				let middle = face.surface.evaluate_position(DVec2::new(midpoint, v)).ok_or(Error::TriangulationFailed)?;
				let last = face.surface.evaluate_position(DVec2::new(interval[1], v)).ok_or(Error::TriangulationFailed)?;
				if point_segment_distance(middle, first, last) > linear {
					needs_split = true;
					break;
				}
			}
			if needs_split {
				u_insertions.push(midpoint);
			}
		}

		let mut v_insertions = Vec::new();
		for (v_interval_index, interval) in v_coordinates.windows(2).enumerate() {
			cancellation_checkpoint(progress, v_interval_index)?;
			let midpoint = (interval[0] + interval[1]) * 0.5;
			let mut needs_split = false;
			for (u_index, u) in u_coordinates.iter().copied().enumerate() {
				cancellation_checkpoint(progress, u_index)?;
				let first = face.surface.evaluate_position(DVec2::new(u, interval[0])).ok_or(Error::TriangulationFailed)?;
				let middle = face.surface.evaluate_position(DVec2::new(u, midpoint)).ok_or(Error::TriangulationFailed)?;
				let last = face.surface.evaluate_position(DVec2::new(u, interval[1])).ok_or(Error::TriangulationFailed)?;
				if point_segment_distance(middle, first, last) > linear {
					needs_split = true;
					break;
				}
			}
			if needs_split {
				v_insertions.push(midpoint);
			}
		}

		// Row-wise chord checks cannot see error introduced by the diagonal of a
		// doubly curved cell. Audit both possible diagonal splits against the exact
		// surface. When neither split meets the requested error, compare virtual U
		// and V subdivisions and insert only the midpoint that lowers the worst
		// child-cell error most. This produces a graded tensor mesh instead of
		// abandoning a well-structured face for an unconstrained point cloud.
		for (u_interval_index, u_interval) in u_coordinates.windows(2).enumerate() {
			cancellation_checkpoint(progress, u_interval_index)?;
			for (v_interval_index, v_interval) in v_coordinates.windows(2).enumerate() {
				cancellation_checkpoint(progress, v_interval_index)?;
				let current = tensor_cell_error(face, u_interval[0], u_interval[1], v_interval[0], v_interval[1], linear, angular)?;
				if current <= 1.0 {
					continue;
				}
				let u_midpoint = (u_interval[0] + u_interval[1]) * 0.5;
				let v_midpoint = (v_interval[0] + v_interval[1]) * 0.5;
				let u_error = tensor_cell_error(face, u_interval[0], u_midpoint, v_interval[0], v_interval[1], linear, angular)?.max(tensor_cell_error(face, u_midpoint, u_interval[1], v_interval[0], v_interval[1], linear, angular)?);
				let v_error = tensor_cell_error(face, u_interval[0], u_interval[1], v_interval[0], v_midpoint, linear, angular)?.max(tensor_cell_error(face, u_interval[0], u_interval[1], v_midpoint, v_interval[1], linear, angular)?);
				if u_error <= v_error {
					u_insertions.push(u_midpoint);
				} else {
					v_insertions.push(v_midpoint);
				}
			}
		}

		if u_insertions.is_empty() && v_insertions.is_empty() {
			return Ok(true);
		}
		let next_u_count = u_coordinates.len().saturating_add(u_insertions.len());
		let next_v_count = v_coordinates.len().saturating_add(v_insertions.len());
		if next_u_count > MAXIMUM_TENSOR_AXIS_POINTS || next_v_count > MAXIMUM_TENSOR_AXIS_POINTS || next_u_count.saturating_sub(1).saturating_mul(next_v_count.saturating_sub(1)) > MAXIMUM_TENSOR_CELLS {
			return Ok(false);
		}
		u_coordinates.extend(u_insertions);
		u_coordinates.sort_by(f64::total_cmp);
		u_coordinates.dedup_by(|first, second| (*first - *second).abs() <= u_tolerance);
		v_coordinates.extend(v_insertions);
		v_coordinates.sort_by(f64::total_cmp);
		v_coordinates.dedup_by(|first, second| (*first - *second).abs() <= v_tolerance);
	}
	Ok(false)
}

fn tensor_cell_error(face: &TrimmedFace, u_min: f64, u_max: f64, v_min: f64, v_max: f64, linear: f64, angular: f64) -> Result<f64, Error> {
	let uvs = [DVec2::new(u_min, v_min), DVec2::new(u_max, v_min), DVec2::new(u_max, v_max), DVec2::new(u_min, v_max)];
	let positions = uvs.map(|uv| face.surface.evaluate_position(uv).ok_or(Error::TriangulationFailed)).into_iter().collect::<Result<Vec<_>, _>>()?;
	let positions: [DVec3; 4] = positions.try_into().map_err(|_| Error::TriangulationFailed)?;
	let first = [[0, 1, 2], [0, 2, 3]];
	let second = [[0, 1, 3], [1, 2, 3]];
	let split_error = |split: [[usize; 3]; 2]| -> Result<f64, Error> {
		split.into_iter().try_fold(0.0_f64, |worst, triangle| {
			let error = strip_triangle_error(face, triangle.map(|index| uvs[index]), triangle.map(|index| positions[index]), linear, angular)?;
			Ok(worst.max(error))
		})
	};
	Ok(split_error(first)?.min(split_error(second)?))
}

fn regularize_axis_coordinates(coordinates: &[f64]) -> Vec<f64> {
	if coordinates.len() < 4 {
		return coordinates.to_vec();
	}
	let mut intervals = coordinates.windows(2).map(|pair| pair[1] - pair[0]).filter(|interval| interval.is_finite() && *interval > 0.0).collect::<Vec<_>>();
	if intervals.is_empty() {
		return coordinates.to_vec();
	}
	intervals.sort_by(f64::total_cmp);
	let minimum_interval = intervals[intervals.len() / 2] * 0.08;
	if !minimum_interval.is_finite() || minimum_interval <= 0.0 {
		return coordinates.to_vec();
	}
	let mut result = Vec::with_capacity(coordinates.len());
	result.push(coordinates[0]);
	for coordinate in coordinates.iter().copied().skip(1).take(coordinates.len() - 2) {
		if coordinate - result[result.len() - 1] >= minimum_interval {
			result.push(coordinate);
		}
	}
	let last = coordinates[coordinates.len() - 1];
	if result.len() > 1 && last - result[result.len() - 1] < minimum_interval {
		result.pop();
	}
	result.push(last);
	result
}

fn metric_distance(first: Point2<f64>, second: Point2<f64>) -> f64 {
	((second.x - first.x).powi(2) + (second.y - first.y).powi(2)).sqrt()
}

fn append_boundary_side(face: &TrimmedFace, vertices_on_side: &[&BoundaryVertex], use_u: bool, vertices: &mut Vec<DVec3>, uvs: &mut Vec<DVec2>, normals: &mut Vec<DVec3>, progress: &ffi::CancellationToken) -> Result<Vec<(f64, usize)>, Error> {
	let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
	vertices_on_side
		.iter()
		.enumerate()
		.map(|(index, vertex)| {
			cancellation_checkpoint(progress, index)?;
			let index = append_surface_vertex(face, vertex.uv, Some(vertex.position), vertices, uvs, normals)?;
			let key = if use_u { parameter_fraction(vertex.uv.x, u_min, u_max) } else { parameter_fraction(vertex.uv.y, v_min, v_max) };
			Ok((key, index))
		})
		.collect()
}

fn weld_exact_boundary_corners(sides: [&mut Vec<(f64, usize)>; 4], vertices: &[DVec3], uvs: &[DVec2]) -> Result<(), Error> {
	fn coordinate_key(value: f64) -> u64 {
		if value == 0.0 {
			0
		} else {
			value.to_bits()
		}
	}

	let mut canonical = BTreeMap::<(u64, u64, PointKey), usize>::new();
	for side in sides {
		if side.len() < 2 {
			return Err(Error::TriangulationFailed);
		}
		for endpoint in [0, side.len() - 1] {
			let index = side[endpoint].1;
			let uv = *uvs.get(index).ok_or(Error::TriangulationFailed)?;
			let position = *vertices.get(index).ok_or(Error::TriangulationFailed)?;
			let key = (coordinate_key(uv.x), coordinate_key(uv.y), point_key(position));
			match canonical.get(&key).copied() {
				Some(existing) => side[endpoint].1 = existing,
				None => {
					canonical.insert(key, index);
				}
			}
		}
	}
	Ok(())
}

fn weld_exact_mesh_vertex_indices(vertices: &[DVec3], uvs: &[DVec2], indices: &mut Vec<u32>, progress: &ffi::CancellationToken) -> Result<(), Error> {
	fn coordinate_key(value: f64) -> u64 {
		if value == 0.0 {
			0
		} else {
			value.to_bits()
		}
	}

	if vertices.len() != uvs.len() || !indices.len().is_multiple_of(3) {
		return Err(Error::TriangulationFailed);
	}
	let mut canonical = BTreeMap::<(u64, u64, PointKey), u32>::new();
	let mut replacements = Vec::with_capacity(vertices.len());
	for (index, (position, uv)) in vertices.iter().zip(uvs).enumerate() {
		cancellation_checkpoint(progress, index)?;
		let index = u32::try_from(index).map_err(|_| Error::TriangulationFailed)?;
		let key = (coordinate_key(uv.x), coordinate_key(uv.y), point_key(*position));
		let canonical = *canonical.entry(key).or_insert(index);
		replacements.push(canonical);
	}
	for (ordinal, index) in indices.iter_mut().enumerate() {
		cancellation_checkpoint(progress, ordinal)?;
		*index = *replacements.get(*index as usize).ok_or(Error::TriangulationFailed)?;
	}
	*indices = indices.chunks_exact(3).filter(|triangle| triangle[0] != triangle[1] && triangle[1] != triangle[2] && triangle[2] != triangle[0]).flatten().copied().collect();
	Ok(())
}

#[derive(Clone, Copy)]
enum PatchSide {
	Lower,
	Upper,
	Left,
	Right,
}

/// A collapsed chart side (sphere pole or cone apex) is one geometric point
/// even though its UV polyline may contain only a few canonical edge samples.
/// Give it the adjacent regular row's parameter keys, all mapped to that exact
/// point, so every interior column closes independently at the singularity.
/// This avoids a sparse-density transition fan that jumps across latitude
/// rings without changing the geometric shared boundary.
#[allow(clippy::too_many_arguments)]
fn augment_collapsed_boundary(face: &TrimmedFace, boundary: &[(f64, usize)], inner: &[(f64, usize)], side: PatchSide, vertices: &mut Vec<DVec3>, uvs: &mut Vec<DVec2>, normals: &mut Vec<DVec3>, progress: &ffi::CancellationToken) -> Result<Vec<(f64, usize)>, Error> {
	check_cancelled(progress)?;
	let Some((_, first_index)) = boundary.first().copied() else {
		return Err(Error::TriangulationFailed);
	};
	let pole = vertices[first_index];
	let maximum_diameter = boundary.iter().map(|(_, index)| vertices[*index].distance(pole)).fold(0.0, f64::max);
	let scale = vertices.iter().map(|point| point.distance(pole)).fold(0.0, f64::max).max(1.0);
	if maximum_diameter > scale * 1.0e-12 {
		return Ok(boundary.to_vec());
	}

	let mut keys = boundary.iter().chain(inner).map(|entry| entry.0).collect::<Vec<_>>();
	keys.sort_by(f64::total_cmp);
	keys.dedup_by(|first, second| (*first - *second).abs() <= 1.0e-12);
	let additional_vertices = keys.iter().filter(|key| !boundary.iter().any(|entry| (entry.0 - **key).abs() <= 1.0e-12)).count();
	let planned_vertex_count = checked_add_resource(vertices.len(), additional_vertices, "tessellation collapsed boundary vertex count overflowed")?;
	if planned_vertex_count > MAXIMUM_FACE_VERTICES {
		return Err(resource_limit("tessellation collapsed boundary exceeded the structured vertex limit"));
	}
	reserve_exact(vertices, additional_vertices, "tessellation collapsed boundary vertex allocation failed")?;
	reserve_exact(uvs, additional_vertices, "tessellation collapsed boundary parameter allocation failed")?;
	reserve_exact(normals, additional_vertices, "tessellation collapsed boundary normal allocation failed")?;
	let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
	keys.into_iter()
		.enumerate()
		.map(|(key_index, key)| {
			cancellation_checkpoint(progress, key_index)?;
			if let Some(entry) = boundary.iter().find(|entry| (entry.0 - key).abs() <= 1.0e-12) {
				return Ok(*entry);
			}
			let uv = match side {
				PatchSide::Lower => DVec2::new(u_min + (u_max - u_min) * key, v_min),
				PatchSide::Upper => DVec2::new(u_min + (u_max - u_min) * key, v_max),
				PatchSide::Left => DVec2::new(u_min, v_min + (v_max - v_min) * key),
				PatchSide::Right => DVec2::new(u_max, v_min + (v_max - v_min) * key),
			};
			let index = append_surface_vertex(face, uv, Some(pole), vertices, uvs, normals)?;
			Ok((key, index))
		})
		.collect()
}

#[allow(clippy::too_many_arguments)]
fn append_transition_rings(face: &TrimmedFace, boundary: &[(f64, usize)], inner: &[(f64, usize)], side: PatchSide, core_u_min: f64, core_u_max: f64, core_v_min: f64, core_v_max: f64, transition_ring_count: usize, vertices: &mut Vec<DVec3>, uvs: &mut Vec<DVec2>, normals: &mut Vec<DVec3>, progress: &ffi::CancellationToken) -> Result<Vec<Vec<(f64, usize)>>, Error> {
	check_cancelled(progress)?;
	let boundary_keys = boundary.iter().map(|entry| entry.0).collect::<Vec<_>>();
	let inner_keys = inner.iter().map(|entry| entry.0).collect::<Vec<_>>();
	let key_stages = transition_key_stages(&boundary_keys, &inner_keys, transition_ring_count, progress)?;
	let [face_u_min, face_u_max, face_v_min, face_v_max] = face.surface.uv_bounds;
	let stage_count = key_stages.len();
	key_stages
		.into_iter()
		.enumerate()
		.map(|(ordinal, keys)| {
			cancellation_checkpoint(progress, ordinal)?;
			let fraction = (ordinal + 1) as f64 / (stage_count + 1) as f64;
			let u_min = face_u_min + (core_u_min - face_u_min) * fraction;
			let u_max = face_u_max + (core_u_max - face_u_max) * fraction;
			let v_min = face_v_min + (core_v_min - face_v_min) * fraction;
			let v_max = face_v_max + (core_v_max - face_v_max) * fraction;
			keys.into_iter()
				.enumerate()
				.map(|(key_index, key)| {
					cancellation_checkpoint(progress, key_index)?;
					let uv = match side {
						PatchSide::Lower => DVec2::new(u_min + (u_max - u_min) * key, v_min),
						PatchSide::Upper => DVec2::new(u_min + (u_max - u_min) * key, v_max),
						PatchSide::Left => DVec2::new(u_min, v_min + (v_max - v_min) * key),
						PatchSide::Right => DVec2::new(u_max, v_min + (v_max - v_min) * key),
					};
					let index = append_surface_vertex(face, uv, None, vertices, uvs, normals)?;
					Ok((key, index))
				})
				.collect()
		})
		.collect()
}

/// Gives every logical collar corner one topological vertex per transition
/// stage. Adjacent sides derive the same mathematical corner through a
/// different interpolation expression; those expressions can differ by one
/// ULP and must not be used as topology identity. Sharing by side and stage is
/// exact by construction and leaves non-corner samples untouched.
fn share_transition_ring_corners(lower: &mut [Vec<(f64, usize)>], upper: &mut [Vec<(f64, usize)>], left: &mut [Vec<(f64, usize)>], right: &mut [Vec<(f64, usize)>]) -> Result<(), Error> {
	if lower.len() != upper.len() || lower.len() != left.len() || lower.len() != right.len() {
		return Err(Error::TriangulationFailed);
	}
	for stage in 0..lower.len() {
		if lower[stage].len() < 2 || upper[stage].len() < 2 || left[stage].len() < 2 || right[stage].len() < 2 {
			return Err(Error::TriangulationFailed);
		}
		let lower_left = lower[stage][0].1;
		let lower_right = lower[stage].last().ok_or(Error::TriangulationFailed)?.1;
		let upper_left = upper[stage][0].1;
		let upper_right = upper[stage].last().ok_or(Error::TriangulationFailed)?.1;
		left[stage][0].1 = lower_left;
		right[stage][0].1 = lower_right;
		let left_last = left[stage].len() - 1;
		let right_last = right[stage].len() - 1;
		left[stage][left_last].1 = upper_left;
		right[stage][right_last].1 = upper_right;
	}
	Ok(())
}

fn transition_key_stages(boundary: &[f64], inner: &[f64], transition_ring_count: usize, progress: &ffi::CancellationToken) -> Result<Vec<Vec<f64>>, Error> {
	// A shallow collar makes the radial spacing comparable to even the densest
	// canonical trim intervals. Morph quantile-corresponding samples from the
	// boundary distribution to the regular interior distribution. This retires
	// curvature-driven micro-intervals gradually instead of propagating their
	// union through the whole annulus or changing density in one abrupt strip.
	if boundary.len() < 2 || inner.len() < 2 || transition_ring_count == 0 || !strictly_increasing(boundary) || !strictly_increasing(inner) {
		return Err(Error::TriangulationFailed);
	}
	let mut stages = Vec::with_capacity(transition_ring_count);
	for stage in 1..=transition_ring_count {
		cancellation_checkpoint(progress, stage - 1)?;
		let blend = stage as f64 / (transition_ring_count + 1) as f64;
		// Grow interval counts geometrically so no early strip absorbs most of
		// the boundary/core density change as one high-valence fan.
		let sample_count = transition_stage_sample_count(boundary.len(), inner.len(), blend);
		let mut keys = Vec::with_capacity(sample_count);
		for index in 0..sample_count {
			cancellation_checkpoint(progress, index)?;
			let quantile = index as f64 / (sample_count - 1) as f64;
			let boundary_key = sample_key_distribution(boundary, quantile);
			let inner_key = sample_key_distribution(inner, quantile);
			keys.push(boundary_key + (inner_key - boundary_key) * blend);
		}
		if !strictly_increasing(&keys) {
			return Err(Error::TriangulationFailed);
		}
		stages.push(keys);
	}
	Ok(stages)
}

fn transition_stage_vertex_count(boundary_count: usize, inner_count: usize, transition_ring_count: usize) -> Result<usize, Error> {
	if boundary_count < 2 || inner_count < 2 || transition_ring_count == 0 {
		return Err(Error::TriangulationFailed);
	}
	let mut total = 0usize;
	for stage in 1..=transition_ring_count {
		let blend = stage as f64 / (transition_ring_count + 1) as f64;
		let sample_count = transition_stage_sample_count(boundary_count, inner_count, blend);
		total = checked_add_resource(total, sample_count, "tessellation transition-ring vertex count overflowed")?;
	}
	Ok(total)
}

fn transition_stage_sample_count(boundary_count: usize, inner_count: usize, blend: f64) -> usize {
	let boundary_intervals = (boundary_count - 1) as f64;
	let inner_intervals = (inner_count - 1) as f64;
	(boundary_intervals.ln() + (inner_intervals.ln() - boundary_intervals.ln()) * blend).exp().round() as usize + 1
}

fn sample_key_distribution(keys: &[f64], quantile: f64) -> f64 {
	let coordinate = quantile.clamp(0.0, 1.0) * (keys.len() - 1) as f64;
	let lower = coordinate.floor() as usize;
	let upper = (lower + 1).min(keys.len() - 1);
	let blend = coordinate - lower as f64;
	keys[lower] + (keys[upper] - keys[lower]) * blend
}

#[allow(clippy::too_many_arguments)]
fn triangulate_transition_rings(face: &TrimmedFace, boundary: &[(f64, usize)], rings: &[Vec<(f64, usize)>], inner: &[(f64, usize)], outer_to_inner: bool, vertices: &[DVec3], uvs: &[DVec2], normals: &[DVec3], linear: f64, angular: f64, indices: &mut Vec<u32>, progress: &ffi::CancellationToken) -> Result<(), Error> {
	let mut previous = boundary;
	for (ring_index, ring) in rings.iter().enumerate() {
		cancellation_checkpoint(progress, ring_index)?;
		// Only equal-density generated rings have quantile-corresponding vertices.
		// Density-changing stages must use their actual parameter keys.
		let equal_density = previous.len() == ring.len();
		let tolerance = if ring_index > 0 && equal_density { f64::INFINITY } else { 1.0e-10 };
		if outer_to_inner {
			triangulate_monotone_strip(face, previous, ring, vertices, uvs, normals, tolerance, linear, angular, indices, progress)?;
		} else {
			triangulate_monotone_strip(face, ring, previous, vertices, uvs, normals, tolerance, linear, angular, indices, progress)?;
		}
		previous = ring;
	}
	if outer_to_inner {
		triangulate_monotone_strip(face, previous, inner, vertices, uvs, normals, 1.0e-10, linear, angular, indices, progress)
	} else {
		triangulate_monotone_strip(face, inner, previous, vertices, uvs, normals, 1.0e-10, linear, angular, indices, progress)
	}
}

fn parameter_fraction(parameter: f64, minimum: f64, maximum: f64) -> f64 {
	(parameter - minimum) / (maximum - minimum)
}

fn append_surface_vertex(face: &TrimmedFace, uv: DVec2, boundary_position: Option<DVec3>, vertices: &mut Vec<DVec3>, uvs: &mut Vec<DVec2>, normals: &mut Vec<DVec3>) -> Result<usize, Error> {
	let sample = face.surface.evaluate(uv).ok_or(Error::TriangulationFailed)?;
	let normal = oriented_surface_normal_from_sample(face, uv, sample).unwrap_or(DVec3::ZERO);
	let index = vertices.len();
	vertices.push(boundary_position.unwrap_or(sample.position));
	uvs.push(uv);
	normals.push(normal);
	Ok(index)
}

fn mean_coordinate(vertices: &[&BoundaryVertex], u: bool) -> f64 {
	vertices.iter().map(|vertex| if u { vertex.uv.x } else { vertex.uv.y }).sum::<f64>() / vertices.len() as f64
}

fn strictly_increasing(values: &[f64]) -> bool {
	values.windows(2).all(|pair| pair[0] < pair[1])
}

#[allow(clippy::too_many_arguments)]
fn triangulate_monotone_strip(face: &TrimmedFace, left: &[(f64, usize)], right: &[(f64, usize)], vertices: &[DVec3], uvs: &[DVec2], normals: &[DVec3], tolerance: f64, linear: f64, angular: f64, indices: &mut Vec<u32>, progress: &ffi::CancellationToken) -> Result<(), Error> {
	check_cancelled(progress)?;
	if left.len() < 2 || right.len() < 2 || !strictly_increasing(&left.iter().map(|entry| entry.0).collect::<Vec<_>>()) || !strictly_increasing(&right.iter().map(|entry| entry.0).collect::<Vec<_>>()) {
		return Err(Error::TriangulationFailed);
	}
	let samples_differ = left.len() != right.len() || left.iter().zip(right).any(|(first, second)| (first.0 - second.0).abs() > tolerance);
	let smaller_row = left.len().min(right.len());
	let larger_row = left.len().max(right.len());
	// The full dynamic path is valuable when a sparse constrained boundary must
	// meet a much denser interior row. Adjacent adaptive rows with comparable
	// density need only the ordered linear zipper below; exploring their entire
	// Cartesian product turns an otherwise linear periodic sphere into cubic
	// work without changing the local connectivity choice.
	if samples_differ && larger_row > smaller_row.saturating_mul(2) {
		return triangulate_unequal_monotone_strip(face, left, right, vertices, uvs, normals, linear, angular, indices, progress);
	}

	let mut left_index = 0;
	let mut right_index = 0;
	let mut step_index = 0;
	while left_index + 1 < left.len() || right_index + 1 < right.len() {
		cancellation_checkpoint(progress, step_index)?;
		step_index += 1;
		let next_left = left.get(left_index + 1).map(|entry| entry.0);
		let next_right = right.get(right_index + 1).map(|entry| entry.0);
		match (next_left, next_right) {
			(Some(left_v), Some(right_v)) if (left_v - right_v).abs() <= tolerance => {
				let lower_left = left[left_index].1;
				let lower_right = right[right_index].1;
				let upper_left = left[left_index + 1].1;
				let upper_right = right[right_index + 1].1;
				let first_split = [[lower_left, lower_right, upper_right], [lower_left, upper_right, upper_left]];
				let second_split = [[lower_left, lower_right, upper_left], [lower_right, upper_right, upper_left]];
				// Matched tensor cells already meet the adaptive U/V error tests.
				// Choose their diagonal from the stable vertex layout instead of
				// re-scoring world-space coordinates: sub-ULP evaluation differences
				// after scaling or placement must not change connectivity. Unequal
				// boundary rows still use the minimax dynamic zipper below.
				let split = if (lower_left + lower_right).is_multiple_of(2) { first_split } else { second_split };
				for triangle in split {
					push_chart_oriented_triangle(face, triangle, vertices, uvs, normals, indices)?;
				}
				left_index += 1;
				right_index += 1;
			}
			(Some(left_v), Some(right_v)) if left_v < right_v => {
				push_chart_oriented_triangle(face, [left[left_index].1, right[right_index].1, left[left_index + 1].1], vertices, uvs, normals, indices)?;
				left_index += 1;
			}
			(Some(_), Some(_)) => {
				push_chart_oriented_triangle(face, [left[left_index].1, right[right_index].1, right[right_index + 1].1], vertices, uvs, normals, indices)?;
				right_index += 1;
			}
			(Some(_), None) => {
				push_chart_oriented_triangle(face, [left[left_index].1, right[right_index].1, left[left_index + 1].1], vertices, uvs, normals, indices)?;
				left_index += 1;
			}
			(None, Some(_)) => {
				push_chart_oriented_triangle(face, [left[left_index].1, right[right_index].1, right[right_index + 1].1], vertices, uvs, normals, indices)?;
				right_index += 1;
			}
			(None, None) => break,
		}
	}
	Ok(())
}

#[derive(Clone, Copy)]
struct StripScore {
	worst_error: f64,
	worst_aspect: f64,
	total_aspect: f64,
}

fn strip_score_is_better(candidate: StripScore, current: StripScore) -> bool {
	fn meaningfully_less(first: f64, second: f64) -> bool {
		let tolerance = first.abs().max(second.abs()).max(1.0) * 1.0e-6;
		first < second - tolerance
	}

	if meaningfully_less(candidate.worst_error, current.worst_error) {
		return true;
	}
	if meaningfully_less(current.worst_error, candidate.worst_error) {
		return false;
	}
	if meaningfully_less(candidate.worst_aspect, current.worst_aspect) {
		return true;
	}
	if meaningfully_less(current.worst_aspect, candidate.worst_aspect) {
		return false;
	}
	// Sub-ppm score differences can come solely from evaluating a small face
	// at a large world-coordinate offset. Prefer the first deterministic split
	// in that equivalence class instead of changing topology after placement.
	!meaningfully_less(current.total_aspect, candidate.total_aspect)
}

#[derive(Clone, Copy)]
enum StripStep {
	Left,
	Right,
}

/// Triangulates the polygon between two ordered polylines without assuming
/// that their parameter samples line up. A key-ordered zipper always joins a
/// long constrained boundary segment to a point near one endpoint, creating
/// an avoidable sliver. This dynamic monotone path instead minimizes the worst
/// physical triangle aspect, so a sparse segment can meet a central sample on
/// the denser row while retaining deterministic, non-crossing connectivity.
#[allow(clippy::too_many_arguments)]
fn triangulate_unequal_monotone_strip(face: &TrimmedFace, left: &[(f64, usize)], right: &[(f64, usize)], vertices: &[DVec3], uvs: &[DVec2], normals: &[DVec3], linear: f64, angular: f64, indices: &mut Vec<u32>, progress: &ffi::CancellationToken) -> Result<(), Error> {
	check_cancelled(progress)?;
	const MAXIMUM_STRIP_STATES: usize = 262_144;
	let column_count = right.len();
	let state_count = left.len().checked_mul(column_count).ok_or(Error::TriangulationFailed)?;
	if state_count > MAXIMUM_STRIP_STATES {
		return Err(Error::TriangulationFailed);
	}
	let mut scores = vec![None::<StripScore>; state_count];
	let mut predecessors = vec![None::<StripStep>; state_count];
	scores[0] = Some(StripScore { worst_error: 1.0, worst_aspect: 0.0, total_aspect: 0.0 });

	for left_index in 0..left.len() {
		for right_index in 0..right.len() {
			let state = left_index * column_count + right_index;
			cancellation_checkpoint(progress, state)?;
			let Some(score) = scores[state] else {
				continue;
			};
			if left_index + 1 < left.len() {
				let triangle = [left[left_index].1, right[right_index].1, left[left_index + 1].1];
				consider_strip_step(face, score, triangle, StripStep::Left, left_index + 1, right_index, column_count, vertices, uvs, linear, angular, &mut scores, &mut predecessors)?;
			}
			if right_index + 1 < right.len() {
				let triangle = [left[left_index].1, right[right_index].1, right[right_index + 1].1];
				consider_strip_step(face, score, triangle, StripStep::Right, left_index, right_index + 1, column_count, vertices, uvs, linear, angular, &mut scores, &mut predecessors)?;
			}
		}
	}

	let mut left_index = left.len() - 1;
	let mut right_index = right.len() - 1;
	if scores[left_index * column_count + right_index].is_none() {
		return Err(Error::TriangulationFailed);
	}
	let mut triangles = Vec::with_capacity(left.len() + right.len() - 2);
	let mut step_index = 0;
	while left_index != 0 || right_index != 0 {
		cancellation_checkpoint(progress, step_index)?;
		step_index += 1;
		let step = predecessors[left_index * column_count + right_index].ok_or(Error::TriangulationFailed)?;
		match step {
			StripStep::Left => {
				triangles.push([left[left_index - 1].1, right[right_index].1, left[left_index].1]);
				left_index -= 1;
			}
			StripStep::Right => {
				triangles.push([left[left_index].1, right[right_index - 1].1, right[right_index].1]);
				right_index -= 1;
			}
		}
	}
	for (triangle_index, triangle) in triangles.into_iter().rev().enumerate() {
		cancellation_checkpoint(progress, triangle_index)?;
		push_chart_oriented_triangle(face, triangle, vertices, uvs, normals, indices)?;
	}
	Ok(())
}

#[allow(clippy::too_many_arguments)]
fn consider_strip_step(face: &TrimmedFace, score: StripScore, triangle: [usize; 3], step: StripStep, next_left: usize, next_right: usize, column_count: usize, vertices: &[DVec3], uvs: &[DVec2], linear: f64, angular: f64, scores: &mut [Option<StripScore>], predecessors: &mut [Option<StripStep>]) -> Result<(), Error> {
	let points = triangle.map(|index| vertices[index]);
	let area_squared = (points[1] - points[0]).cross(points[2] - points[0]).length_squared();
	let aspect = if area_squared <= triangle_area_threshold(points) {
		if triangle_has_collapsed_edge(points) {
			0.0
		} else {
			return Ok(());
		}
	} else {
		triangle_aspect(points)
	};
	if !aspect.is_finite() {
		return Ok(());
	}
	let error = strip_triangle_error(face, triangle.map(|index| uvs[index]), points, linear, angular)?;
	let candidate = StripScore { worst_error: score.worst_error.max(error).max(1.0), worst_aspect: score.worst_aspect.max(aspect), total_aspect: score.total_aspect + aspect };
	let index = next_left * column_count + next_right;
	let improves = scores[index].is_none_or(|current| strip_score_is_better(candidate, current));
	if improves {
		scores[index] = Some(candidate);
		predecessors[index] = Some(step);
	}
	Ok(())
}

fn strip_triangle_error(face: &TrimmedFace, uvs: [DVec2; 3], positions: [DVec3; 3], linear: f64, angular: f64) -> Result<f64, Error> {
	let usable_linear = (linear - face.surface.approximation_error).max(linear * 0.20).max(1.0e-10);
	let usable_angular = angular.max(1.0e-3);
	let center_uv = regularized_triangle_center_uv(face, uvs);
	let center = face.surface.evaluate(center_uv).ok_or(Error::TriangulationFailed)?;
	let mut error = point_triangle_distance(center.position, positions) / usable_linear;
	if let (Some(geometric), Some(expected)) = ((positions[1] - positions[0]).cross(positions[2] - positions[0]).try_normalize(), oriented_surface_normal_from_sample(face, center_uv, center)) {
		error = error.max(geometric.dot(expected).abs().clamp(-1.0, 1.0).acos() / usable_angular);
	}
	for edge in 0..3 {
		let next = (edge + 1) % 3;
		let midpoint_uv = regularized_edge_midpoint_uv(face, uvs[edge], uvs[next]);
		let midpoint = face.surface.evaluate_position(midpoint_uv).ok_or(Error::TriangulationFailed)?;
		error = error.max(point_segment_distance(midpoint, positions[edge], positions[next]) / usable_linear);
	}
	Ok(error)
}

fn push_surface_oriented_triangle(mut triangle: [usize; 3], vertices: &[DVec3], normals: &[DVec3], indices: &mut Vec<u32>) -> Result<(), Error> {
	let points = triangle.map(|index| vertices[index]);
	let area = (points[1] - points[0]).cross(points[2] - points[0]);
	if !area.is_finite() {
		return Err(Error::TriangulationFailed);
	}
	if area.length_squared() <= triangle_area_threshold(points) {
		if triangle_has_collapsed_edge(points) {
			// A sphere pole, cone apex, or another explicitly collapsed chart
			// boundary has a zero-area parametric cell by construction. Its
			// neighboring non-degenerate cells close the geometric surface.
			return Ok(());
		}
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation rejected non-collapsed zero-area triangle {triangle:?}, points {points:?}");
		}
		return Err(Error::TriangulationFailed);
	}
	let expected = triangle.into_iter().map(|index| normals[index]).sum::<DVec3>();
	if expected.length_squared() > 1.0e-24 && area.dot(expected) < 0.0 {
		triangle.swap(1, 2);
	}
	for index in triangle {
		indices.push(u32::try_from(index).map_err(|_| Error::TriangulationFailed)?);
	}
	Ok(())
}

fn push_chart_oriented_triangle(face: &TrimmedFace, mut triangle: [usize; 3], vertices: &[DVec3], uvs: &[DVec2], _normals: &[DVec3], indices: &mut Vec<u32>) -> Result<(), Error> {
	let chart = triangle.map(|index| uvs[index]);
	let chart_area = (chart[1] - chart[0]).perp_dot(chart[2] - chart[0]);
	if !chart_area.is_finite() {
		return Err(Error::TriangulationFailed);
	}
	let target_direction = if face.reversed { -1.0 } else { 1.0 };
	if chart_area * target_direction < 0.0 {
		triangle.swap(1, 2);
	}

	let points = triangle.map(|index| vertices[index]);
	let area = (points[1] - points[0]).cross(points[2] - points[0]);
	if !area.is_finite() {
		return Err(Error::TriangulationFailed);
	}
	if area.length_squared() <= triangle_area_threshold(points) {
		if triangle_has_collapsed_edge(points) {
			return Ok(());
		}
		return Err(Error::TriangulationFailed);
	}
	// Surface topology, not a point differential, defines winding. A formally
	// smooth procedural spline can reverse its differential inside a tiny cell
	// around a hard profile turn; rejecting the chart-oriented cell here would
	// discard a watertight structured grid and replace it with asymptotic local
	// refinement. The final tolerance audit checks exact chord and normal error,
	// including the explicit sub-deflection cusp rule, before accepting the mesh.
	for index in triangle {
		indices.push(u32::try_from(index).map_err(|_| Error::TriangulationFailed)?);
	}
	Ok(())
}

fn triangle_area_threshold(points: [DVec3; 3]) -> f64 {
	let maximum_edge_squared = [(points[1] - points[0]).length_squared(), (points[2] - points[1]).length_squared(), (points[0] - points[2]).length_squared()].into_iter().fold(0.0, f64::max);
	(maximum_edge_squared * maximum_edge_squared * 1.0e-24).max(1.0e-30)
}

fn triangle_has_collapsed_edge(points: [DVec3; 3]) -> bool {
	let edge_squared = [(points[1] - points[0]).length_squared(), (points[2] - points[1]).length_squared(), (points[0] - points[2]).length_squared()];
	let maximum = edge_squared.into_iter().fold(0.0, f64::max);
	edge_squared.into_iter().fold(f64::INFINITY, f64::min) <= (maximum * 1.0e-24).max(1.0e-30)
}

fn oriented_surface_normal(face: &TrimmedFace, uv: DVec2) -> Option<DVec3> {
	let sample = face.surface.evaluate(uv)?;
	oriented_surface_normal_from_sample(face, uv, sample)
}

fn oriented_surface_normal_from_sample(face: &TrimmedFace, uv: DVec2, sample: SurfaceSample) -> Option<DVec3> {
	let orient = |normal: DVec3| if face.reversed { -normal } else { normal };
	if let Some(normal) = sample.normal() {
		return Some(orient(normal));
	}

	let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
	let u_range = (u_max - u_min).abs();
	let v_range = (v_max - v_min).abs();
	// At a smooth collapsed pole one parametric derivative vanishes exactly on
	// the boundary even though the geometric surface has a unique limit normal.
	// Probe toward the chart interior at successively larger ULP-stable scales;
	// the smallest successful probe is the closest deterministic approximation
	// of that limit and is independent of the incident triangle arrangement.
	for fraction in [2.0_f64.powi(-24), 2.0_f64.powi(-20), 2.0_f64.powi(-16), 2.0_f64.powi(-12)] {
		let u_step = (u_range * fraction).max(f64::EPSILON * u_min.abs().max(u_max.abs()).max(1.0) * 16.0);
		let v_step = (v_range * fraction).max(f64::EPSILON * v_min.abs().max(v_max.abs()).max(1.0) * 16.0);
		let inward_u = inward_parameter(uv.x, u_min, u_max, u_step);
		let inward_v = inward_parameter(uv.y, v_min, v_max, v_step);
		let probes = [DVec2::new(uv.x, inward_v), DVec2::new(inward_u, uv.y), DVec2::new(inward_u, inward_v)];
		for probe in probes {
			if probe != uv {
				if let Some(normal) = face.surface.evaluate(probe).and_then(SurfaceSample::normal) {
					return Some(orient(normal));
				}
			}
		}
	}
	None
}

fn inward_parameter(parameter: f64, minimum: f64, maximum: f64, step: f64) -> f64 {
	let lower_distance = (parameter - minimum).abs();
	let upper_distance = (maximum - parameter).abs();
	if lower_distance <= upper_distance {
		(parameter + step).min(maximum)
	} else {
		(parameter - step).max(minimum)
	}
}

fn stabilize_vertex_normals(face: &TrimmedFace, uvs: &[DVec2], vertices: &[DVec3], indices: &[u32], normals: &mut [DVec3], progress: &ffi::CancellationToken) -> Result<(), Error> {
	check_cancelled(progress)?;
	if uvs.len() != vertices.len() || normals.len() != vertices.len() || !indices.len().is_multiple_of(3) {
		return Err(Error::TriangulationFailed);
	}
	let mut accumulated = vec![DVec3::ZERO; vertices.len()];
	for (triangle_index, triangle) in indices.chunks_exact(3).enumerate() {
		cancellation_checkpoint(progress, triangle_index)?;
		if triangle.iter().any(|index| *index as usize >= vertices.len()) {
			return Err(Error::TriangulationFailed);
		}
		let normal = (vertices[triangle[1] as usize] - vertices[triangle[0] as usize]).cross(vertices[triangle[2] as usize] - vertices[triangle[0] as usize]);
		for index in triangle {
			accumulated[*index as usize] += normal;
		}
	}
	for (vertex_index, ((normal, uv), fallback)) in normals.iter_mut().zip(uvs).zip(accumulated).enumerate() {
		cancellation_checkpoint(progress, vertex_index)?;
		let geometric = fallback.try_normalize();
		if normal.length_squared() <= 1.0e-24 {
			*normal = oriented_surface_normal(face, *uv).or(geometric).ok_or(Error::TriangulationFailed)?;
		}
		let exact = normal.try_normalize().ok_or(Error::TriangulationFailed)?;
		// A rational spline can have a finite but ill-conditioned derivative at an
		// artificial knot or a sub-deflection cusp.  Passing that outlier to the
		// renderer makes every incident triangle interpolate through the bad normal,
		// producing a conspicuous star even though the facets themselves are sound.
		// The local area-weighted fan is the presentation geometry the normal shades;
		// use it only for gross disagreements, retaining exact differential normals
		// across ordinary curvature and keeping the correction within this B-rep face.
		if let Some(geometric) = geometric {
			const MINIMUM_FAN_ALIGNMENT: f64 = 0.866_025_403_784_438_6;
			*normal = if exact.dot(geometric) < MINIMUM_FAN_ALIGNMENT { geometric } else { exact };
		} else {
			*normal = exact;
		}
	}
	Ok(())
}

/// Adds a regular interior lattice to a four-sided isoparametric B-rep face.
///
/// Extrusions, lofts, ruled surfaces, cylinders, cones, and the regular chart
/// of spheres all arrive in this form. Constrained Delaunay still owns the
/// final connectivity and arbitrary trimmed faces retain the general path,
/// but these coherent seeds prevent long diagonals and centroid-refinement
/// pinwheels on the semi-organic surfaces that dominate CAD presentation.
fn seed_structured_patch(face: &TrimmedFace, chart: FaceChart, insertion_domain: &InsertionDomain, triangulation: &mut FaceTriangulation) -> Result<(), Error> {
	let Some(trim_loop) = face.loops.first().filter(|_| face.loops.len() == 1) else {
		return Ok(());
	};
	let runs = boundary_edge_runs(trim_loop);
	if runs.len() != 4 {
		return Ok(());
	}
	let [u_min, u_max, v_min, v_max] = face.surface.uv_bounds;
	let u_range = u_max - u_min;
	let v_range = v_max - v_min;
	let u_tolerance = u_range.abs().max(1.0) * 1.0e-8;
	let v_tolerance = v_range.abs().max(1.0) * 1.0e-8;

	let mut horizontal = Vec::new();
	let mut vertical = Vec::new();
	for run in &runs {
		let (run_u_min, run_u_max, run_v_min, run_v_max) = run.iter().fold((f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY), |(u0, u1, v0, v1), vertex| (u0.min(vertex.uv.x), u1.max(vertex.uv.x), v0.min(vertex.uv.y), v1.max(vertex.uv.y)));
		if run_v_max - run_v_min <= v_tolerance && run_u_max - run_u_min >= u_range.abs() * 0.90 {
			horizontal.push(run);
		} else if run_u_max - run_u_min <= u_tolerance && run_v_max - run_v_min >= v_range.abs() * 0.90 {
			vertical.push(run);
		} else {
			return Ok(());
		}
	}
	if horizontal.len() != 2 || vertical.len() != 2 {
		return Ok(());
	}

	let u_boundary_intervals = horizontal.iter().map(|run| run.len().saturating_sub(1)).max().unwrap_or(1).max(1);
	let v_boundary_intervals = vertical.iter().map(|run| run.len().saturating_sub(1)).max().unwrap_or(1).max(1);
	let metric_u = {
		let Some(first) = chart.map_uv(face, DVec2::new(u_min, (v_min + v_max) * 0.5)) else {
			return Ok(());
		};
		let Some(second) = chart.map_uv(face, DVec2::new(u_max, (v_min + v_max) * 0.5)) else {
			return Ok(());
		};
		((second.x - first.x).powi(2) + (second.y - first.y).powi(2)).sqrt()
	};
	let metric_v = {
		let Some(first) = chart.map_uv(face, DVec2::new((u_min + u_max) * 0.5, v_min)) else {
			return Ok(());
		};
		let Some(second) = chart.map_uv(face, DVec2::new((u_min + u_max) * 0.5, v_max)) else {
			return Ok(());
		};
		((second.x - first.x).powi(2) + (second.y - first.y).powi(2)).sqrt()
	};
	if !metric_u.is_finite() || !metric_v.is_finite() || metric_u <= 1.0e-12 || metric_v <= 1.0e-12 {
		return Ok(());
	}

	let boundary_u_size = metric_u / u_boundary_intervals as f64;
	let boundary_v_size = metric_v / v_boundary_intervals as f64;
	let target_size = boundary_u_size.min(boundary_v_size).max(metric_u.max(metric_v) / 256.0).max(1.0e-9);
	let mut u_intervals = u_boundary_intervals.max((metric_u / target_size).ceil() as usize).clamp(1, 256);
	let mut v_intervals = v_boundary_intervals.max((metric_v / target_size).ceil() as usize).clamp(1, 256);
	while u_intervals.saturating_sub(1) * v_intervals.saturating_sub(1) > MAXIMUM_FACE_VERTICES / 2 {
		if u_intervals >= v_intervals {
			u_intervals = (u_intervals / 2).max(1);
		} else {
			v_intervals = (v_intervals / 2).max(1);
		}
	}
	let mut budget = SeedInsertionBudget::new(MAXIMUM_STRUCTURED_SEED_INSERTIONS, "structured seed");
	for v_index in 1..v_intervals {
		let v = v_min + v_range * v_index as f64 / v_intervals as f64;
		for u_index in 1..u_intervals {
			let u = u_min + u_range * u_index as f64 / u_intervals as f64;
			let uv = DVec2::new(u, v);
			if point_in_trim(uv, &face.loops) {
				budget.insert(face, triangulation, uv, chart, insertion_domain)?;
			}
		}
	}
	Ok(())
}

/// Seeds a face-local inward front that follows the exact trim density.
///
/// Exact edge sampling is intentionally immutable while a face is meshed: two
/// adjacent faces must consume the same canonical boundary vertices. A regular
/// interior lattice alone cannot grade gracefully from a highly non-uniform
/// curved trim, however, and Delaunay refinement then tends to create a chain
/// of progressively thinner cells beside the boundary. This front mirrors
/// each usable boundary sample at one and two local edge lengths into the
/// chart. The samples are unconstrained, so Delaunay still owns connectivity,
/// while their spacing supplies the boundary-size field that a patch-aware
/// mesher needs.
fn seed_boundary_collar(face: &TrimmedFace, chart: FaceChart, insertion_domain: &InsertionDomain, triangulation: &mut FaceTriangulation) -> Result<(), Error> {
	let mut budget = SeedInsertionBudget::new(MAXIMUM_BOUNDARY_COLLAR_INSERTIONS, "boundary collar");
	for trim_loop in &face.loops {
		if trim_loop.vertices.len() < 3 {
			continue;
		}
		let points = trim_loop.vertices.iter().filter_map(|vertex| chart.map_boundary(face, vertex)).collect::<Vec<_>>();
		if points.len() != trim_loop.vertices.len() {
			continue;
		}
		let signed_area = (0..points.len())
			.map(|index| {
				let first = points[index];
				let second = points[(index + 1) % points.len()];
				first.x * second.y - first.y * second.x
			})
			.sum::<f64>();
		if !signed_area.is_finite() || signed_area.abs() <= 1.0e-24 {
			continue;
		}
		for index in 0..points.len() {
			let previous = points[(index + points.len() - 1) % points.len()];
			let current = points[index];
			let next = points[(index + 1) % points.len()];
			let incoming = Point2::new(current.x - previous.x, current.y - previous.y);
			let outgoing = Point2::new(next.x - current.x, next.y - current.y);
			let incoming_length = (incoming.x * incoming.x + incoming.y * incoming.y).sqrt();
			let outgoing_length = (outgoing.x * outgoing.x + outgoing.y * outgoing.y).sqrt();
			let local_length = incoming_length.min(outgoing_length);
			if !local_length.is_finite() || local_length <= 1.0e-12 {
				continue;
			}
			let incoming_normal = Point2::new(-incoming.y / incoming_length, incoming.x / incoming_length);
			let outgoing_normal = Point2::new(-outgoing.y / outgoing_length, outgoing.x / outgoing_length);
			let bisector = Point2::new(incoming_normal.x + outgoing_normal.x, incoming_normal.y + outgoing_normal.y);
			let bisector_length = (bisector.x * bisector.x + bisector.y * bisector.y).sqrt();
			let direction = if bisector_length > 1.0e-12 { Point2::new(bisector.x / bisector_length, bisector.y / bisector_length) } else { outgoing_normal };

			for layer in [0.8, 1.6] {
				let mut distance = local_length * layer;
				let mut inserted = false;
				for _ in 0..6 {
					for sign in [1.0, -1.0] {
						budget.consume()?;
						let candidate = Point2::new(current.x + direction.x * distance * sign, current.y + direction.y * distance * sign);
						if let Some(uv) = chart.unmap(face, candidate) {
							if point_in_trim(uv, &face.loops) {
								budget.insert_consumed(face, triangulation, uv, chart, insertion_domain)?;
								inserted = true;
								break;
							}
						}
					}
					if inserted {
						break;
					}
					distance *= 0.5;
				}
				if !inserted {
					break;
				}
			}
		}
	}
	Ok(())
}

struct InsertionDomain {
	loops: Vec<Vec<Point2<f64>>>,
	minimum: Point2<f64>,
	maximum: Point2<f64>,
	extent: f64,
	representative_boundary_length: f64,
	minimum_site_spacing: f64,
}

impl InsertionDomain {
	fn from_face(face: &TrimmedFace, chart: FaceChart, linear: f64) -> Option<Self> {
		let loops = face.loops.iter().map(|trim_loop| trim_loop.vertices.iter().filter_map(|vertex| chart.map_boundary(face, vertex)).collect::<Vec<_>>()).collect::<Vec<_>>();
		if loops.iter().zip(&face.loops).any(|(mapped, trim_loop)| mapped.len() != trim_loop.vertices.len()) {
			return None;
		}
		let mut boundary_lengths = loops
			.iter()
			.flat_map(|trim_loop| {
				(0..trim_loop.len()).filter_map(|index| {
					let length = metric_distance(trim_loop[index], trim_loop[(index + 1) % trim_loop.len()]);
					(length.is_finite() && length > 1.0e-12).then_some(length)
				})
			})
			.collect::<Vec<_>>();
		if boundary_lengths.is_empty() {
			return None;
		}
		boundary_lengths.sort_by(f64::total_cmp);
		let representative_boundary_length = boundary_lengths[(boundary_lengths.len() * 3 / 5).min(boundary_lengths.len() - 1)];
		let (minimum, maximum) = loops.iter().flatten().fold((Point2::new(f64::INFINITY, f64::INFINITY), Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY)), |(minimum, maximum), point| (Point2::new(minimum.x.min(point.x), minimum.y.min(point.y)), Point2::new(maximum.x.max(point.x), maximum.y.max(point.y))));
		let extent = (maximum.x - minimum.x).abs().max((maximum.y - minimum.y).abs());
		// Once two adaptive sites are many orders of magnitude closer than the
		// requested chord tolerance, separating them cannot measurably improve the
		// approximation. Treat them as one site to prevent an ill-conditioned
		// surface probe from generating an asymptotic cloud of near duplicates.
		let minimum_site_spacing = (extent.max(1.0) * 1.0e-12).max(linear * 3.0);
		(extent.is_finite() && extent > 1.0e-12 && minimum_site_spacing.is_finite()).then_some(Self { loops, minimum, maximum, extent, representative_boundary_length, minimum_site_spacing })
	}

	fn lattice_spacing(&self, linear: f64) -> Option<f64> {
		let spacing = self.representative_boundary_length.max(self.extent / 128.0).max(linear * 1.5).min(self.extent / 8.0);
		(spacing.is_finite() && spacing > 1.0e-12).then_some(spacing)
	}

	fn distance_to_boundary(&self, point: Point2<f64>) -> f64 {
		metric_distance_to_loops(point, &self.loops)
	}

	fn numeric_boundary_clearance(&self) -> f64 {
		self.extent.max(1.0) * 1.0e-10
	}

	fn duplicate_tolerance(&self) -> f64 {
		self.minimum_site_spacing
	}
}

#[derive(Clone, Copy, Debug)]
struct LatticePattern {
	angle: f64,
	x_phase: f64,
	y_phase: f64,
}

#[derive(Clone, Copy, Debug)]
struct PlanarMeshQuality {
	worst_aspect: f64,
	p95_aspect: f64,
	minimum_angle_degrees: f64,
	p05_minimum_angle_degrees: f64,
	mean_aspect: f64,
	vertex_count: usize,
}

impl PlanarMeshQuality {
	fn from_mesh(mesh: &MeshedFace) -> Option<Self> {
		let triangles = mesh.indices.chunks_exact(3).map(|triangle| [mesh.vertices[triangle[0] as usize], mesh.vertices[triangle[1] as usize], mesh.vertices[triangle[2] as usize]]).collect::<Vec<_>>();
		let mut aspects = triangles.iter().copied().map(triangle_aspect).collect::<Vec<_>>();
		let mut minimum_angles = triangles.iter().copied().map(triangle_minimum_angle_degrees).collect::<Option<Vec<_>>>()?;
		if aspects.is_empty() || aspects.iter().any(|aspect| !aspect.is_finite()) {
			return None;
		}
		let worst_aspect = aspects.iter().copied().fold(0.0, f64::max);
		let mean_aspect = aspects.iter().sum::<f64>() / aspects.len() as f64;
		let p95_aspect = percentile(&mut aspects, 0.95)?;
		let minimum_angle_degrees = minimum_angles.iter().copied().fold(f64::INFINITY, f64::min);
		let p05_minimum_angle_degrees = percentile(&mut minimum_angles, 0.05)?;
		Some(Self { worst_aspect, p95_aspect, minimum_angle_degrees, p05_minimum_angle_degrees, mean_aspect, vertex_count: mesh.vertices.len() })
	}

	fn improves(self, current: Self) -> bool {
		self.target_penalty().total_cmp(&current.target_penalty()).then_with(|| self.p95_aspect.total_cmp(&current.p95_aspect)).then_with(|| current.p05_minimum_angle_degrees.total_cmp(&self.p05_minimum_angle_degrees)).then_with(|| self.worst_aspect.total_cmp(&current.worst_aspect)).then_with(|| self.mean_aspect.total_cmp(&current.mean_aspect)).then_with(|| self.vertex_count.cmp(&current.vertex_count)).is_lt()
	}

	fn meets_quality_target(self) -> bool {
		self.worst_aspect <= 30.0 && self.p95_aspect <= 4.0 && self.minimum_angle_degrees >= 0.75 && self.p05_minimum_angle_degrees >= 15.0
	}

	fn target_penalty(self) -> f64 {
		(self.worst_aspect / 30.0).max(self.p95_aspect / 4.0).max(0.75 / self.minimum_angle_degrees.max(f64::MIN_POSITIVE)).max(15.0 / self.p05_minimum_angle_degrees.max(f64::MIN_POSITIVE))
	}
}

fn triangle_minimum_angle_degrees(points: [DVec3; 3]) -> Option<f64> {
	let mut minimum = f64::INFINITY;
	for corner in 0..3 {
		let first = (points[(corner + 1) % 3] - points[corner]).try_normalize()?;
		let second = (points[(corner + 2) % 3] - points[corner]).try_normalize()?;
		minimum = minimum.min(first.dot(second).clamp(-1.0, 1.0).acos().to_degrees());
	}
	Some(minimum)
}

/// Selects a deterministic planar lattice by measuring the constrained mesh,
/// not by assuming one world-axis phase is suitable for every trim.
///
/// A triangular lattice is isotropic in its interior, but its first row can be
/// almost tangent to a curved canonical trim sample for one particular phase.
/// That creates a visible sliver even though both the lattice and the CDT are
/// individually well behaved. Planar faces are inexpensive to evaluate, so a
/// small, fixed pattern set is built independently and ranked by physical
/// worst-case, p95, and mean aspect. The exact constrained boundary is cloned
/// unchanged into every trial; only unconstrained interior points differ.
fn seed_best_planar_lattice(face: &TrimmedFace, chart: FaceChart, linear: f64, insertion_domain: &InsertionDomain, base: FaceTriangulation, progress: &ffi::CancellationToken) -> Result<FaceTriangulation, Error> {
	const PATTERNS: [LatticePattern; 8] = [
		LatticePattern { angle: 0.0, x_phase: 0.0, y_phase: 0.0 },
		LatticePattern { angle: 0.0, x_phase: 0.5, y_phase: 0.5 },
		LatticePattern { angle: std::f64::consts::PI / 12.0, x_phase: 0.0, y_phase: 0.5 },
		LatticePattern { angle: std::f64::consts::PI / 12.0, x_phase: 0.5, y_phase: 0.0 },
		LatticePattern { angle: std::f64::consts::PI / 6.0, x_phase: 0.25, y_phase: 0.25 },
		LatticePattern { angle: std::f64::consts::PI / 6.0, x_phase: 0.75, y_phase: 0.75 },
		LatticePattern { angle: std::f64::consts::PI / 4.0, x_phase: 0.25, y_phase: 0.75 },
		LatticePattern { angle: std::f64::consts::PI / 4.0, x_phase: 0.75, y_phase: 0.25 },
	];

	let mut best = None::<(PlanarMeshQuality, FaceTriangulation)>;
	for (pattern_index, pattern) in PATTERNS.into_iter().enumerate() {
		cancellation_checkpoint(progress, pattern_index)?;
		let mut trial = base.clone();
		seed_metric_lattice_pattern(face, chart, linear, insertion_domain, pattern, &mut trial)?;
		let Ok(mesh) = build_face_mesh(face, &trial, progress) else {
			continue;
		};
		let Some(quality) = PlanarMeshQuality::from_mesh(&mesh) else {
			continue;
		};
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation face {} planar lattice pattern {pattern_index} {pattern:?}: {quality:?}", face.index);
		}
		if quality.meets_quality_target() {
			return Ok(trial);
		}
		if best.as_ref().is_none_or(|(current, _)| quality.improves(*current)) {
			best = Some((quality, trial));
		}
	}
	let Some((_, triangulation)) = best else {
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation face {} found no valid seeded planar lattice; retaining its constrained boundary triangulation", face.index);
		}
		return Ok(base);
	};
	Ok(triangulation)
}

/// Seeds arbitrary trimmed charts with a coherent triangular lattice in the
/// local first-fundamental-form metric.
///
/// Boundary-only CDT is a poor starting point for smooth caps: it spans a
/// dense curved trim with long diagonals, then local error refinement has to
/// recover a size field from scratch. A staggered lattice gives Delaunay an
/// isotropic interior size field up front. Points stay away from the immutable
/// exact boundary by a fraction of one cell, leaving a single graded collar
/// for the constrained triangulation to fill.
fn seed_metric_lattice(face: &TrimmedFace, chart: FaceChart, linear: f64, insertion_domain: &InsertionDomain, triangulation: &mut FaceTriangulation) -> Result<(), Error> {
	seed_metric_lattice_pattern(face, chart, linear, insertion_domain, LatticePattern { angle: 0.0, x_phase: 0.0, y_phase: 0.0 }, triangulation)
}

fn seed_metric_lattice_pattern(face: &TrimmedFace, chart: FaceChart, linear: f64, insertion_domain: &InsertionDomain, pattern: LatticePattern, triangulation: &mut FaceTriangulation) -> Result<(), Error> {
	let Some(spacing) = insertion_domain.lattice_spacing(linear) else {
		return Ok(());
	};
	let center = Point2::new((insertion_domain.minimum.x + insertion_domain.maximum.x) * 0.5, (insertion_domain.minimum.y + insertion_domain.maximum.y) * 0.5);
	let cosine = pattern.angle.cos();
	let sine = pattern.angle.sin();
	let to_lattice = |point: Point2<f64>| {
		let x = point.x - center.x;
		let y = point.y - center.y;
		Point2::new(x * cosine + y * sine, -x * sine + y * cosine)
	};
	let from_lattice = |point: Point2<f64>| Point2::new(center.x + point.x * cosine - point.y * sine, center.y + point.x * sine + point.y * cosine);
	let (minimum, maximum) = insertion_domain.loops.iter().flatten().map(|point| to_lattice(*point)).fold((Point2::new(f64::INFINITY, f64::INFINITY), Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY)), |(minimum, maximum), point| (Point2::new(minimum.x.min(point.x), minimum.y.min(point.y)), Point2::new(maximum.x.max(point.x), maximum.y.max(point.y))));
	let width = maximum.x - minimum.x;
	let height = maximum.y - minimum.y;
	let row_step = spacing * 3.0_f64.sqrt() * 0.5;
	let columns = (width / spacing).ceil() as usize + 3;
	let rows = (height / row_step).ceil() as usize + 3;
	if columns.saturating_mul(rows) > MAXIMUM_LATTICE_INSERTIONS {
		return Err(resource_limit("tessellation metric lattice exceeded its seed insertion limit"));
	}
	let mut budget = SeedInsertionBudget::new(MAXIMUM_LATTICE_INSERTIONS, "metric lattice");
	let boundary_clearance = spacing * 0.28;
	for row in 0..rows {
		let y = minimum.y + (row as f64 - 1.0 + pattern.y_phase) * row_step;
		let offset = if row.is_multiple_of(2) { 0.0 } else { spacing * 0.5 };
		for column in 0..columns {
			let point = from_lattice(Point2::new(minimum.x + (column as f64 - 1.0 + pattern.x_phase) * spacing + offset, y));
			let Some(uv) = chart.unmap(face, point) else {
				continue;
			};
			if !point_in_trim(uv, &face.loops) || insertion_domain.distance_to_boundary(point) < boundary_clearance {
				continue;
			}
			budget.insert(face, triangulation, uv, chart, insertion_domain)?;
		}
	}
	Ok(())
}

fn metric_distance_to_loops(point: Point2<f64>, loops: &[Vec<Point2<f64>>]) -> f64 {
	loops.iter().filter(|trim_loop| trim_loop.len() >= 2).flat_map(|trim_loop| (0..trim_loop.len()).map(move |index| metric_point_segment_distance(point, trim_loop[index], trim_loop[(index + 1) % trim_loop.len()]))).fold(f64::INFINITY, f64::min)
}

fn metric_point_segment_distance(point: Point2<f64>, first: Point2<f64>, second: Point2<f64>) -> f64 {
	let segment = Point2::new(second.x - first.x, second.y - first.y);
	let offset = Point2::new(point.x - first.x, point.y - first.y);
	let length_squared = segment.x * segment.x + segment.y * segment.y;
	if length_squared <= 1.0e-30 {
		return metric_distance(point, first);
	}
	let fraction = ((offset.x * segment.x + offset.y * segment.y) / length_squared).clamp(0.0, 1.0);
	let closest = Point2::new(first.x + segment.x * fraction, first.y + segment.y * fraction);
	metric_distance(point, closest)
}

struct SeedInsertionBudget {
	remaining: usize,
	label: &'static str,
}

impl SeedInsertionBudget {
	fn new(limit: usize, label: &'static str) -> Self {
		Self { remaining: limit, label }
	}

	fn insert(&mut self, face: &TrimmedFace, triangulation: &mut FaceTriangulation, uv: DVec2, chart: FaceChart, insertion_domain: &InsertionDomain) -> Result<(), Error> {
		self.consume()?;
		self.insert_consumed(face, triangulation, uv, chart, insertion_domain)
	}

	fn consume(&mut self) -> Result<(), Error> {
		if self.remaining == 0 {
			return Err(resource_limit(format!("tessellation {} exceeded its insertion limit", self.label)));
		}
		self.remaining -= 1;
		Ok(())
	}

	fn insert_consumed(&self, face: &TrimmedFace, triangulation: &mut FaceTriangulation, uv: DVec2, chart: FaceChart, insertion_domain: &InsertionDomain) -> Result<(), Error> {
		if triangulation.num_vertices() >= MAXIMUM_CDT_FACE_VERTICES {
			return Err(resource_limit("tessellation face exceeded the CDT vertex limit"));
		}
		insert_interior_vertex(face, triangulation, uv, chart, insertion_domain);
		Ok(())
	}
}

fn insert_interior_vertex(face: &TrimmedFace, triangulation: &mut FaceTriangulation, uv: DVec2, chart: FaceChart, insertion_domain: &InsertionDomain) {
	let Some(metric_position) = chart.map_uv(face, uv) else {
		return;
	};
	// Refinement probes can round onto, or a few ULPs inside, an immutable trim
	// constraint. Spade may classify such a point as lying in the face rather
	// than on the constraint. Inserting it creates a fan of nearly coincident
	// boundary triangles and can consume the entire refinement budget without
	// improving the surface. Canonical boundary samples already satisfy the
	// curve tolerance, so keep a scale-relative numerical exclusion zone around
	// every trim segment.
	let boundary_clearance = insertion_domain.numeric_boundary_clearance();
	let distance_to_boundary = insertion_domain.distance_to_boundary(metric_position);
	if distance_to_boundary <= boundary_clearance {
		return;
	}
	// Independent structured, lattice, and adaptive probes can describe the
	// same interior point with adjacent floating-point values.  Spade correctly
	// treats those as distinct coordinates, but their two zero-width cells are
	// later removed and leave a real incidence-one cavity.  Reject only a
	// scale-relative numerical duplicate of an existing vertex; this is orders
	// of magnitude below any supported geometric refinement spacing.
	let location = triangulation.locate(metric_position);
	let duplicate_tolerance = insertion_domain.duplicate_tolerance();
	let near_duplicate = match location {
		PositionInTriangulation::OnVertex(_) => true,
		PositionInTriangulation::OnEdge(edge) | PositionInTriangulation::OutsideOfConvexHull(edge) => triangulation.directed_edge(edge).vertices().into_iter().any(|vertex| metric_distance(vertex.position(), metric_position) <= duplicate_tolerance),
		PositionInTriangulation::OnFace(face_handle) => triangulation.face(face_handle).vertices().into_iter().any(|vertex| metric_distance(vertex.position(), metric_position) <= duplicate_tolerance),
		PositionInTriangulation::NoTriangulation => false,
	};
	if near_duplicate {
		return;
	}
	match location {
		PositionInTriangulation::OnVertex(_) => {}
		PositionInTriangulation::OnEdge(edge) if triangulation.directed_edge(edge).is_constraint_edge() => {}
		_ => {
			let _ = triangulation.insert(ParametricVertex { uv, metric: metric_position, boundary_position: None, boundary_occurrences: [None, None] });
		}
	}
}

fn boundary_edge_runs(trim_loop: &TrimLoop) -> Vec<Vec<&BoundaryVertex>> {
	let vertices = &trim_loop.vertices;
	if vertices.is_empty() {
		return Vec::new();
	}
	let mut starts = vec![0];
	for index in 1..vertices.len() {
		if vertices[index].edge_index != vertices[index - 1].edge_index {
			starts.push(index);
		}
	}
	starts
		.iter()
		.enumerate()
		.map(|(ordinal, start)| {
			let end = starts.get(ordinal + 1).copied().unwrap_or(vertices.len());
			let mut run = vertices[*start..end].iter().collect::<Vec<_>>();
			run.push(&vertices[end % vertices.len()]);
			run
		})
		.collect()
}

#[derive(Clone, Copy, Debug)]
struct RefinementCandidate {
	required: bool,
	linear_required: bool,
	score: f64,
	uv: DVec2,
}

fn refinement_candidates(face: &TrimmedFace, triangulation: &FaceTriangulation, linear: f64, angular: f64, progress: &ffi::CancellationToken) -> Result<Vec<RefinementCandidate>, Error> {
	const RETAINED_CANDIDATE_MULTIPLIER: usize = 4;
	let required_capacity = MAXIMUM_REQUIRED_INSERTIONS_PER_PASS * RETAINED_CANDIDATE_MULTIPLIER;
	let quality_capacity = MAXIMUM_QUALITY_INSERTIONS_PER_PASS * RETAINED_CANDIDATE_MULTIPLIER;
	let mut required_candidates = Vec::with_capacity(required_capacity);
	let mut quality_candidates = Vec::with_capacity(quality_capacity);

	for (triangle_index, triangle) in triangulation.inner_faces().enumerate() {
		if triangle_index.is_multiple_of(256) && progress.is_cancelled() {
			return Err(Error::Cancelled);
		}
		let vertices = triangle.vertices();
		let handles = vertices.map(|vertex| vertex.fix());
		let parametric = vertices.map(|vertex| *vertex.data());
		let uv = parametric.map(|vertex| vertex.uv);
		let center = (uv[0] + uv[1] + uv[2]) / 3.0;
		if !point_in_trim(center, &face.loops) {
			continue;
		}
		let [Some(first), Some(second), Some(third)] = uv.map(|point| face.surface.evaluate_position(point)) else {
			continue;
		};
		let Some(center_sample) = face.surface.evaluate(center) else {
			continue;
		};
		let positions = std::array::from_fn(|index| parametric[index].boundary_position.unwrap_or([first, second, third][index]));
		let Some(mut geometric_normal) = (positions[1] - positions[0]).cross(positions[2] - positions[0]).try_normalize() else {
			continue;
		};
		// A C0/repeated-knot joint may not have a unique differential at the
		// triangle center. Linear chord refinement is still well-defined there and
		// must not be skipped merely because the optional angular probe is singular.
		if let Some(expected_normal) = center_sample.normal() {
			if geometric_normal.dot(expected_normal) < 0.0 {
				geometric_normal = -geometric_normal;
			}
		}
		let angular_singular = face.surface.triangle_crosses_nonsmooth_knot(uv) || subdeflection_cusp(face, uv, positions, linear);

		// Retain the worst mandatory probe per triangle. Refining every center
		// and midpoint from a single cell in one pass creates redundant sites;
		// selecting its maximum error provides the same convergence path while
		// keeping the work queue strictly proportional to the triangle count.
		let mut required = Vec::with_capacity(4);
		let center_deviation = point_triangle_distance(center_sample.position, positions);
		push_surface_probe(&mut required, center, center_sample, center_deviation, geometric_normal, linear, (!angular_singular).then_some(angular));
		for edge in 0..3 {
			let next = (edge + 1) % 3;
			if triangulation.get_edge_from_neighbors(handles[edge], handles[next]).is_some_and(|edge| edge.is_constraint_edge()) {
				continue;
			}
			let midpoint_uv = (uv[edge] + uv[next]) * 0.5;
			if let Some(midpoint) = face.surface.evaluate(midpoint_uv) {
				let deviation = point_segment_distance(midpoint.position, positions[edge], positions[next]);
				push_surface_probe(&mut required, midpoint_uv, midpoint, deviation, geometric_normal, linear, (!angular_singular).then_some(angular));
			}
		}
		if let Some(candidate) = required.into_iter().min_by(refinement_candidate_order) {
			push_bounded_candidate(&mut required_candidates, candidate, required_capacity);
			continue;
		}

		let aspect = triangle_aspect(positions);
		if aspect <= MAXIMUM_PHYSICAL_ASPECT * (1.0 + 1.0e-9) {
			continue;
		}
		let circumcenter = triangle.circumcenter();
		let barycentric = triangle.barycentric_interpolation(circumcenter);
		let candidate_uv = uv[0] * barycentric[0] + uv[1] * barycentric[1] + uv[2] * barycentric[2];
		let quality = if barycentric.iter().all(|weight| *weight >= 0.05) && candidate_uv.is_finite() && point_in_trim(candidate_uv, &face.loops) {
			Some(RefinementCandidate { required: false, linear_required: false, score: aspect / TARGET_PHYSICAL_ASPECT, uv: candidate_uv })
		} else {
			// An obtuse sliver has an exterior circumcenter. Split its longest
			// unconstrained physical edge so Delaunay legalization can improve
			// both incident cells without touching an exact trim segment.
			(0..3)
				.filter_map(|edge| {
					let next = (edge + 1) % 3;
					let constrained = triangulation.get_edge_from_neighbors(handles[edge], handles[next]).is_some_and(|edge| edge.is_constraint_edge());
					(!constrained).then_some((positions[edge].distance_squared(positions[next]), edge, next))
				})
				.max_by(|first, second| first.0.total_cmp(&second.0))
				.and_then(|(_, edge, next)| {
					let midpoint = (uv[edge] + uv[next]) * 0.5;
					(midpoint.is_finite() && point_in_trim(midpoint, &face.loops)).then_some(RefinementCandidate { required: false, linear_required: false, score: aspect / TARGET_PHYSICAL_ASPECT, uv: midpoint })
				})
		};
		if let Some(candidate) = quality {
			push_bounded_candidate(&mut quality_candidates, candidate, quality_capacity);
		}
	}

	let mut candidates = if required_candidates.is_empty() { quality_candidates } else { required_candidates };
	candidates.sort_by(refinement_candidate_order);
	let capacity = if candidates.first().is_some_and(|candidate| candidate.required) { required_capacity } else { quality_capacity };
	candidates.truncate(capacity);
	Ok(candidates)
}

fn refinement_candidate_order(first: &RefinementCandidate, second: &RefinementCandidate) -> std::cmp::Ordering {
	second.linear_required.cmp(&first.linear_required).then_with(|| second.score.total_cmp(&first.score)).then_with(|| first.uv.x.total_cmp(&second.uv.x)).then_with(|| first.uv.y.total_cmp(&second.uv.y))
}

fn push_bounded_candidate(candidates: &mut Vec<RefinementCandidate>, candidate: RefinementCandidate, capacity: usize) {
	candidates.push(candidate);
	if candidates.len() > capacity * 2 {
		candidates.sort_by(refinement_candidate_order);
		candidates.truncate(capacity);
	}
}

fn push_surface_probe(candidates: &mut Vec<RefinementCandidate>, uv: DVec2, sample: SurfaceSample, deviation: f64, geometric_normal: DVec3, linear: f64, angular: Option<f64>) {
	// A degree-p knot with multiplicity p is C0: both one-sided normals are
	// valid, but no finite triangle can approximate their discontinuous turn as
	// one smooth angular field. Linear deflection remains mandatory across that
	// joint; suppress only the impossible angular objective.
	let angular_score = angular.zip(sample.normal()).map(|(angular, normal)| geometric_normal.dot(normal).clamp(-1.0, 1.0).acos() / angular).unwrap_or(0.0);
	let linear_score = deviation / linear;
	let candidate = RefinementCandidate { required: true, linear_required: linear_score > 1.0, score: linear_score.max(angular_score), uv };
	if candidate.score > 1.0 && candidate.score.is_finite() {
		candidates.push(candidate);
	}
}

fn point_segment_distance(point: DVec3, first: DVec3, second: DVec3) -> f64 {
	let segment = second - first;
	let length_squared = segment.length_squared();
	if length_squared <= 1.0e-30 {
		return point.distance(first);
	}
	let fraction = (point - first).dot(segment) / length_squared;
	point.distance(first + segment * fraction.clamp(0.0, 1.0))
}

fn point_triangle_distance(point: DVec3, triangle: [DVec3; 3]) -> f64 {
	let [first, second, third] = triangle;
	let first_second = second - first;
	let first_third = third - first;
	let first_point = point - first;
	let d1 = first_second.dot(first_point);
	let d2 = first_third.dot(first_point);
	if d1 <= 0.0 && d2 <= 0.0 {
		return point.distance(first);
	}

	let second_point = point - second;
	let d3 = first_second.dot(second_point);
	let d4 = first_third.dot(second_point);
	if d3 >= 0.0 && d4 <= d3 {
		return point.distance(second);
	}

	let first_edge_region = d1 * d4 - d3 * d2;
	if first_edge_region <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
		let fraction = d1 / (d1 - d3);
		return point.distance(first + first_second * fraction);
	}

	let third_point = point - third;
	let d5 = first_second.dot(third_point);
	let d6 = first_third.dot(third_point);
	if d6 >= 0.0 && d5 <= d6 {
		return point.distance(third);
	}

	let second_edge_region = d5 * d2 - d1 * d6;
	if second_edge_region <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
		let fraction = d2 / (d2 - d6);
		return point.distance(first + first_third * fraction);
	}

	let third_edge_region = d3 * d6 - d5 * d4;
	if third_edge_region <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
		let fraction = (d4 - d3) / ((d4 - d3) + (d5 - d6));
		return point.distance(second + (third - second) * fraction);
	}

	let denominator = first_edge_region + second_edge_region + third_edge_region;
	if denominator.abs() <= 1.0e-30 {
		return point_segment_distance(point, first, second).min(point_segment_distance(point, second, third)).min(point_segment_distance(point, third, first));
	}
	let inverse = 1.0 / denominator;
	let second_weight = second_edge_region * inverse;
	let third_weight = first_edge_region * inverse;
	let projection = first + first_second * second_weight + first_third * third_weight;
	point.distance(projection)
}

fn triangle_aspect(points: [DVec3; 3]) -> f64 {
	let edges = [points[1] - points[0], points[2] - points[1], points[0] - points[2]];
	let longest = edges.into_iter().map(DVec3::length).fold(0.0, f64::max);
	let twice_area = (points[1] - points[0]).cross(points[2] - points[0]).length();
	if longest <= 1.0e-15 || twice_area <= 1.0e-30 {
		f64::INFINITY
	} else {
		longest * longest / twice_area
	}
}

fn build_face_mesh(face: &TrimmedFace, triangulation: &FaceTriangulation, progress: &ffi::CancellationToken) -> Result<MeshedFace, Error> {
	check_cancelled(progress)?;
	let mut triangles = Vec::new();
	let mut used = BTreeSet::new();
	for (triangle_index, triangle) in triangulation.inner_faces().enumerate() {
		cancellation_checkpoint(progress, triangle_index)?;
		let handles = triangle.vertices().map(|vertex| vertex.fix().index());
		let parametric = handles.map(|index| *triangulation.vertex(spade::handles::FixedVertexHandle::from_index(index)).data());
		let uv = parametric.map(|vertex| vertex.uv);
		let center = (uv[0] + uv[1] + uv[2]) / 3.0;
		if !point_in_trim(center, &face.loops) {
			continue;
		}
		let metric = parametric.map(|vertex| vertex.metric);
		let metric_area = ((metric[1].x - metric[0].x) * (metric[2].y - metric[0].y) - (metric[1].y - metric[0].y) * (metric[2].x - metric[0].x)).abs();
		let metric_edge_scale_squared = [(metric[1], metric[0]), (metric[2], metric[1]), (metric[0], metric[2])].into_iter().map(|(first, second)| (second.x - first.x).powi(2) + (second.y - first.y).powi(2)).fold(0.0, f64::max);
		if triangle_shares_boundary_occurrence(parametric) && metric_area <= metric_edge_scale_squared * 1.0e-12 {
			// A CDT can expose a face between three samples of one curved trim
			// occurrence. The samples are collinear in the face chart but not in
			// 3D, so a geometric area test mistakes the boundary's osculating
			// plane for a surface cell. Typed occurrence provenance distinguishes
			// this phantom from a legitimate curved cap ear and from the opposite
			// occurrence of a periodic self-seam.
			continue;
		}
		let mut positions = [DVec3::ZERO; 3];
		for corner in 0..3 {
			let handle = triangulation.vertex(spade::handles::FixedVertexHandle::from_index(handles[corner]));
			let vertex = *handle.data();
			positions[corner] = vertex.boundary_position.unwrap_or(face.surface.evaluate_position(vertex.uv).ok_or(Error::TriangulationFailed)?);
		}
		let geometric = (positions[1] - positions[0]).cross(positions[2] - positions[0]);
		if !geometric.is_finite() {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation face {} produced non-finite CDT triangle at uvs {:?}, points {:?}", face.index, uv, positions);
			}
			return Err(Error::TriangulationFailed);
		}
		if geometric.length_squared() <= triangle_area_threshold(positions) {
			// A constrained triangulation may retain a zero-area face between
			// three distinct samples of a locally straight canonical boundary.
			// It contributes no surface area. Discard it and let the mandatory
			// exact boundary-segment and edge-incidence audit below prove that no
			// actual trim segment or interior region was lost.
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation face {} discarded zero-area CDT triangle {handles:?}, uvs {uv:?}, points {positions:?}", face.index);
			}
			continue;
		}
		let uv_area = (uv[1] - uv[0]).perp_dot(uv[2] - uv[0]);
		let mut oriented = handles;
		// Spade emits counter-clockwise triangles in the metric chart and the
		// chart transform has a positive determinant. Surface orientation is
		// therefore a single face-wide topological decision; never flip cells
		// independently near a pole or other ill-conditioned derivative.
		if (uv_area < 0.0) != face.reversed {
			oriented.swap(1, 2);
		}
		used.extend(oriented);
		triangles.push(oriented);
	}
	if triangles.is_empty() {
		return Err(Error::TriangulationFailed);
	}
	// Spade does not promise a stable inner-face iteration order when tiny
	// floating-point perturbations leave the same constrained triangulation in
	// place. Normalize each oriented triangle by a cyclic rotation, then sort
	// the cells by their stable insertion handles. This changes neither winding
	// nor adjacency, but makes mesh bytes deterministic across rigid placement,
	// scale, and serial/parallel scheduling.
	for triangle in &mut triangles {
		let minimum_corner = triangle.iter().enumerate().min_by_key(|(_, index)| **index).map(|(corner, _)| corner).unwrap_or(0);
		triangle.rotate_left(minimum_corner);
	}
	triangles.sort_unstable();

	let mut remap = vec![u32::MAX; triangulation.num_vertices()];
	let mut vertices = Vec::with_capacity(used.len());
	let mut uvs = Vec::with_capacity(used.len());
	let mut normals = Vec::with_capacity(used.len());
	for (vertex_index, old_index) in used.into_iter().enumerate() {
		cancellation_checkpoint(progress, vertex_index)?;
		let handle = triangulation.vertex(spade::handles::FixedVertexHandle::from_index(old_index));
		let vertex = *handle.data();
		let sample = face.surface.evaluate(vertex.uv).ok_or(Error::TriangulationFailed)?;
		let position = vertex.boundary_position.unwrap_or(sample.position);
		let normal = oriented_surface_normal_from_sample(face, vertex.uv, sample).unwrap_or(DVec3::ZERO);
		remap[old_index] = u32::try_from(vertices.len()).map_err(|_| Error::TriangulationFailed)?;
		vertices.push(position);
		uvs.push(vertex.uv);
		normals.push(normal);
	}
	let mut indices = Vec::with_capacity(triangles.len() * 3);
	for (triangle_index, triangle) in triangles.into_iter().enumerate() {
		cancellation_checkpoint(progress, triangle_index)?;
		indices.extend(triangle.map(|index| remap[index]));
	}
	stabilize_vertex_normals(face, &uvs, &vertices, &indices, &mut normals, progress)?;
	Ok(MeshedFace {
		index: face.index,
		tshape_id: face.tshape_id,
		vertices,
		uvs,
		normals,
		indices,
		refined_edges: BTreeMap::new(),
		boundary_refinements: Vec::new(),
		quality_exempt_vertices: BTreeSet::new(),
	})
}

fn triangle_shares_boundary_occurrence(vertices: [ParametricVertex; 3]) -> bool {
	vertices[0].boundary_occurrences.into_iter().flatten().any(|occurrence| vertices[1].boundary_occurrences.contains(&Some(occurrence)) && vertices[2].boundary_occurrences.contains(&Some(occurrence)))
}

fn point_in_trim(point: DVec2, loops: &[TrimLoop]) -> bool {
	loops.iter().fold(false, |inside, trim_loop| inside ^ point_in_polygon(point, &trim_loop.vertices))
}

fn point_in_polygon(point: DVec2, vertices: &[BoundaryVertex]) -> bool {
	let mut inside = false;
	for index in 0..vertices.len() {
		let first = vertices[index].uv;
		let second = vertices[(index + 1) % vertices.len()].uv;
		if (first.y > point.y) != (second.y > point.y) {
			let crossing = first.x + (point.y - first.y) * (second.x - first.x) / (second.y - first.y);
			if point.x < crossing {
				inside = !inside;
			}
		}
	}
	inside
}

fn assemble_mesh_data(faces: Vec<MeshedFace>, edges: &[Vec<DVec3>], include_edges: bool, progress: &ffi::CancellationToken) -> Result<ffi::MeshData, Error> {
	check_cancelled(progress)?;
	let mut seen_face_indices = BTreeSet::new();
	let mut edge_refinements = BTreeMap::<u32, &[DVec3]>::new();
	let mut vertex_count = 0usize;
	let mut index_count = 0usize;
	let mut triangle_count = 0usize;
	for (face_index, face) in faces.iter().enumerate() {
		cancellation_checkpoint(progress, face_index)?;
		if face.vertices.is_empty() || face.vertices.len() != face.uvs.len() || face.vertices.len() != face.normals.len() || face.vertices.len() > MAXIMUM_FACE_VERTICES || !face.indices.len().is_multiple_of(3) || face.indices.iter().any(|index| *index as usize >= face.vertices.len()) || face.vertices.iter().any(|vertex| !vertex.is_finite()) || face.normals.iter().any(|normal| !normal.is_finite()) || !seen_face_indices.insert(face.index) {
			return Err(Error::TriangulationFailed);
		}
		for (edge_index, points) in &face.refined_edges {
			match edge_refinements.get(edge_index) {
				Some(existing) if !point_sequences_match(existing, points) => return Err(Error::TriangulationFailed),
				Some(_) => {}
				None => {
					edge_refinements.insert(*edge_index, points);
				}
			}
		}
		vertex_count = checked_add_resource(vertex_count, face.vertices.len(), "tessellation request vertex count overflowed")?;
		index_count = checked_add_resource(index_count, face.indices.len(), "tessellation request index count overflowed")?;
		triangle_count = checked_add_resource(triangle_count, face.indices.len() / 3, "tessellation request triangle count overflowed")?;
	}
	if vertex_count > MAXIMUM_REQUEST_VERTICES || triangle_count > MAXIMUM_REQUEST_TRIANGLES || index_count > MAXIMUM_REQUEST_INDICES || vertex_count > u32::MAX as usize || index_count > u32::MAX as usize {
		return Err(resource_limit("tessellation request exceeded aggregate mesh resource limits"));
	}

	let mut effective_edges = Vec::<(&[DVec3], bool, u32)>::new();
	let mut edge_point_count = 0usize;
	if include_edges {
		reserve_exact(&mut effective_edges, edges.len(), "tessellation edge plan allocation failed")?;
		for (index, edge) in edges.iter().enumerate() {
			cancellation_checkpoint(progress, index)?;
			let edge_index = u32::try_from(index).map_err(|_| resource_limit("tessellation edge count exceeded index capacity"))?;
			let (points, reversed) = if let Some(refinement) = edge_refinements.get(&edge_index).copied() {
				if edge.len() < 2 || refinement.len() < 2 || edge.iter().any(|point| !point.is_finite()) || refinement.iter().any(|point| !point.is_finite()) {
					return Err(Error::TriangulationFailed);
				}
				let source_first = point_key(edge[0]);
				let source_last = point_key(*edge.last().ok_or(Error::TriangulationFailed)?);
				let reversed = if point_key(refinement[0]) == source_last && point_key(*refinement.last().ok_or(Error::TriangulationFailed)?) == source_first {
					true
				} else if point_key(refinement[0]) == source_first && point_key(*refinement.last().ok_or(Error::TriangulationFailed)?) == source_last {
					false
				} else {
					return Err(Error::TriangulationFailed);
				};
				if edge.iter().any(|source| !refinement.iter().any(|point| point_key(*point) == point_key(*source))) {
					return Err(Error::TriangulationFailed);
				}
				(refinement, reversed)
			} else {
				if edge.len() < 2 || edge.iter().any(|point| !point.is_finite()) {
					return Err(Error::TriangulationFailed);
				}
				(edge.as_slice(), false)
			};
			if points.iter().all(|point| point_key(*point) == point_key(points[0])) {
				continue;
			}
			edge_point_count = checked_add_resource(edge_point_count, points.len(), "tessellation edge-point count overflowed")?;
			effective_edges.push((points, reversed, edge_index));
		}
	}

	let face_count = faces.len();
	let edge_count = effective_edges.len();
	let face_offset_count = checked_add_resource(face_count, 1, "tessellation face-offset count overflowed")?;
	let edge_offset_count = checked_add_resource(edge_count, 1, "tessellation edge-offset count overflowed")?;
	let vertex_values = checked_mul_resource(vertex_count, 3, "tessellation vertex payload size overflowed")?;
	let normal_values = checked_mul_resource(vertex_count, 3, "tessellation normal payload size overflowed")?;
	let edge_point_values = checked_mul_resource(edge_point_count, 3, "tessellation edge payload size overflowed")?;
	validate_request_payload_bytes(vertex_count, triangle_count, face_count, edge_point_count, edge_count)?;
	if edge_point_count > u32::MAX as usize {
		return Err(resource_limit("tessellation request exceeded the output payload limit"));
	}

	let mut result = ffi::MeshData {
		vertices: Vec::new(),
		normals: Vec::new(),
		indices: Vec::new(),
		face_tshape_ids: Vec::new(),
		chunk_face_tshape_ids: Vec::new(),
		chunk_face_indices: Vec::new(),
		face_vertex_offsets: Vec::new(),
		face_index_offsets: Vec::new(),
		edge_points: Vec::new(),
		chunk_edge_indices: Vec::new(),
		edge_point_offsets: Vec::new(),
		success: false,
	};
	reserve_exact(&mut result.vertices, vertex_values, "tessellation vertex payload allocation failed")?;
	reserve_exact(&mut result.normals, normal_values, "tessellation normal payload allocation failed")?;
	reserve_exact(&mut result.indices, index_count, "tessellation index payload allocation failed")?;
	reserve_exact(&mut result.face_tshape_ids, triangle_count, "tessellation face payload allocation failed")?;
	reserve_exact(&mut result.chunk_face_tshape_ids, face_count, "tessellation face payload allocation failed")?;
	reserve_exact(&mut result.chunk_face_indices, face_count, "tessellation face payload allocation failed")?;
	reserve_exact(&mut result.face_vertex_offsets, face_offset_count, "tessellation face-offset allocation failed")?;
	reserve_exact(&mut result.face_index_offsets, face_offset_count, "tessellation face-offset allocation failed")?;
	reserve_exact(&mut result.edge_points, edge_point_values, "tessellation edge payload allocation failed")?;
	reserve_exact(&mut result.chunk_edge_indices, edge_count, "tessellation edge payload allocation failed")?;
	reserve_exact(&mut result.edge_point_offsets, edge_offset_count, "tessellation edge-offset allocation failed")?;
	result.face_vertex_offsets.push(0);
	result.face_index_offsets.push(0);
	result.edge_point_offsets.push(0);

	for (face_index, face) in faces.iter().enumerate() {
		cancellation_checkpoint(progress, face_index)?;
		let vertex_offset = u32::try_from(result.vertices.len() / 3).map_err(|_| resource_limit("tessellation vertex count exceeded index capacity"))?;
		result.chunk_face_indices.push(face.index);
		result.chunk_face_tshape_ids.push(face.tshape_id);
		for (vertex_index, vertex) in face.vertices.iter().enumerate() {
			cancellation_checkpoint(progress, vertex_index)?;
			result.vertices.extend(vertex.to_array());
		}
		for (normal_index, normal) in face.normals.iter().enumerate() {
			cancellation_checkpoint(progress, normal_index)?;
			result.normals.extend(normal.to_array());
		}
		for (index_ordinal, index) in face.indices.iter().enumerate() {
			cancellation_checkpoint(progress, index_ordinal)?;
			result.indices.push(vertex_offset.checked_add(*index).ok_or_else(|| resource_limit("tessellation vertex index overflowed"))?);
		}
		result.face_tshape_ids.extend(std::iter::repeat_n(face.tshape_id, face.indices.len() / 3));
		result.face_vertex_offsets.push(u32::try_from(result.vertices.len() / 3).map_err(|_| resource_limit("tessellation vertex count exceeded index capacity"))?);
		result.face_index_offsets.push(u32::try_from(result.indices.len()).map_err(|_| resource_limit("tessellation index count exceeded index capacity"))?);
	}
	for (edge_ordinal, (points, reversed, edge_index)) in effective_edges.into_iter().enumerate() {
		cancellation_checkpoint(progress, edge_ordinal)?;
		result.chunk_edge_indices.push(edge_index);
		if reversed {
			for (point_index, point) in points.iter().rev().enumerate() {
				cancellation_checkpoint(progress, point_index)?;
				result.edge_points.extend(point.to_array());
			}
		} else {
			for (point_index, point) in points.iter().enumerate() {
				cancellation_checkpoint(progress, point_index)?;
				result.edge_points.extend(point.to_array());
			}
		}
		result.edge_point_offsets.push(u32::try_from(result.edge_points.len() / 3).map_err(|_| resource_limit("tessellation edge-point count exceeded index capacity"))?);
	}
	result.success = true;
	Ok(result)
}

fn point_sequences_match(first: &[DVec3], second: &[DVec3]) -> bool {
	(first.len() == second.len() && first.iter().zip(second).all(|(first, second)| point_key(*first) == point_key(*second))) || (first.len() == second.len() && first.iter().zip(second.iter().rev()).all(|(first, second)| point_key(*first) == point_key(*second)))
}

fn valid_boundary_provenance(vertices: &[BoundaryVertex], edges: &[Vec<DVec3>]) -> bool {
	if vertices.is_empty() {
		return false;
	}
	for vertex in vertices {
		let Some(edge) = edges.get(vertex.edge_index as usize) else {
			return false;
		};
		let sample_index = vertex.edge_sample_index as usize;
		if sample_index >= edge.len()
			|| match vertex.edge_occurrence_direction {
				EdgeOccurrenceDirection::Forward => sample_index + 1 >= edge.len(),
				EdgeOccurrenceDirection::Reversed => sample_index == 0,
			} {
			return false;
		}
	}
	for pair in vertices.windows(2) {
		let first = pair[0];
		let second = pair[1];
		if first.edge_occurrence_index == second.edge_occurrence_index {
			if first.edge_index != second.edge_index || first.edge_occurrence_direction != second.edge_occurrence_direction || !first.edge_occurrence_direction.samples_are_ordered(first.edge_sample_index, second.edge_sample_index) {
				return false;
			}
		} else if first.edge_occurrence_index >= second.edge_occurrence_index {
			// Occurrence ordinals are local to one loop and never reappear after
			// their run has ended. Gaps remain valid when a degenerate occurrence
			// contributes no distinct UV vertex.
			return false;
		}
	}
	true
}

fn validate_face_trim_resource_limits(face_loop_offsets: &[u32], loop_vertex_offsets: &[u32], face_count: usize) -> Result<(), Error> {
	for face in 0..face_count {
		let loop_start = *face_loop_offsets.get(face).ok_or(Error::TriangulationFailed)? as usize;
		let loop_end = *face_loop_offsets.get(face + 1).ok_or(Error::TriangulationFailed)? as usize;
		let trim_start = *loop_vertex_offsets.get(loop_start).ok_or(Error::TriangulationFailed)? as usize;
		let trim_end = *loop_vertex_offsets.get(loop_end).ok_or(Error::TriangulationFailed)? as usize;
		let face_loop_count = loop_end.checked_sub(loop_start).ok_or(Error::TriangulationFailed)?;
		let trim_vertex_count = trim_end.checked_sub(trim_start).ok_or(Error::TriangulationFailed)?;
		if face_loop_count > MAXIMUM_FACE_TRIM_LOOPS {
			return Err(resource_limit("tessellation face exceeded the trim-loop limit"));
		}
		if trim_vertex_count > MAXIMUM_FACE_TRIM_VERTICES {
			return Err(resource_limit("tessellation face exceeded the trim-vertex limit"));
		}
	}
	Ok(())
}

fn decode_source(data: ffi::BrepMeshSourceData) -> Result<BrepMeshSource, Error> {
	let face_count = data.face_indices.len();
	let Some(face_offset_count) = face_count.checked_add(1) else {
		return Err(Error::TriangulationFailed);
	};
	let Some(face_uv_value_count) = face_count.checked_mul(4) else {
		return Err(Error::TriangulationFailed);
	};
	let Some(loop_count) = data.loop_vertex_offsets.len().checked_sub(1) else {
		return Err(Error::TriangulationFailed);
	};
	if !data.success
		|| !data.linear_deflection.is_finite()
		|| data.linear_deflection <= 0.0
		|| data.face_indices.iter().copied().collect::<BTreeSet<_>>().len() != face_count
		|| data.face_tshape_ids.len() != face_count
		|| data.face_reversed.len() != face_count
		|| data.face_reversed.iter().any(|reversed| *reversed > 1)
		|| data.face_u_degrees.len() != face_count
		|| data.face_v_degrees.len() != face_count
		|| data.face_u_pole_counts.len() != face_count
		|| data.face_v_pole_counts.len() != face_count
		|| data.face_uv_bounds.len() != face_uv_value_count
		|| data.face_approximation_errors.len() != face_count
		|| data.face_approximation_errors.iter().any(|error| !error.is_finite() || *error < 0.0 || *error > data.linear_deflection)
		|| !data.poles.len().is_multiple_of(3)
		|| data.weights.len() != data.poles.len() / 3
		|| !data.loop_uvs.len().is_multiple_of(2)
		|| data.loop_edge_indices.len() != data.loop_uvs.len() / 2
		|| data.loop_edge_sample_indices.len() != data.loop_edge_indices.len()
		|| data.loop_edge_occurrence_indices.len() != data.loop_edge_indices.len()
		|| data.loop_edge_occurrence_directions.len() != data.loop_edge_indices.len()
		|| data.loop_edge_occurrence_directions.iter().any(|direction| EdgeOccurrenceDirection::decode(*direction).is_none())
		|| !data.edge_points.len().is_multiple_of(3)
		|| !valid_offsets(&data.face_pole_offsets, face_offset_count, data.poles.len() / 3)
		|| !valid_offsets(&data.face_u_knot_offsets, face_offset_count, data.u_knots.len())
		|| !valid_offsets(&data.face_v_knot_offsets, face_offset_count, data.v_knots.len())
		|| !valid_offsets(&data.face_loop_offsets, face_offset_count, loop_count)
		|| !valid_offsets(&data.loop_vertex_offsets, loop_count + 1, data.loop_uvs.len() / 2)
		|| !valid_offsets(&data.edge_point_offsets, data.edge_point_offsets.len(), data.edge_points.len() / 3)
	{
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			eprintln!("custom tessellation source rejected structural metadata: success={}, faces={}, loops={}, loop_uvs={}, loop_edges={}, loop_samples={}, edges={}, edge_values={}, poles={}, weights={}", data.success, face_count, loop_count, data.loop_uvs.len(), data.loop_edge_indices.len(), data.loop_edge_sample_indices.len(), data.edge_point_offsets.len().saturating_sub(1), data.edge_points.len(), data.poles.len(), data.weights.len());
		}
		return Err(Error::TriangulationFailed);
	}
	validate_face_trim_resource_limits(&data.face_loop_offsets, &data.loop_vertex_offsets, face_count)?;
	let edge_points = data.edge_points.chunks_exact(3).map(|point| DVec3::new(point[0], point[1], point[2])).collect::<Vec<_>>();
	if edge_points.iter().any(|point| !point.is_finite()) {
		return Err(Error::TriangulationFailed);
	}
	let mut edges = Vec::with_capacity(data.edge_point_offsets.len().saturating_sub(1));
	for offsets in data.edge_point_offsets.windows(2) {
		let start = offsets[0] as usize;
		let end = offsets[1] as usize;
		if start > end || end > edge_points.len() || end - start < 2 {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation source rejected edge offsets {start}..{end} of {}", edge_points.len());
			}
			return Err(Error::TriangulationFailed);
		}
		edges.push(edge_points[start..end].to_vec());
	}

	let poles = data.poles.chunks_exact(3).map(|point| DVec3::new(point[0], point[1], point[2])).collect::<Vec<_>>();
	let uvs = data.loop_uvs.chunks_exact(2).map(|point| DVec2::new(point[0], point[1])).collect::<Vec<_>>();
	if uvs.iter().any(|uv| !uv.is_finite()) {
		return Err(Error::TriangulationFailed);
	}

	let mut decoded_loops = Vec::with_capacity(loop_count);
	for offsets in data.loop_vertex_offsets.windows(2) {
		let start = offsets[0] as usize;
		let end = offsets[1] as usize;
		if start > end || end > uvs.len() || end - start < 3 {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation source rejected loop offsets {start}..{end} of {}", uvs.len());
			}
			return Err(Error::TriangulationFailed);
		}
		let mut vertices = Vec::with_capacity(end - start);
		for (relative_index, uv) in uvs[start..end].iter().copied().enumerate() {
			let index = start + relative_index;
			let edge_index = data.loop_edge_indices[index] as usize;
			let sample_index = data.loop_edge_sample_indices[index] as usize;
			let position = *edges.get(edge_index).and_then(|edge| edge.get(sample_index)).ok_or(Error::TriangulationFailed)?;
			let edge_occurrence_direction = EdgeOccurrenceDirection::decode(data.loop_edge_occurrence_directions[index]).ok_or(Error::TriangulationFailed)?;
			vertices.push(BoundaryVertex {
				uv,
				position,
				edge_index: data.loop_edge_indices[index],
				edge_sample_index: data.loop_edge_sample_indices[index],
				edge_occurrence_index: data.loop_edge_occurrence_indices[index],
				edge_occurrence_direction,
			});
		}
		if !valid_boundary_provenance(&vertices, &edges) {
			return Err(Error::TriangulationFailed);
		}
		decoded_loops.push(TrimLoop { vertices });
	}

	let mut faces = Vec::with_capacity(face_count);
	for index in 0..face_count {
		let pole_start = data.face_pole_offsets[index] as usize;
		let pole_end = data.face_pole_offsets[index + 1] as usize;
		let u_knot_start = data.face_u_knot_offsets[index] as usize;
		let u_knot_end = data.face_u_knot_offsets[index + 1] as usize;
		let v_knot_start = data.face_v_knot_offsets[index] as usize;
		let v_knot_end = data.face_v_knot_offsets[index + 1] as usize;
		let loop_start = data.face_loop_offsets[index] as usize;
		let loop_end = data.face_loop_offsets[index + 1] as usize;
		let u_count = data.face_u_pole_counts[index] as usize;
		let v_count = data.face_v_pole_counts[index] as usize;
		let u_degree = data.face_u_degrees[index] as usize;
		let v_degree = data.face_v_degrees[index] as usize;
		let expected_pole_count = u_count.checked_mul(v_count).ok_or(Error::TriangulationFailed)?;
		let expected_u_knot_count = u_count.checked_add(u_degree).and_then(|count| count.checked_add(1)).ok_or(Error::TriangulationFailed)?;
		let expected_v_knot_count = v_count.checked_add(v_degree).and_then(|count| count.checked_add(1)).ok_or(Error::TriangulationFailed)?;
		if u_count < 2 || v_count < 2 || u_degree == 0 || v_degree == 0 || u_degree >= u_count || v_degree >= v_count || pole_end - pole_start != expected_pole_count || u_knot_end - u_knot_start != expected_u_knot_count || v_knot_end - v_knot_start != expected_v_knot_count || loop_start >= loop_end {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation source rejected face {} dimensions: degrees {u_degree}/{v_degree}, counts {u_count}/{v_count}, poles {} expected {expected_pole_count}, knots {}/{} expected {expected_u_knot_count}/{expected_v_knot_count}, loops {loop_start}..{loop_end}", data.face_indices[index], pole_end - pole_start, u_knot_end - u_knot_start, v_knot_end - v_knot_start);
			}
			return Err(Error::TriangulationFailed);
		}
		let uv_bounds: [f64; 4] = data.face_uv_bounds[index * 4..index * 4 + 4].try_into().map_err(|_| Error::TriangulationFailed)?;
		let weights = data.weights[pole_start..pole_end].to_vec();
		let u_knots = data.u_knots[u_knot_start..u_knot_end].to_vec();
		let v_knots = data.v_knots[v_knot_start..v_knot_end].to_vec();
		if !valid_surface_values(&poles[pole_start..pole_end], &weights, &u_knots, &v_knots, uv_bounds, u_count, v_count, u_degree, v_degree) {
			if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
				eprintln!("custom tessellation source rejected face {} rational surface values", data.face_indices[index]);
			}
			return Err(Error::TriangulationFailed);
		}
		let poles = poles[pole_start..pole_end].to_vec();
		let origin = poles[0];
		let local_poles = poles.iter().map(|pole| *pole - origin).collect();
		let surface = RationalSurface { u_degree, v_degree, u_count, v_count, poles, local_poles, origin, weights, u_knots, v_knots, uv_bounds, approximation_error: data.face_approximation_errors[index] };
		for vertex in decoded_loops[loop_start..loop_end].iter().flat_map(|trim_loop| &trim_loop.vertices) {
			if !uv_is_within_bounds(vertex.uv, uv_bounds) {
				if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
					eprintln!("custom tessellation source rejected face {} trim uv {:?} outside {:?}", data.face_indices[index], vertex.uv, uv_bounds);
				}
				return Err(Error::TriangulationFailed);
			}
			let position = surface.evaluate_position(vertex.uv).ok_or(Error::TriangulationFailed)?;
			let boundary_error = position.distance(vertex.position);
			let numeric_tolerance = position.length().max(vertex.position.length()).max(1.0) * 1.0e-10;
			let allowed_error = surface.approximation_error + (data.linear_deflection * 1.0e-6).max(numeric_tolerance);
			if !boundary_error.is_finite() || boundary_error > allowed_error {
				if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
					eprintln!("custom tessellation source rejected face {} boundary uv {:?}: error {boundary_error:.12e} > {allowed_error:.12e}, surface {:?}, edge {:?}", data.face_indices[index], vertex.uv, position, vertex.position);
				}
				return Err(Error::TriangulationFailed);
			}
		}
		if std::env::var_os("PLEX_TESSELLATION_DIAGNOSTICS").is_some() {
			let mut maximum_boundary_error = 0.0_f64;
			let mut worst = None;
			for trim_loop in &decoded_loops[loop_start..loop_end] {
				for vertex in &trim_loop.vertices {
					let Some(position) = surface.evaluate_position(vertex.uv) else {
						continue;
					};
					let error = position.distance(vertex.position);
					if error > maximum_boundary_error {
						maximum_boundary_error = error;
						worst = Some((vertex.uv, position, vertex.position));
					}
				}
			}
			eprintln!("custom tessellation face {}: boundary error {:.12e}, bounds {:?}, worst {:?}", data.face_indices[index], maximum_boundary_error, uv_bounds, worst);
		}
		let collapsed_boundaries = collapsed_boundaries(&surface);
		faces.push(TrimmedFace {
			index: data.face_indices[index],
			tshape_id: data.face_tshape_ids[index],
			reversed: data.face_reversed[index] != 0,
			surface,
			loops: decoded_loops[loop_start..loop_end].to_vec(),
			collapsed_boundaries,
		});
	}
	Ok(BrepMeshSource { faces, edges, linear_deflection: data.linear_deflection })
}

#[cfg(feature = "test-support")]
pub(super) struct BoundaryRunProvenance {
	pub face_index: u32,
	pub loop_index: u32,
	pub edge_index: u32,
	pub edge_occurrence_index: u32,
	pub reversed: bool,
	pub sample_ordinals: Vec<u32>,
}

#[cfg(feature = "test-support")]
pub(super) fn boundary_occurrence_metadata_overflow_is_rejected() -> bool {
	let mut vertex = ParametricVertex { uv: DVec2::ZERO, metric: Point2::new(0.0, 0.0), boundary_position: None, boundary_occurrences: [None, None] };
	let occurrence = |loop_index| BoundaryOccurrence { loop_index, edge_index: 7, occurrence_index: loop_index };
	vertex.add_boundary_occurrence(occurrence(0)).is_ok() && vertex.add_boundary_occurrence(occurrence(0)).is_ok() && vertex.add_boundary_occurrence(occurrence(1)).is_ok() && vertex.add_boundary_occurrence(occurrence(2)).is_err()
}

#[cfg(feature = "test-support")]
pub(super) fn synthetic_trim_resource_limit_errors() -> Result<(Error, Error), Error> {
	validate_face_trim_resource_limits(&[0, 1], &[0, MAXIMUM_FACE_TRIM_VERTICES as u32], 1)?;
	let trim_vertex_error = validate_face_trim_resource_limits(&[0, 1], &[0, MAXIMUM_FACE_TRIM_VERTICES as u32 + 1], 1).expect_err("one trim vertex over the quota must fail");

	let loop_offsets_at_limit = vec![0; MAXIMUM_FACE_TRIM_LOOPS + 1];
	validate_face_trim_resource_limits(&[0, MAXIMUM_FACE_TRIM_LOOPS as u32], &loop_offsets_at_limit, 1)?;
	let loop_offsets_over_limit = vec![0; MAXIMUM_FACE_TRIM_LOOPS + 2];
	let trim_loop_error = validate_face_trim_resource_limits(&[0, MAXIMUM_FACE_TRIM_LOOPS as u32 + 1], &loop_offsets_over_limit, 1).expect_err("one trim loop over the quota must fail");
	Ok((trim_vertex_error, trim_loop_error))
}

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub(super) fn synthetic_aggregate_resource_limit_error(parallel: bool) -> Result<Error, Error> {
	use rayon::prelude::*;

	const FACE_COUNT_AT_LIMIT: usize = 64;
	let face_vertices = MAXIMUM_REQUEST_VERTICES / FACE_COUNT_AT_LIMIT;
	let face_triangles = MAXIMUM_REQUEST_TRIANGLES / FACE_COUNT_AT_LIMIT;
	let face_indices = MAXIMUM_REQUEST_INDICES / FACE_COUNT_AT_LIMIT;
	let face_payload = checked_add_resource(checked_mul_resource(face_vertices, 48, "synthetic payload overflowed")?, checked_mul_resource(face_triangles, 20, "synthetic payload overflowed")?, "synthetic payload overflowed")?;
	let budget = RequestMeshBudget::default();
	let admit = |_| budget.admit_counts(face_vertices, face_triangles, face_indices, face_payload);
	let result = if parallel { bounded_face_pool()?.install(|| (0..=FACE_COUNT_AT_LIMIT).into_par_iter().try_for_each(admit)) } else { (0..=FACE_COUNT_AT_LIMIT).try_for_each(admit) };
	Ok(result.expect_err("one synthetic face over the aggregate quota must fail"))
}

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub(super) fn synthetic_resource_accounting_totals(worker_count: usize) -> Result<[usize; 4], Error> {
	use rayon::prelude::*;

	const FACE_COUNT: usize = 64;
	let face_vertices = MAXIMUM_REQUEST_VERTICES / FACE_COUNT;
	let face_triangles = MAXIMUM_REQUEST_TRIANGLES / FACE_COUNT;
	let face_indices = MAXIMUM_REQUEST_INDICES / FACE_COUNT;
	let face_payload = checked_add_resource(checked_mul_resource(face_vertices, 48, "synthetic payload overflowed")?, checked_mul_resource(face_triangles, 20, "synthetic payload overflowed")?, "synthetic payload overflowed")?;
	let budget = RequestMeshBudget::default();
	let pool = rayon::ThreadPoolBuilder::new().num_threads(worker_count).build().map_err(|_| resource_limit("synthetic accounting worker pool failed"))?;
	pool.install(|| (0..FACE_COUNT).into_par_iter().try_for_each(|_| budget.admit_counts(face_vertices, face_triangles, face_indices, face_payload)))?;
	let totals = budget.snapshot()?;
	Ok([totals.vertices, totals.triangles, totals.indices, totals.payload_bytes])
}

#[cfg(feature = "test-support")]
pub(super) fn synthetic_transition_ring_resource_limit_error() -> Result<Error, Error> {
	let transition_vertices = transition_stage_vertex_count(MAXIMUM_FACE_TRIM_VERTICES, MAXIMUM_FACE_TRIM_VERTICES, 64)?;
	if transition_vertices <= MAXIMUM_FACE_VERTICES {
		return Err(Error::TriangulationFailed);
	}
	Ok(resource_limit("tessellation inset structured transition rings exceeded the vertex limit"))
}

#[cfg(feature = "test-support")]
pub(super) fn synthetic_payload_resource_limit_error() -> Result<Error, Error> {
	let fixed_bytes = validate_request_payload_bytes(0, 0, 0, 0, 0)?;
	let edge_points_at_limit = (MAXIMUM_REQUEST_PAYLOAD_BYTES - fixed_bytes) / 24;
	validate_request_payload_bytes(0, 0, 0, edge_points_at_limit, 0)?;
	Ok(validate_request_payload_bytes(0, 0, 0, edge_points_at_limit + 1, 0).expect_err("one edge point over the payload quota must fail"))
}

#[cfg(feature = "test-support")]
pub(super) fn decode_boundary_run_provenance(data: ffi::BrepMeshSourceData) -> Result<Vec<BoundaryRunProvenance>, Error> {
	let source = decode_source(data)?;
	let mut runs = Vec::new();
	for face in source.faces {
		for (loop_index, trim_loop) in face.loops.into_iter().enumerate() {
			let mut start = 0;
			while start < trim_loop.vertices.len() {
				let first = trim_loop.vertices[start];
				let mut end = start + 1;
				while end < trim_loop.vertices.len() && trim_loop.vertices[end].edge_occurrence_index == first.edge_occurrence_index {
					end += 1;
				}
				runs.push(BoundaryRunProvenance {
					face_index: face.index,
					loop_index: u32::try_from(loop_index).map_err(|_| Error::TriangulationFailed)?,
					edge_index: first.edge_index,
					edge_occurrence_index: first.edge_occurrence_index,
					reversed: first.edge_occurrence_direction == EdgeOccurrenceDirection::Reversed,
					sample_ordinals: trim_loop.vertices[start..end].iter().map(|vertex| vertex.edge_sample_index).collect(),
				});
				start = end;
			}
		}
	}
	Ok(runs)
}

fn valid_offsets(offsets: &[u32], expected_offset_count: usize, item_count: usize) -> bool {
	offsets.len() == expected_offset_count && offsets.first() == Some(&0) && offsets.last().is_some_and(|offset| *offset as usize == item_count) && offsets.windows(2).all(|pair| pair[0] <= pair[1])
}

#[allow(clippy::too_many_arguments)]
fn valid_surface_values(poles: &[DVec3], weights: &[f64], u_knots: &[f64], v_knots: &[f64], uv_bounds: [f64; 4], u_count: usize, v_count: usize, u_degree: usize, v_degree: usize) -> bool {
	if !poles.iter().all(|point| point.is_finite()) || !weights.iter().all(|weight| weight.is_finite() && *weight > 0.0) || !valid_knot_vector(u_knots, u_count, u_degree) || !valid_knot_vector(v_knots, v_count, v_degree) || !uv_bounds.iter().all(|value| value.is_finite()) || uv_bounds[0] >= uv_bounds[1] || uv_bounds[2] >= uv_bounds[3] {
		return false;
	}
	let u_domain = [u_knots[u_degree], u_knots[u_count]];
	let v_domain = [v_knots[v_degree], v_knots[v_count]];
	u_domain[0] < u_domain[1] && v_domain[0] < v_domain[1] && coordinate_is_within_domain(uv_bounds[0], u_domain) && coordinate_is_within_domain(uv_bounds[1], u_domain) && coordinate_is_within_domain(uv_bounds[2], v_domain) && coordinate_is_within_domain(uv_bounds[3], v_domain)
}

fn valid_knot_vector(knots: &[f64], control_count: usize, degree: usize) -> bool {
	let Some(expected_count) = control_count.checked_add(degree).and_then(|count| count.checked_add(1)) else {
		return false;
	};
	if knots.len() != expected_count || !knots.iter().all(|value| value.is_finite()) || !knots.windows(2).all(|pair| pair[0] <= pair[1]) {
		return false;
	}

	let mut run_start = 0;
	while run_start < knots.len() {
		let mut run_end = run_start + 1;
		while run_end < knots.len() && knots[run_end] == knots[run_start] {
			run_end += 1;
		}
		let multiplicity = run_end - run_start;
		// Non-periodic end knots may be clamped to degree + 1. An
		// interior knot at that multiplicity disconnects the surface; OCCT's
		// valid B-spline representation limits it to the degree instead.
		let maximum = if run_start == 0 || run_end == knots.len() { degree + 1 } else { degree };
		if multiplicity > maximum {
			return false;
		}
		run_start = run_end;
	}
	true
}

fn uv_is_within_bounds(uv: DVec2, bounds: [f64; 4]) -> bool {
	coordinate_is_within_domain(uv.x, [bounds[0], bounds[1]]) && coordinate_is_within_domain(uv.y, [bounds[2], bounds[3]])
}

fn coordinate_is_within_domain(value: f64, domain: [f64; 2]) -> bool {
	let scale = value.abs().max(domain[0].abs()).max(domain[1].abs()).max((domain[1] - domain[0]).abs()).max(1.0);
	let tolerance = scale * 1.0e-9;
	value >= domain[0] - tolerance && value <= domain[1] + tolerance
}
