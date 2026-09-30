use cadrum::{CancellationToken, DVec3, Edge, Solid};

fn poles() -> [DVec3; 4] {
	[DVec3::ZERO, DVec3::new(0., 4., 0.), DVec3::new(4., 4., 0.), DVec3::new(4., 0., 0.)]
}

#[test]
fn poles_define_the_bezier_without_interpolating_its_control_cage() {
	let edge = Edge::from_bspline_poles(&poles(), 3, &[0., 0., 0., 0., 1., 1., 1., 1.]).unwrap();
	for i in 0..101 {
		let t = i as f64 / 100.;
		let expected = DVec3::new(12. * t * t - 8. * t * t * t, 12. * t * (1. - t), 0.);
		let (projected, _) = edge.project(expected).unwrap();
		assert!((projected - expected).length() < 1.0e-8);
	}
	assert_eq!(edge.start_point(), DVec3::ZERO);
	assert_eq!(edge.end_point(), DVec3::new(4., 0., 0.));
	assert!((edge.project(DVec3::new(0., 4., 0.)).unwrap().0 - DVec3::new(0., 4., 0.)).length() > 1.);
}

#[test]
fn periodic_expansion_keeps_its_active_domain_and_seam() {
	let mut poles = vec![DVec3::new(-3., -3., 0.), DVec3::new(3., -3., 0.), DVec3::new(3., 3., 0.), DVec3::new(-3., 3., 0.)];
	poles.extend_from_within(0..3);
	let knots = (0..11).map(|i| i as f64).collect::<Vec<_>>();
	let edge = Edge::from_bspline_poles(&poles, 3, &knots).unwrap();
	let expected = DVec3::new(2., -2., 0.);
	assert!((edge.start_point() - expected).length() < 1.0e-12);
	assert!((edge.end_point() - expected).length() < 1.0e-12);
	assert!(edge.is_closed());
	assert!((edge.start_tangent() - edge.end_tangent()).length() < 1.0e-12);
	let progress = CancellationToken::new();
	let arrangement = Edge::planar_regions(std::slice::from_ref(&edge), 1.0e-7, 4096, &progress).unwrap();
	assert_eq!(arrangement.regions.len(), 1);
	assert!((arrangement.regions[0].area - 24.4).abs() < 1.0e-8);
	let solid = Solid::extrude_wires_cancelable([std::slice::from_ref(&edge)], DVec3::Z * 3., &progress).unwrap();
	assert!((solid.volume() - 73.2).abs() < 1.0e-7);
	assert!((solid.center() - DVec3::Z * 1.5).length() < 1.0e-8);
	// Green's theorem gives the planar second moments 54742/1155 mm⁴.
	let inertia = solid.inertia();
	let expected = DVec3::new(139288. / 385., 139288. / 385., 109484. / 385.);
	assert!((DVec3::new(inertia.x_axis.x, inertia.y_axis.y, inertia.z_axis.z) - expected).abs().max_element() < 1.0e-5);
	assert!(inertia.y_axis.x.abs() < 1.0e-6);
	assert!(inertia.z_axis.x.abs() < 1.0e-6);
	assert!(inertia.z_axis.y.abs() < 1.0e-6);
}

#[test]
fn malformed_bases_and_nonfinite_poles_fail_without_entering_occt() {
	for degree in [0, 4, 26, u32::MAX] {
		assert!(Edge::from_bspline_poles(&poles(), degree, &[0., 0., 0., 0., 1., 1., 1., 1.]).is_err());
	}
	for knots in [vec![], vec![0.; 8], vec![0., 0., 0., 0., 1., 0., 1., 1.], vec![0., 0., 0., 0., f64::NAN, 1., 1., 1.]] {
		assert!(Edge::from_bspline_poles(&poles(), 3, &knots).is_err());
	}
	assert!(Edge::from_bspline_poles(&[DVec3::NAN, DVec3::ZERO], 1, &[0., 0., 1., 1.]).is_err());
	assert!(Edge::from_bspline_poles(&poles()[..3], 1, &[0., 0., 0., 1., 1.]).is_err());
}
