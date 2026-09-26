#![cfg(feature = "test-support")]

use std::collections::BTreeSet;

use cadrum::{
	occt::test_support::{tessellation_boundary_runs, tessellation_rejects_corrupted_source_contracts, tessellation_rejects_invalid_boundary_direction, tessellation_retains_boundary_junction_occurrences, BoundaryDirection, BoundaryRun},
	DVec3, Edge, ProfileOrient, Solid, Tessellation, TopologyQueryOptions,
};

fn options() -> Tessellation {
	Tessellation { deflection_linear: 0.05, deflection_angular: 0.2, relative_linear: false, include_edges: true, parallel: false }
}

fn assert_run_is_ordered(run: &BoundaryRun) {
	assert!(!run.sample_ordinals.is_empty(), "boundary occurrence has no canonical samples: {run:?}");
	assert!(
		run.sample_ordinals.windows(2).all(|samples| match run.direction {
			BoundaryDirection::Forward => samples[0] < samples[1],
			BoundaryDirection::Reversed => samples[0] > samples[1],
		}),
		"boundary occurrence does not follow its declared direction: {run:?}"
	);
}

#[test]
fn decoder_rejects_invalid_occurrence_direction_metadata() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	assert!(tessellation_rejects_invalid_boundary_direction(&cube, options()).expect("exercise corrupted boundary metadata"));
}

#[test]
fn boundary_junctions_retain_every_distinct_occurrence() {
	assert!(tessellation_retains_boundary_junction_occurrences());
}

#[test]
fn decoder_rejects_corrupted_parallel_arrays_and_cross_references() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	assert!(tessellation_rejects_corrupted_source_contracts(&cube, options()).expect("exercise corrupted exact-snapshot contracts"));
}

#[test]
fn ordinary_shared_edge_retains_one_oppositely_directed_occurrence_per_face() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let chunks = Solid::mesh_chunks([&cube], options()).expect("mesh ordinary shared-edge contract");
	assert!(!chunks.faces.is_empty());
	let topology = cube.topology_snapshot().expect("snapshot cube topology");
	let edge_index = 0;
	let expected_faces = topology.edge_faces(edge_index).expect("query shared-edge incidence").iter().copied().collect::<BTreeSet<_>>();
	assert_eq!(expected_faces.len(), 2, "cube edge must be ordinarily manifold");

	let all_runs = tessellation_boundary_runs(&cube, options()).expect("extract cube boundary provenance");
	let edge_runs = all_runs.iter().filter(|run| run.edge_index == edge_index).collect::<Vec<_>>();
	assert_eq!(edge_runs.len(), 2, "ordinary shared edge must occur exactly once in each incident face: {edge_runs:#?}");
	assert_eq!(edge_runs.iter().map(|run| run.face_index).collect::<BTreeSet<_>>(), expected_faces);
	assert_ne!(edge_runs[0].direction, edge_runs[1].direction, "closed-shell shared-edge occurrences must have opposite directions");
	for run in edge_runs {
		assert_run_is_ordered(run);
	}
}

#[test]
fn periodic_self_seam_retains_two_distinct_occurrences_on_one_face_loop() {
	let cylinder = Solid::cylinder(8.0, DVec3::Z * 20.0);
	let chunks = Solid::mesh_chunks([&cylinder], options()).expect("mesh periodic self-seam contract");
	assert!(!chunks.faces.is_empty());
	let topology = cylinder.topology_snapshot_with_options(TopologyQueryOptions::SEMANTIC_IDENTITY).expect("snapshot cylinder topology");
	let seam_edge = (0..topology.edge_ids().len() as u32).find(|edge| topology.edge_facts(*edge).is_some_and(|facts| facts.seam)).expect("primitive cylinder must retain an explicit periodic seam");
	let seam_face = *topology.edge_faces(seam_edge).expect("query seam incidence").first().expect("seam must have an incident face");

	let all_runs = tessellation_boundary_runs(&cylinder, options()).expect("extract cylinder boundary provenance");
	let seam_runs = all_runs.iter().filter(|run| run.edge_index == seam_edge).collect::<Vec<_>>();
	assert_eq!(seam_runs.len(), 2, "a periodic self-seam must remain two edge occurrences: {seam_runs:#?}");
	assert!(seam_runs.iter().all(|run| run.face_index == seam_face));
	assert_eq!(seam_runs[0].loop_index, seam_runs[1].loop_index, "self-seam occurrences must belong to the same face loop");
	assert_ne!(seam_runs[0].edge_occurrence_index, seam_runs[1].edge_occurrence_index, "self-seam occurrences must not collapse to one run identity");
	assert_ne!(seam_runs[0].direction, seam_runs[1].direction, "self-seam occurrences must traverse the canonical edge in opposite directions");
	for run in seam_runs {
		assert_run_is_ordered(run);
	}
}

#[test]
fn collapsed_cone_pole_contract_meshes_without_a_phantom_boundary_segment() {
	let cone = Solid::cone(8.0, 0.0, DVec3::Z * 20.0);
	let cone_options = Tessellation { deflection_angular: 0.15, parallel: true, ..options() };
	let chunks = Solid::mesh_chunks([&cone], cone_options).expect("mesh collapsed pole contract");
	assert!(!chunks.faces.is_empty());
	assert!(chunks.faces.iter().all(|face| !face.indices.is_empty()));
}

#[test]
fn small_relative_elliptical_sweep_extracts_complete_boundary_provenance() {
	let scale = 1.0e-3;
	let profile = Edge::ellipse(3.0 * scale, scale, DVec3::X, DVec3::Z).expect("construct small elliptical profile");
	let spine = Edge::line(DVec3::ZERO, DVec3::Z * (12.0 * scale)).expect("construct small sweep spine");
	let sweep = Solid::sweep([&profile], [&spine], ProfileOrient::Fixed).expect("construct small elliptical sweep");
	let options = Tessellation { deflection_linear: 0.004, deflection_angular: 0.15, relative_linear: true, include_edges: true, parallel: false };
	let runs = tessellation_boundary_runs(&sweep, options).expect("extract complete small-sweep boundary provenance");
	assert!(!runs.is_empty());
	assert!(runs.iter().all(|run| !run.sample_ordinals.is_empty()));
}
