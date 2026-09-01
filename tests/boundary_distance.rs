use cadrum::{DVec3, Solid};

#[test]
fn nested_solid_boundaries_have_positive_distance() {
	let outer = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let inner = Solid::cube(DVec3::splat(2.0), DVec3::splat(8.0));
	let distance = outer.boundary_distance(&inner).expect("boundary distance");

	assert!((distance.distance - 2.0).abs() < 1.0e-9);
}

#[test]
fn crossing_solid_boundaries_have_zero_distance() {
	let outer = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let crossing = Solid::cube(DVec3::new(8.0, 2.0, 2.0), DVec3::new(12.0, 8.0, 8.0));
	let distance = outer.boundary_distance(&crossing).expect("boundary distance");

	assert!(distance.distance <= 1.0e-9);
}
