use cadrum::{occt::CancellationToken, DVec3, Solid, Tessellation};

#[test]
fn prepared_surfaces_outlive_native_shapes_and_mesh_on_another_thread() {
	let options = Tessellation { deflection_linear: 0.05, deflection_angular: 0.5, relative_linear: false, include_edges: true, parallel: true };
	let progress = CancellationToken::new();
	let (prepared, expected) = {
		let source = Solid::cube(DVec3::ZERO, DVec3::splat(7.0));
		(Solid::prepare_mesh([&source], options, &progress).unwrap(), Solid::mesh_chunks([&source], options).unwrap())
	};
	let actual = std::thread::spawn(move || prepared.mesh(&progress)).join().unwrap().unwrap();
	assert_eq!(actual, expected);
	let source = Solid::cube(DVec3::ZERO, DVec3::splat(7.0));
	let progress = CancellationToken::new();
	let prepared = Solid::prepare_mesh([&source], options, &progress).unwrap();
	progress.cancel();
	assert!(matches!(prepared.mesh(&progress), Err(cadrum::Error::Cancelled)));
}
