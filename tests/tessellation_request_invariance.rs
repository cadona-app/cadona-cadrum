use cadrum::{DVec3, Solid, Tessellation};

fn options(include_edges: bool, parallel: bool) -> Tessellation {
	Tessellation { deflection_linear: 0.02, deflection_angular: 0.2, relative_linear: false, include_edges, parallel }
}

#[test]
fn face_filtering_and_edge_output_do_not_change_surface_connectivity() {
	let cylinder = Solid::cylinder(7.5, DVec3::Z * 19.0);
	let complete = Solid::mesh_chunks([&cylinder], options(true, false)).expect("tessellate the complete cylinder");
	let surfaces_only = Solid::mesh_chunks([&cylinder], options(false, false)).expect("tessellate cylinder surfaces without presentation edges");

	assert_eq!(surfaces_only.faces, complete.faces, "requesting semantic edge polylines must not change the surface mesh");
	assert!(surfaces_only.edges.is_empty(), "a surface-only request unexpectedly returned semantic edge polylines");
	assert!(!complete.edges.is_empty(), "the cylinder fixture must exercise semantic edge output");

	for expected in &complete.faces {
		let selected = cylinder.mesh_face_chunks(&[expected.face_index], options(false, false)).expect("tessellate one selected face");
		assert_eq!(selected.as_slice(), std::slice::from_ref(expected), "face {} changed when requested independently; face-filtered extraction must retain whole-shape canonical edge sampling", expected.face_index);
	}
}

#[test]
fn face_filtered_surface_is_schedule_independent() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(8.0));
	let face_indices = [0, 1, 2, 3, 4, 5];
	let serial = cube.mesh_face_chunks(&face_indices, options(false, false)).expect("serial face tessellation");
	let parallel = cube.mesh_face_chunks(&face_indices, options(false, true)).expect("parallel face tessellation");

	assert_eq!(parallel, serial, "face-parallel scheduling changed a filtered face's vertex or triangle ordering");
}
