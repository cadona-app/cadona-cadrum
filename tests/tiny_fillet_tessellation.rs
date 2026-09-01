use cadrum::{DVec3, Solid, Tessellation};

#[test]
fn microscopic_fillet_on_large_body_stays_resource_bounded() {
	let source = Solid::cube(DVec3::ZERO, DVec3::new(100.0, 80.0, 60.0));
	let edge = source.iter_edge().next().expect("box edge");
	let fillet = source.fillet_edges(0.01, [edge]).expect("microscopic fillet");
	let options = Tessellation { deflection_linear: 0.004, deflection_angular: 0.25, relative_linear: true, include_edges: true, parallel: true };

	let mesh = Solid::mesh_chunks([&fillet], options).expect("tessellate microscopic fillet");
	let triangle_count = mesh.faces.iter().map(|face| face.indices.len() / 3).sum::<usize>();

	assert!(triangle_count > 0);
	assert!(triangle_count < 10_000, "a microscopic fillet generated {triangle_count} triangles");
}
