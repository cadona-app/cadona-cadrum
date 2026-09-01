use std::{
	thread,
	time::{Duration, Instant},
};

use cadrum::{Boolean, CancellationToken, DVec3, Error, Solid, Tessellation};

#[test]
fn cancelled_fillet_is_distinct_from_an_algorithm_failure() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let edge = cube.iter_edge().next().expect("cube edge");
	let cancellation = CancellationToken::new();
	cancellation.cancel();

	let result = cube.fillet_edges_cancelable(1.0, [edge], &cancellation);
	assert!(matches!(result, Err(Error::Cancelled)));
}

#[test]
fn cancelled_extrusion_stops_before_publishing_partial_topology() {
	let profile = cadrum::Edge::polygon(&[DVec3::ZERO, DVec3::X * 10.0, DVec3::new(10.0, 10.0, 0.0), DVec3::Y * 10.0]).expect("profile");
	let cancellation = CancellationToken::new();
	cancellation.cancel();

	assert!(matches!(Solid::extrude_cancelable(&profile, DVec3::Z * 10.0, &cancellation), Err(Error::Cancelled)));
}

#[test]
fn cancelled_boolean_does_not_poison_later_occt_work() {
	let left = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let right = Solid::cube(DVec3::splat(5.0), DVec3::splat(15.0));
	let expression = Boolean::from(&left) + Boolean::from(&right);
	let cancellation = CancellationToken::new();
	cancellation.cancel();

	assert!(matches!(Solid::boolean_build_cancelable(&expression, &cancellation), Err(Error::Cancelled)));
	let result = (Boolean::from(&left) + Boolean::from(&right)).build_vec().expect("later boolean");
	assert_eq!(result.len(), 1);
}

#[test]
fn completed_builder_reports_progress() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let edge = cube.iter_edge().next().expect("cube edge");
	let progress = CancellationToken::new();

	cube.fillet_edges_cancelable(1.0, [edge], &progress).expect("fillet");
	assert!(progress.progress() > 0.0);
	assert!(progress.progress() <= 1.0);
}

#[test]
fn cancelled_presentation_does_not_publish_partial_chunks() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let cancellation = CancellationToken::new();
	cancellation.cancel();

	assert!(matches!(Solid::mesh_chunks_cancelable([&cube], Tessellation::default(), &cancellation), Err(Error::Cancelled)));
	assert!(matches!(cube.mesh_face_chunks_cancelable(&[0, 1], Tessellation::default(), &cancellation), Err(Error::Cancelled)));
	assert!(matches!(cube.edge_polyline_chunks_cancelable(Tessellation::default(), &cancellation), Err(Error::Cancelled)));
	assert_eq!(Solid::mesh_chunks([&cube], Tessellation::default()).expect("later presentation").faces.len(), 6);
}

#[test]
fn cancellation_after_source_extraction_stops_rust_refinement() {
	let solids = (0..32).map(|index| Solid::sphere(10.0).translate(DVec3::X * f64::from(index) * 24.0)).collect::<Vec<_>>();
	let cancellation = CancellationToken::new();
	let monitor_token = cancellation.clone();
	let monitor = thread::spawn(move || {
		let deadline = Instant::now() + Duration::from_secs(10);
		loop {
			let progress = monitor_token.progress();
			if progress >= 1.0 {
				return false;
			}
			if progress >= 0.25 {
				monitor_token.cancel();
				return true;
			}
			if Instant::now() >= deadline {
				monitor_token.cancel();
				return false;
			}
			thread::yield_now();
		}
	});
	let options = Tessellation { deflection_linear: 0.04, deflection_angular: 0.08, relative_linear: false, include_edges: false, parallel: false };
	let result = Solid::mesh_chunks_cancelable(solids.iter(), options, &cancellation);

	assert!(monitor.join().expect("cancellation monitor"), "the fixture completed before entering Rust face refinement");
	assert!(matches!(result, Err(Error::Cancelled)));
	let later = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	assert!(!Solid::mesh_chunks([&later], Tessellation::default()).expect("later tessellation").faces.is_empty());
}
