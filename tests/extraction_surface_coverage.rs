#![cfg(feature = "test-support")]

use std::f64::consts::TAU;

use cadrum::{
	occt::test_support::{tessellation_boundary_runs, BoundaryDirection},
	DVec3, Edge, ProfileOrient, Solid, Tessellation,
};

fn options() -> Tessellation {
	Tessellation { deflection_linear: 0.02, deflection_angular: 0.2, relative_linear: true, include_edges: true, parallel: false }
}

fn assert_extracts_boundary_contract(name: &str, solid: &Solid) {
	let validation = solid.validate().unwrap_or_else(|error| panic!("{name}: validate exact fixture: {error:?}"));
	assert!(validation.valid, "{name}: exact fixture is invalid: {validation:?}");
	let runs = tessellation_boundary_runs(solid, options()).unwrap_or_else(|error| panic!("{name}: extract exact rational-surface snapshot: {error:?}"));
	assert!(!runs.is_empty(), "{name}: extraction returned no face-loop boundary provenance");
	for run in runs {
		assert!(!run.sample_ordinals.is_empty(), "{name}: edge occurrence has no canonical sample provenance: {run:?}");
		assert!(
			run.sample_ordinals.windows(2).all(|samples| match run.direction {
				BoundaryDirection::Forward => samples[0] < samples[1],
				BoundaryDirection::Reversed => samples[0] > samples[1],
			}),
			"{name}: edge occurrence sample ordinals do not follow their declared direction: {run:?}"
		);
	}
}

fn periodic_section(z: f64, radial_scale: f64, vertical_scale: f64) -> Edge {
	let points = (0..16)
		.map(|index| {
			let angle = TAU * index as f64 / 16.0;
			DVec3::new(radial_scale * angle.cos(), vertical_scale * angle.sin(), z)
		})
		.collect::<Vec<_>>();
	Edge::bspline(&points, cadrum::BSplineEnd::Periodic).expect("construct periodic loft section")
}

#[test]
fn supported_exact_surface_families_extract_complete_rational_snapshots() {
	let box_solid = Solid::cube(DVec3::splat(-4.0), DVec3::splat(4.0));
	let cylinder = Solid::cylinder(4.0, DVec3::Z * 11.0);
	let cone = Solid::cone(5.0, 1.5, DVec3::Z * 12.0);
	let sphere = Solid::sphere(5.0);
	let torus = Solid::torus(8.0, 2.0, DVec3::Z);

	let extrusion_profile = Edge::bspline(&[DVec3::new(-4.0, -2.0, 0.0), DVec3::new(-1.0, -3.0, 0.0), DVec3::new(3.5, -1.0, 0.0), DVec3::new(4.0, 2.0, 0.0), DVec3::new(0.0, 3.0, 0.0), DVec3::new(-4.0, -2.0, 0.0)], cadrum::BSplineEnd::NotAKnot).expect("construct B-spline extrusion profile");
	let extrusion = Solid::extrude([&extrusion_profile], DVec3::Z * 7.0).expect("extrude B-spline profile");

	let sweep_profile = Edge::ellipse(3.0, 1.5, DVec3::X, DVec3::Z).expect("construct sweep profile");
	let sweep_spine = Edge::line(DVec3::ZERO, DVec3::Z * 14.0).expect("construct sweep spine");
	let sweep = Solid::sweep([&sweep_profile], [&sweep_spine], ProfileOrient::Fixed).expect("sweep elliptical profile");

	let loft_sections = [periodic_section(0.0, 4.0, 2.0), periodic_section(8.0, 3.2, 2.6), periodic_section(17.0, 2.1, 1.4)];
	let loft = Solid::loft(loft_sections.iter().map(std::iter::once), false).expect("loft periodic B-spline sections");
	let located_loft = loft.located(DVec3::new(0.3, 0.7, 0.2), 0.61, DVec3::new(1.0e6, -2.0e6, 3.0e6));

	let fillet_source = Solid::cube(DVec3::splat(-5.0), DVec3::splat(5.0));
	let fillet_edge = fillet_source.iter_edge().next().expect("box edge");
	let fillet = fillet_source.fillet_edges(1.25, [fillet_edge]).expect("fillet box edge");

	let offset = Solid::sphere(5.0).offset_surface(0.75, 1.0e-6).expect("offset sphere surface");

	for (name, solid) in [("plane", &box_solid), ("cylinder", &cylinder), ("cone", &cone), ("sphere", &sphere), ("torus", &torus), ("B-spline extrusion", &extrusion), ("elliptical sweep", &sweep), ("B-spline loft", &loft), ("located B-spline loft", &located_loft), ("fillet blend", &fillet), ("offset surface", &offset)] {
		assert_extracts_boundary_contract(name, solid);
	}
}

#[test]
fn chart_mapping_accounts_for_the_canonical_edge_discrepancy() {
	let errors = cadrum::occt::test_support::tessellation_chart_mapping_errors();
	let [allowed, original, normalized_conversion, normalized_canonical, mapped_conversion, mapped_canonical] = errors.as_slice() else {
		panic!("map the cylinder boundary back to its exact surface");
	};
	assert!(*original > 0.0 && original < allowed);
	assert!(normalized_conversion < allowed, "the shortcut fits the surface alone");
	assert!(normalized_canonical > allowed, "the shortcut must exceed the combined budget");
	assert!(mapped_conversion <= allowed, "projected chart must fit the surface budget");
	assert!(mapped_canonical <= allowed, "projected chart must fit the canonical edge budget");
}
