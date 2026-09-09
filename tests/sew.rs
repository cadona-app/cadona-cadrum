//! Integration tests for `Solid::sew`.
//!
//! Covers:
//! - 分解した box の 6 face を縫合 → 元の体積が回復する
//! - 開いた shell (face 不足) はエラー
//! - 空入力はエラー

use cadrum::{Error, Face, Solid};
use glam::DVec3;

// ==================== (1) box の 6 face を縫合して体積回復 ====================

#[test]
fn test_sew_01_box_faces_recover_volume() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::new(2.0, 3.0, 4.0));
	let expected = cube.volume();

	let sewn = Solid::sew(cube.iter_face(), 1.0e-6).expect("sewing 6 box faces should succeed");

	let rel = (sewn.volume() - expected).abs() / expected;
	assert!(rel < 1.0e-9, "sewn volume {:.9} vs original {:.9} (relative error {:.3e})", sewn.volume(), expected, rel);

	let bbox = sewn.bounding_box();
	assert!((bbox[0] - DVec3::ZERO).length() < 1.0e-6 && (bbox[1] - DVec3::new(2.0, 3.0, 4.0)).length() < 1.0e-6, "sewn bounding box {:?} must match the original box", bbox);
	assert!(sewn.contains(DVec3::new(1.0, 1.5, 2.0)));
}

// ==================== (2) face 不足 (開いた shell) はエラー ====================

#[test]
fn test_sew_02_open_shell_returns_sew_failed() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	let five: Vec<&Face> = cube.iter_face().take(5).collect();

	let err = Solid::sew(five, 1.0e-6).err().expect("5 of 6 faces must not sew into a solid");
	match err {
		Error::SewFailed(msg) => assert!(msg.contains("closed shell"), "error message should mention closed shell, got: {}", msg),
		other => panic!("expected Error::SewFailed, got {:?}", other),
	}
}

// ==================== (3) 空入力はエラー ====================

#[test]
fn test_sew_03_empty_input_returns_sew_failed() {
	let none: Vec<&Face> = vec![];
	let err = Solid::sew(none, 1.0e-6).err().expect("empty face set must return Err");
	match err {
		Error::SewFailed(msg) => assert!(msg.contains("no faces"), "error message should mention empty input, got: {}", msg),
		other => panic!("expected Error::SewFailed, got {:?}", other),
	}
}

#[test]
fn sewing_a_narrow_strip_preserves_modified_face_history() {
	use cadrum::{Edge, InputTopology, TopologyKind, TopologyQueryOptions};
	let profile = Edge::polygon(&[DVec3::ZERO, DVec3::new(10.0, 0.0, 0.0), DVec3::new(10.0, 10.0, 0.0), DVec3::new(0.0001, 10.0, 0.0), DVec3::new(0.0, 9.9999, 0.0)]).expect("profile with a tiny corner");
	let source = Solid::extrude(&profile, DVec3::Z * 5.0).expect("extrusion");
	let before = source.topology_snapshot_with_options(TopologyQueryOptions::MEASUREMENT).unwrap();
	let narrow = (0..before.face_ids().len() as u32).find(|face| before.face_facts(*face).unwrap().area.unwrap() < 0.001).unwrap();
	let sewn = source.sew_without_faces(&[narrow], 0.001).expect("collapse narrow strip");
	assert!(sewn.validate().unwrap().valid);
	assert!((sewn.volume() - source.volume()).abs() < 0.01);
	assert_eq!(sewn.iter_face().count(), 6);
	let mesh = Solid::mesh_chunks([&sewn], cadrum::Tessellation::default()).expect("mesh collapsed planar boundary");
	assert_eq!(mesh.faces.len(), 6);
	assert!(mesh.faces.iter().all(|face| !face.indices.is_empty() && face.vertices.iter().all(|point| point.is_finite())));
	let after = sewn.topology_snapshot_with_options(TopologyQueryOptions::MEASUREMENT).expect("query collapsed edges safely");
	assert!(after.edge_ids().len() >= 12);
	let history = sewn.topology_history();
	for face in 0..before.face_ids().len() as u32 {
		let input = InputTopology { operand: 0, kind: TopologyKind::Face, index: face };
		if face == narrow {
			assert!(history.deleted().contains(&input));
		} else {
			assert!(history.relations().iter().any(|relation| relation.source == input && relation.result.kind == TopologyKind::Face));
		}
	}
	assert_eq!(before, source.topology_snapshot_with_options(TopologyQueryOptions::MEASUREMENT).unwrap(), "sewing leaves the source unchanged");
}

#[test]
fn sewing_without_faces_rejects_gaps_and_invalid_arguments() {
	let cube = Solid::cube(DVec3::ZERO, DVec3::splat(10.0));
	assert!(cube.sew_without_faces(&[0], 0.001).is_err());
	assert!(cube.sew_without_faces(&[6], 0.001).is_err());
	for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
		assert!(cube.sew_without_faces(&[], tolerance).is_err());
	}
}
