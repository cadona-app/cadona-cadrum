use cadrum::{CancellationToken, DVec3, Edge, Error, PlanarProfileArrangement, PlanarProfileRegion, ProfileIssue, Solid};

const TOLERANCE: f64 = 1.0e-7;

fn line(a: [f64; 2], b: [f64; 2]) -> Edge {
	Edge::line(DVec3::new(a[0], a[1], 0.0), DVec3::new(b[0], b[1], 0.0)).unwrap()
}

fn square() -> Vec<Edge> {
	vec![line([0., 0.], [10., 0.]), line([10., 0.], [10., 10.]), line([10., 10.], [0., 10.]), line([0., 10.], [0., 0.])]
}

fn circle(x: f64, radius: f64) -> Edge {
	Edge::circle(radius, DVec3::Z).unwrap().translate(DVec3::X * x)
}

fn arrange(edges: &[Edge]) -> PlanarProfileArrangement {
	Edge::planar_regions(edges, TOLERANCE, 4096, &CancellationToken::new()).unwrap()
}

fn extrude(edges: &[Edge], region: &PlanarProfileRegion, distance: f64, volume_tolerance: f64) -> Solid {
	let wires = region.wires.iter().map(|wire| wire.iter().map(|span| edges[span.source as usize].profile_span(span.first, span.last, span.reversed, TOLERANCE).unwrap()).collect::<Vec<_>>()).collect::<Vec<_>>();
	let solid = Solid::extrude_wires_cancelable(wires.iter().map(|wire| wire.iter()), DVec3::Z * distance, &CancellationToken::new()).unwrap();
	assert!(solid.validate().unwrap().valid);
	assert!((solid.volume() - region.area * distance.abs()).abs() < volume_tolerance, "volume {} expected {}", solid.volume(), region.area * distance.abs());
	solid
}

#[test]
fn independent_lines_and_an_interior_divider_build_exact_bounded_cells() {
	let mut edges = square();
	let first = arrange(&edges);
	assert_eq!(first.regions.len(), 1);
	assert!((first.regions[0].area - 100.).abs() < 1.0e-9);
	edges.push(line([0., 5.], [10., 5.]));
	let split = arrange(&edges);
	assert_eq!(split.regions.len(), 2);
	assert!(split.unused_sources.is_empty());
	for region in split.regions {
		assert!((region.area - 50.).abs() < 1.0e-9);
		assert!(region.wires[0].iter().any(|span| span.source == 4));
		for distance in [3., -3.] {
			extrude(&edges, &region, distance, 1.0e-7);
		}
	}
}

#[test]
fn nested_circles_form_annular_cells_without_filling_holes() {
	let edges = vec![circle(0., 6.), circle(0., 4.), circle(0., 1.)];
	let result = arrange(&edges);
	assert_eq!(result.regions.len(), 3);
	let mut areas = result.regions.iter().map(|region| region.area / std::f64::consts::PI).collect::<Vec<_>>();
	areas.sort_by(f64::total_cmp);
	for (actual, expected) in areas.into_iter().zip([1., 15., 20.]) {
		assert!((actual - expected).abs() < 1.0e-9);
	}
	assert_eq!(result.regions.iter().filter(|region| region.wires.len() == 2).count(), 2);
	for region in &result.regions {
		extrude(&edges, region, 3., 1.0e-7);
	}
}

#[test]
fn overlapping_circles_preserve_exact_lens_and_crescent_boundaries() {
	let edges = vec![circle(0., 5.), circle(5., 5.)];
	let result = arrange(&edges);
	let lens = 50. * (0.5_f64).acos() - 2.5 * 75_f64.sqrt();
	let mut areas = result.regions.iter().map(|region| region.area).collect::<Vec<_>>();
	areas.sort_by(f64::total_cmp);
	assert_eq!(areas.len(), 3);
	for (actual, expected) in areas.into_iter().zip([lens, 25. * std::f64::consts::PI - lens, 25. * std::f64::consts::PI - lens]) {
		assert!((actual - expected).abs() < 1.0e-8);
	}
	for region in &result.regions {
		assert!(region.wires[0].iter().any(|span| span.source == 0));
		assert!(region.wires[0].iter().any(|span| span.source == 1));
		extrude(&edges, region, 3., 1.0e-7);
	}
}

#[test]
fn tangency_dangling_edges_and_a_single_closed_curve_have_explicit_outcomes() {
	assert_eq!(arrange(&[circle(0., 5.), circle(10., 5.)]).regions.len(), 2);
	let single = vec![circle(0., 5.)];
	let disk = arrange(&single);
	assert_eq!(disk.regions.len(), 1);
	extrude(&single, &disk.regions[0], 3., 1.0e-7);
	let mut edges = square();
	edges.push(line([5., 5.], [15., 5.]));
	let dangling = arrange(&edges);
	assert_eq!(dangling.regions.len(), 1);
	assert!((dangling.regions[0].area - 100.).abs() < 1.0e-9);
	assert_eq!(dangling.unused_sources, vec![4]);
	let open = arrange(&[line([0., 0.], [1., 0.])]);
	assert!(open.regions.is_empty());
	assert_eq!(open.unused_sources, vec![0]);
}

