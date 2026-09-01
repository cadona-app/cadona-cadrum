use std::{
	sync::mpsc,
	thread,
	time::{Duration, Instant},
};

use cadrum::{CancellationToken, DVec3, Error, Solid, Tessellation};

fn structured_options(parallel: bool) -> Tessellation {
	Tessellation { deflection_linear: 0.01, deflection_angular: 0.02, relative_linear: true, include_edges: false, parallel }
}

fn progress_options(parallel: bool) -> Tessellation {
	Tessellation { deflection_linear: 0.04, deflection_angular: 0.08, relative_linear: true, include_edges: false, parallel }
}

#[test]
fn expensive_single_structured_face_cancels_after_rust_meshing_starts() {
	let sphere = Solid::sphere(30.0);
	let cancellation = CancellationToken::new();
	let monitor_token = cancellation.clone();
	let (ready_sender, ready_receiver) = mpsc::channel();
	let monitor = thread::spawn(move || {
		ready_sender.send(()).expect("signal cancellation monitor readiness");
		let deadline = Instant::now() + Duration::from_secs(10);
		loop {
			let progress = monitor_token.progress();
			if progress > 0.25 && progress < 1.0 {
				// Let the single face advance beyond dispatch so cancellation has to
				// be observed by its structured construction or validation loops.
				thread::sleep(Duration::from_millis(10));
				if monitor_token.progress() >= 1.0 {
					return None;
				}
				let cancelled_at = Instant::now();
				monitor_token.cancel();
				return Some(cancelled_at);
			}
			if progress >= 1.0 || Instant::now() >= deadline {
				monitor_token.cancel();
				return None;
			}
			thread::yield_now();
		}
	});
	ready_receiver.recv().expect("cancellation monitor starts");

	let result = Solid::mesh_chunks_cancelable([&sphere], structured_options(false), &cancellation);
	let cancelled_at = monitor.join().expect("join cancellation monitor").expect("the structured face must enter Rust meshing before it completes");

	assert!(matches!(result, Err(Error::Cancelled)), "single-face tessellation published a result after cancellation: {result:?}");
	assert!(cancelled_at.elapsed() <= Duration::from_secs(2), "single structured face took {:?} to observe cancellation", cancelled_at.elapsed());
}

#[test]
fn rust_face_progress_is_monotonic_and_completes_for_serial_and_parallel_requests() {
	for parallel in [false, true] {
		let solids = (0..2).map(|index| Solid::sphere(20.0).translate(DVec3::X * f64::from(index) * 45.0)).collect::<Vec<_>>();
		let progress = CancellationToken::new();
		let monitor_token = progress.clone();
		let (ready_sender, ready_receiver) = mpsc::channel();
		let monitor = thread::spawn(move || {
			ready_sender.send(()).expect("signal progress monitor readiness");
			let deadline = Instant::now() + Duration::from_secs(20);
			let mut samples = Vec::new();
			let mut previous_bits = None;
			loop {
				let sample = monitor_token.progress();
				if previous_bits != Some(sample.to_bits()) {
					samples.push(sample);
					previous_bits = Some(sample.to_bits());
				}
				if sample >= 1.0 {
					return samples;
				}
				assert!(Instant::now() < deadline, "progress monitor timed out at {sample}");
				thread::yield_now();
			}
		});
		ready_receiver.recv().expect("progress monitor starts");

		Solid::mesh_chunks_cancelable(solids.iter(), progress_options(parallel), &progress).expect("structured tessellation completes");
		let samples = monitor.join().expect("join progress monitor");

		assert_eq!(progress.progress(), 1.0, "completed request must publish atomic completion");
		assert!(samples.windows(2).all(|pair| pair[0] <= pair[1]), "{parallel:?} progress regressed: {samples:?}");
		assert!(samples.iter().any(|sample| *sample > 0.25 && *sample < 1.0), "{parallel:?} request never exposed Rust face progress: {samples:?}");
	}
}
