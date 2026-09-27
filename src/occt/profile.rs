use super::{edge::Edge, ffi};
use crate::{Error, ProfileIssue};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfilePointLocation {
	Outside,
	Inside,
	Boundary,
}

/// A bounded interval of one original curve, before wire traversal reversal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfileCurveSpan {
	pub source: u32,
	pub first: f64,
	pub last: f64,
	pub reversed: bool,
}

/// One bounded planar cell, with its outer wire first and clockwise holes afterward.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarProfileRegion {
	pub area: f64,
	pub wires: Vec<Vec<ProfileCurveSpan>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlanarProfileArrangement {
	pub regions: Vec<PlanarProfileRegion>,
	pub unused_sources: Vec<u32>,
}

impl Edge {
	/// Classifies XY points against exact outer/hole wires using an absolute distance tolerance.
	pub fn classify_planar_region<'a, W>(wires: impl IntoIterator<Item = W>, points: &[glam::DVec2], tolerance: f64, progress: &ffi::CancellationToken) -> Result<Vec<ProfilePointLocation>, Error>
	where
		W: IntoIterator<Item = &'a Edge>,
	{
		if !tolerance.is_finite() || tolerance < 1.0e-7 || points.iter().any(|point| !point.is_finite()) {
			return Err(Error::InvalidInput("profile query points must be finite and tolerance at least 1e-7".into()));
		}
		if progress.is_cancelled() {
			return Err(Error::Cancelled);
		}
		let mut input = ffi::edge_vec_new();
		for (index, wire) in wires.into_iter().enumerate() {
			if index > 0 {
				ffi::edge_vec_push_null(input.pin_mut());
			}
			let mut count = 0;
			for edge in wire {
				ffi::edge_vec_push(input.pin_mut(), &edge.inner);
				count += 1;
			}
			if count == 0 {
				return Err(Error::InvalidInput("profile query wires cannot be empty".into()));
			}
		}
		let points = points.iter().flat_map(|point| [point.x, point.y]).collect::<Vec<_>>();
		let mut locations = Vec::new();
		ffi::begin_operation();
		if !ffi::classify_planar_profile(&input, &points, tolerance, progress, &mut locations) {
			return Err(if progress.is_cancelled() { Error::Cancelled } else { ffi::operation_error(Error::TopologyQueryFailed, "classify profile points", "classify") });
		}
		if locations.len() != points.len() / 2 {
			return Err(Error::TopologyQueryFailed);
		}
		locations
			.into_iter()
			.map(|location| match location {
				0 => Ok(ProfilePointLocation::Outside),
				1 => Ok(ProfilePointLocation::Inside),
				2 => Ok(ProfilePointLocation::Boundary),
				_ => Err(Error::TopologyQueryFailed),
			})
			.collect()
	}

	/// Splits XY-plane curves into exact bounded cells; no display sampling or hole inference is used.
	pub fn planar_regions(edges: &[Self], tolerance: f64, max_fragments: u32, progress: &ffi::CancellationToken) -> Result<PlanarProfileArrangement, Error> {
		if !tolerance.is_finite() || tolerance < 1.0e-7 || max_fragments == 0 {
			return Err(Error::InvalidInput("profile tolerance must be at least 1e-7 and the fragment limit must be positive".into()));
		}
		if progress.is_cancelled() {
			return Err(Error::Cancelled);
		}
		if edges.len() > max_fragments as usize {
			return Err(Error::InvalidProfile { issue: ProfileIssue::ResourceLimit, sources: Vec::new() });
		}
		let mut input = ffi::edge_vec_new();
		for edge in edges {
			ffi::edge_vec_push(input.pin_mut(), &edge.inner);
		}
		ffi::begin_operation();
		let result = ffi::arrange_planar_edges(&input, tolerance, max_fragments, progress);
		if result.unused_sources.iter().chain(&result.error_sources).any(|&source| source as usize >= edges.len()) {
			return Err(Error::TopologyQueryFailed);
		}
		if !result.success {
			if progress.is_cancelled() {
				return Err(Error::Cancelled);
			}
			let issue = match result.error_code {
				1 => ProfileIssue::NonPlanar,
				2 => ProfileIssue::CoincidentCurves,
				3 => ProfileIssue::AmbiguousJunction,
				4 => ProfileIssue::ResourceLimit,
				_ => return Err(ffi::operation_error(Error::TopologyQueryFailed, "arrange planar curves", "build regions")),
			};
			return Err(Error::InvalidProfile { issue, sources: result.error_sources.into_iter().collect() });
		}
		let mut regions = Vec::new();
		for region in result.regions {
			if !region.area.is_finite() || region.area <= 0.0 || region.wire_offsets.len() < 2 || region.wire_offsets.first() != Some(&0) || region.wire_offsets.last().copied() != Some(region.spans.len() as u32) {
				return Err(Error::TopologyQueryFailed);
			}
			let mut wires = Vec::new();
			for pair in region.wire_offsets.windows(2) {
				if pair[0] >= pair[1] || pair[1] as usize > region.spans.len() {
					return Err(Error::TopologyQueryFailed);
				}
				let mut wire = Vec::new();
				for span in &region.spans[pair[0] as usize..pair[1] as usize] {
					if span.source as usize >= edges.len() || !span.first.is_finite() || !span.last.is_finite() || span.first < 0.0 || span.last > 1.0 || span.first >= span.last {
						return Err(Error::TopologyQueryFailed);
					}
					wire.push(ProfileCurveSpan { source: span.source, first: span.first, last: span.last, reversed: span.reversed });
				}
				wires.push(wire);
			}
			regions.push(PlanarProfileRegion { area: region.area, wires });
		}
		Ok(PlanarProfileArrangement { regions, unused_sources: result.unused_sources.into_iter().collect() })
	}

	/// Reconstructs an exact normalized source interval, preserving its underlying curve.
	pub fn profile_span(&self, first: f64, last: f64, reversed: bool, tolerance: f64) -> Result<Self, Error> {
		if !first.is_finite() || !last.is_finite() || first < 0.0 || last > 1.0 || first >= last || !tolerance.is_finite() || tolerance < 1.0e-7 {
			return Err(Error::InvalidInput("invalid normalized profile curve interval or tolerance".into()));
		}
		ffi::begin_operation();
		Self::try_from_ffi(ffi::trim_profile_edge(&self.inner, first, last, reversed, tolerance), "profile interval could not be constructed".into())
	}
}
