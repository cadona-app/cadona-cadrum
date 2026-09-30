//! Validate `area` / `center` / `inertia` queries against analytical solutions.
//!
//! OCCT computes properties with uniform density ρ = 1; the inertia tensor is
//! returned about the world origin (not the center of mass).

use cadrum::{CancellationToken, Edge, Solid};
use glam::DVec3;

const EPS: f64 = 1e-6;

#[test]
fn complete_elliptical_prisms_and_rings_have_analytic_mass_properties() {
	let height = 6.;
	for offset in [DVec3::ZERO, DVec3::new(12., -8., 0.)] {
		for hole in [false, true] {
			let edges = [(10., 4.), (5., 2.)].into_iter().take(if hole { 2 } else { 1 }).map(|(a, b)| Edge::ellipse(a, b, DVec3::X, DVec3::Z).unwrap().translate(offset)).collect::<Vec<_>>();
			let progress = CancellationToken::new();
			let arrangement = Edge::planar_regions(&edges, 1.0e-7, 4096, &progress).unwrap();
			let region = arrangement.regions.iter().find(|region| region.wires.len() == if hole { 2 } else { 1 }).unwrap();
			let wires = region.wires.iter().map(|wire| wire.iter().map(|span| edges[span.source as usize].profile_span(span.first, span.last, span.reversed, 1.0e-7).unwrap()).collect::<Vec<_>>()).collect::<Vec<_>>();
			let solid = Solid::extrude_wires_cancelable(wires.iter().map(|wire| wire.iter()), DVec3::Z * height, &progress).unwrap();
			assert!(solid.validate().unwrap().valid);
			let properties = |a: f64, b: f64| {
				let mass = std::f64::consts::PI * a * b * height;
				(mass, DVec3::new(mass * (b * b / 4. + height * height / 12.), mass * (a * a / 4. + height * height / 12.), mass * (a * a + b * b) / 4.))
			};
			let (outer_mass, outer_inertia) = properties(10., 4.);
			let (inner_mass, inner_inertia) = if hole { properties(5., 2.) } else { (0., DVec3::ZERO) };
			let mass = outer_mass - inner_mass;
			let center = offset + DVec3::Z * height / 2.;
			let centered = outer_inertia - inner_inertia;
			let expected = centered + mass * (DVec3::splat(center.length_squared()) - center * center);
			assert!((solid.volume() - mass).abs() < 1.0e-7, "{} != {mass}", solid.volume());
			assert!((solid.center() - center).length() < 1.0e-8);
			let inertia = solid.inertia();
			assert!((DVec3::new(inertia.x_axis.x, inertia.y_axis.y, inertia.z_axis.z) - expected).abs().max_element() < 1.0e-5);
			assert!((inertia.y_axis.x + mass * center.x * center.y).abs() < 1.0e-5);
			assert!((inertia.z_axis.x + mass * center.x * center.z).abs() < 1.0e-5);
			assert!((inertia.z_axis.y + mass * center.y * center.z).abs() < 1.0e-5);
		}
	}
}

/// Cube of side `a` with corner at the world origin, ρ = 1.
/// Analytical values referenced throughout:
///   volume = a³
///   center = (a/2, a/2, a/2)
///   area   = 6 a²
///   I_xx = I_yy = I_zz = ∫(y² + z²) dV over [0,a]³ = 2 a⁵ / 3
///   |I_xy| = |I_yz| = |I_zx| = a⁵ / 4  (sign depends on tensor sign convention)
#[test]
fn test_cube_mass_properties_match_analytical() {
	let a = 10.0_f64;
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(a));

	assert!((cube.volume() - a.powi(3)).abs() < EPS);
	assert!((cube.area() - 6.0 * a.powi(2)).abs() < EPS);
	assert!((cube.center() - DVec3::splat(a / 2.0)).length() < EPS);

	let i = cube.inertia();
	let expected_diag = 2.0 * a.powi(5) / 3.0;
	let expected_off = a.powi(5) / 4.0;
	// Diagonals are positive and equal for a symmetric cube.
	assert!((i.col(0).x - expected_diag).abs() < 1e-3, "I_xx = {}, expected {expected_diag}", i.col(0).x);
	assert!((i.col(1).y - expected_diag).abs() < 1e-3, "I_yy = {}, expected {expected_diag}", i.col(1).y);
	assert!((i.col(2).z - expected_diag).abs() < 1e-3, "I_zz = {}, expected {expected_diag}", i.col(2).z);
	// Off-diagonals: magnitude only (sign depends on OCCT convention).
	assert!((i.col(1).x.abs() - expected_off).abs() < 1e-3, "|I_xy| = {}, expected {expected_off}", i.col(1).x.abs());
	assert!((i.col(2).y.abs() - expected_off).abs() < 1e-3, "|I_yz| = {}, expected {expected_off}", i.col(2).y.abs());
	assert!((i.col(2).x.abs() - expected_off).abs() < 1e-3, "|I_zx| = {}, expected {expected_off}", i.col(2).x.abs());
}

/// Sphere of radius `r` centered at origin.
///   volume = (4/3) π r³
///   area   = 4 π r²
///   center = 0
///   I_diag = (2/5) m r² = (8/15) π r⁵  (sphere centered at origin → COM = origin)
#[test]
fn test_sphere_mass_properties_match_analytical() {
	let r = 5.0_f64;
	let sphere = Solid::sphere(r);

	let pi = std::f64::consts::PI;
	assert!((sphere.volume() - 4.0 / 3.0 * pi * r.powi(3)).abs() < 1e-2);
	assert!((sphere.area() - 4.0 * pi * r.powi(2)).abs() < 1e-2);
	assert!(sphere.center().length() < 1e-3, "sphere COM should be at origin, got {:?}", sphere.center());

	let i = sphere.inertia();
	let expected_diag = 8.0 / 15.0 * pi * r.powi(5);
	assert!((i.col(0).x - expected_diag).abs() < 1e-1, "I_xx = {}, expected {expected_diag}", i.col(0).x);
	assert!((i.col(1).y - expected_diag).abs() < 1e-1);
	assert!((i.col(2).z - expected_diag).abs() < 1e-1);
	// Off-diagonals should be ~0 for a sphere at origin.
	assert!(i.col(1).x.abs() < 1e-3);
	assert!(i.col(2).x.abs() < 1e-3);
	assert!(i.col(2).y.abs() < 1e-3);
}
