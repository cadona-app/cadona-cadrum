use cadrum::{CancellationToken, DVec2, DVec3, Edge, Error, PlanarProfileRegion, ProfilePointLocation};

const TOLERANCE: f64 = 1.0e-7;

fn classify(sources: &[Edge], region: &PlanarProfileRegion, points: &[DVec2]) -> Vec<ProfilePointLocation> {
	let wires = region.wires.iter().map(|wire| wire.iter().map(|span| sources[span.source as usize].profile_span(span.first, span.last, span.reversed, TOLERANCE).unwrap()).collect::<Vec<_>>()).collect::<Vec<_>>();
	Edge::classify_planar_region(wires.iter().map(|wire| wire.iter()), points, TOLERANCE, &CancellationToken::new()).unwrap()
}

#[test]
fn exact_circle_classification_does_not_use_display_chords() {
	let sources = [Edge::circle(5., DVec3::Z).unwrap()];
	let arrangement = Edge::planar_regions(&sources, TOLERANCE, 4096, &CancellationToken::new()).unwrap();
	let mut points = Vec::new();
	for index in 0..96 {
		let angle = (index as f64 + 0.5) * std::f64::consts::TAU / 96.;
		let direction = DVec2::new(angle.cos(), angle.sin());
		for radius in [5. - 1.0e-4, 5. + 1.0e-4, 5.] {
			points.push(direction * radius);
		}
	}
	let result = classify(&sources, &arrangement.regions[0], &points);
	for locations in result.chunks_exact(3) {
		assert_eq!(locations, [ProfilePointLocation::Inside, ProfilePointLocation::Outside, ProfilePointLocation::Boundary]);
	}
}

#[test]
fn holes_and_boundary_tolerance_have_distinct_outcomes() {
	let sources = [Edge::circle(5., DVec3::Z).unwrap(), Edge::circle(2., DVec3::Z).unwrap()];
	let arrangement = Edge::planar_regions(&sources, TOLERANCE, 4096, &CancellationToken::new()).unwrap();
	let annulus = arrangement.regions.iter().find(|region| region.wires.len() == 2).unwrap();
	let points = [0., 2., 3., 5., 6., 5. - TOLERANCE * 0.5, 5. + TOLERANCE * 0.5, 5. - TOLERANCE * 2., 5. + TOLERANCE * 2.].map(|x| DVec2::new(x, 0.));
	use ProfilePointLocation::{Boundary, Inside, Outside};
	assert_eq!(classify(&sources, annulus, &points), [Outside, Boundary, Inside, Boundary, Outside, Boundary, Boundary, Inside, Outside]);
	assert!(classify(&sources, annulus, &[]).is_empty());
}

#[test]
fn invalid_profiles_and_query_points_fail_instead_of_returning_outside() {
	let token = CancellationToken::new();
	let points = [DVec2::ZERO];
	let open = [Edge::line(DVec3::ZERO, DVec3::X).unwrap()];
	assert!(Edge::classify_planar_region([&open], &points, TOLERANCE, &token).is_err());
	let empty: [Edge; 0] = [];
	assert!(Edge::classify_planar_region([&empty], &points, TOLERANCE, &token).is_err());
	let no_wires: [&[Edge]; 0] = [];
	assert!(Edge::classify_planar_region(no_wires, &points, TOLERANCE, &token).is_err());
	let raised = [Edge::circle(5., DVec3::Z).unwrap().translate(DVec3::Z)];
	assert!(Edge::classify_planar_region([&raised], &points, TOLERANCE, &token).is_err());
	let clockwise = [Edge::circle(5., DVec3::Z).unwrap().profile_span(0., 1., true, TOLERANCE).unwrap()];
	assert!(Edge::classify_planar_region([&clockwise], &points, TOLERANCE, &token).is_err());
	let disk = [Edge::circle(5., DVec3::Z).unwrap()];
	assert!(matches!(Edge::classify_planar_region([&disk], &[DVec2::new(f64::NAN, 0.)], TOLERANCE, &token), Err(Error::InvalidInput(_))));
	assert!(matches!(Edge::classify_planar_region([&disk], &points, TOLERANCE * 0.5, &token), Err(Error::InvalidInput(_))));
	let token = CancellationToken::new();
	token.cancel();
	assert!(matches!(Edge::classify_planar_region([&disk], &points, TOLERANCE, &token), Err(Error::Cancelled)));
}

#[test]
fn cancellation_during_a_batch_does_not_publish_partial_locations() {
	let polls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
	let counter = polls.clone();
	let token = CancellationToken::new().with_cancellation_check(move || counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) >= 20);
	let disk = [Edge::circle(5., DVec3::Z).unwrap()];
	let points = vec![DVec2::ZERO; 1000];
	assert!(matches!(Edge::classify_planar_region([&disk], &points, TOLERANCE, &token), Err(Error::Cancelled)));
	assert!(polls.load(std::sync::atomic::Ordering::Relaxed) >= 20);
}
