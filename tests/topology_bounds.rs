use cadrum::{DVec3, ResultTopology, Solid, TopologyKind};

#[test]
fn exact_subshape_bounds_include_curved_extrema_and_reject_missing_entities() {
	let sphere = Solid::sphere(5.0);
	let bounds = sphere.topology_bounds(ResultTopology { kind: TopologyKind::Face, index: 0 }).unwrap();
	assert!((bounds[0] - DVec3::splat(-5.0)).length() < 1e-6);
	assert!((bounds[1] - DVec3::splat(5.0)).length() < 1e-6);
	assert!(sphere.topology_bounds(ResultTopology { kind: TopologyKind::Face, index: 99 }).is_err());
}