#[test]
fn coincident_and_nonplanar_sources_are_localized() {
	let mut edges = square();
	edges.push(line([10., 0.], [0., 0.]));
	assert!(matches!(Edge::planar_regions(&edges, TOLERANCE, 4096, &CancellationToken::new()), Err(Error::InvalidProfile { issue: ProfileIssue::CoincidentCurves, sources }) if sources == vec![0,4]));
	assert!(matches!(Edge::planar_regions(&[circle(0., 1.).translate(DVec3::Z)], TOLERANCE, 4096, &CancellationToken::new()), Err(Error::InvalidProfile { issue: ProfileIssue::NonPlanar, sources }) if sources == vec![0]));
}

#[test]
fn joining_cannot_silently_exceed_the_requested_tolerance() {
	let mut edges = square();
	edges[3] = line([0., 10.], [0., TOLERANCE * 0.5]);
	let closed = arrange(&edges);
	assert_eq!(closed.regions.len(), 1);
	extrude(&edges, &closed.regions[0], 3., 40. * TOLERANCE * 3.);
	edges[3] = line([0., 10.], [0., TOLERANCE * 1.5]);
	assert!(matches!(Edge::planar_regions(&edges, TOLERANCE, 4096, &CancellationToken::new()), Err(Error::InvalidProfile { issue: ProfileIssue::AmbiguousJunction, .. })));
	edges[3] = line([0., 10.], [0., 1.0e-3]);
	assert!(arrange(&edges).regions.is_empty());
}

#[test]
fn cancellation_and_fragment_limits_fail_closed() {
	let token = CancellationToken::new();
	token.cancel();
	assert!(matches!(Edge::planar_regions(&square(), TOLERANCE, 4096, &token), Err(Error::Cancelled)));
	assert!(matches!(Edge::planar_regions(&square(), TOLERANCE, 3, &CancellationToken::new()), Err(Error::InvalidProfile { issue: ProfileIssue::ResourceLimit, .. })));
	assert!(matches!(Edge::planar_regions(&[circle(0., 5.), circle(5., 5.)], TOLERANCE, 5, &CancellationToken::new()), Err(Error::InvalidProfile { issue: ProfileIssue::ResourceLimit, .. })));
}

#[test]
fn region_areas_and_classification_survive_scale_translation_and_input_reordering() {
	let lens = 50. * (0.5_f64).acos() - 2.5 * 75_f64.sqrt();
	for (scale, offset) in [(1.0e-3, DVec3::ZERO), (1., DVec3::new(1.0e9, -1.0e9, 0.)), (1.0e4, DVec3::ZERO)] {
		for reverse in [false, true] {
			let mut edges = vec![circle(0., 5.), circle(5., 5.)].into_iter().map(|edge| edge.scale(DVec3::ZERO, scale).translate(offset)).collect::<Vec<_>>();
			if reverse {
				edges.reverse();
			}
			let result = arrange(&edges);
			assert_eq!(result.regions.len(), 3);
			let mut areas = result.regions.iter().map(|region| region.area / scale.powi(2)).collect::<Vec<_>>();
			areas.sort_by(f64::total_cmp);
			for (actual, expected) in areas.into_iter().zip([lens, 25. * std::f64::consts::PI - lens, 25. * std::f64::consts::PI - lens]) {
				assert!((actual - expected).abs() < 1.0e-5, "scale {scale} offset {offset}: area {actual} expected {expected}");
			}
		}
	}
}

#[test]
fn self_intersecting_outline_splits_into_two_bounded_cells() {
	let edges = vec![line([-5., -5.], [5., 5.]), line([5., 5.], [-5., 5.]), line([-5., 5.], [5., -5.]), line([5., -5.], [-5., -5.])];
	let result = arrange(&edges);
	assert_eq!(result.regions.len(), 2);
	for region in &result.regions {
		assert!((region.area - 25.).abs() < 1.0e-9);
		extrude(&edges, region, 3., 1.0e-7);
	}
}

#[test]
fn internally_tangent_hole_is_reported_as_a_pinched_region() {
	let error = Edge::planar_regions(&[circle(0., 5.), circle(3., 2.)], TOLERANCE, 4096, &CancellationToken::new()).unwrap_err();
	assert!(matches!(error, Error::InvalidProfile { issue: ProfileIssue::AmbiguousJunction, sources } if sources.len() == 2));
}
