use std::{
	thread,
	time::{Duration, Instant},
};

use cadrum::{CancellationToken, DVec3, Error, Solid, Tessellation};

#[test]
fn parallel_tessellation_observes_cancellation_promptly_and_recovers() {
	let solids = (0..32).map(|index| Solid::sphere(10.0).translate(DVec3::X * f64::from(index) * 24.0)).collect::<Vec<_>>();
	let cancellation = CancellationToken::new();
	let monitor_token = cancellation.clone();
	let monitor = thread::spawn(move || {
		let deadline = Instant::now() + Duration::from_secs(10);
		loop {
			if monitor_token.progress() >= 1.0 {
				return None;
			}
			if monitor_token.progress() >= 0.25 {
				monitor_token.cancel();
				return Some(Instant::now());
			}
			if Instant::now() >= deadline {
				monitor_token.cancel();
				return None;
			}
			thread::yield_now();
		}
	});
	let options = Tessellation { deflection_linear: 0.04, deflection_angular: 0.08, relative_linear: false, include_edges: false, parallel: true };
	let result = Solid::mesh_chunks_cancelable(solids.iter(), options, &cancellation);
	let cancelled_at = monitor.join().expect("cancellation monitor").expect("the fixture must remain active long enough to request cancellation");

	assert!(matches!(result, Err(Error::Cancelled)), "parallel tessellation published a result after cancellation: {result:?}");
	assert!(cancelled_at.elapsed() <= Duration::from_secs(2), "parallel tessellation took {:?} to observe cancellation", cancelled_at.elapsed());

	let later = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	assert!(!Solid::mesh_chunks([&later], Tessellation::default()).expect("later tessellation remains usable").faces.is_empty());
}
