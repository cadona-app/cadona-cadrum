use cadrum::{CancellationToken, DVec3, Edge, Error, Solid, TopologyKind};
use std::f64::consts::{PI, TAU};

#[test]
fn revolved_ring_retains_exact_volume_and_edge_history() {
	let outer = Edge::circle(2., DVec3::Z).unwrap().translate(DVec3::X * 5.);
	let inner = Edge::circle(1., -DVec3::Z).unwrap().translate(DVec3::X * 5.);
	for angle in [TAU, -TAU, 1., -1.] {
		let solid = Solid::revolve_wires_cancelable([vec![&outer], vec![&inner]], DVec3::ZERO, DVec3::Y, angle, &CancellationToken::new()).unwrap();
		assert!(solid.validate().unwrap().valid);
		assert!((solid.volume() - 3. * PI * 5. * angle.abs()).abs() < 1.0e-7);
		assert!(solid.topology_history().relations().iter().any(|r| r.result.kind == TopologyKind::Face && r.source.kind == TopologyKind::Edge));
		let faces = solid.topology_history().relations().iter().filter(|r| r.result.kind == TopologyKind::Face && r.source.kind == TopologyKind::Edge).map(|r| r.result.index).collect::<std::collections::BTreeSet<_>>();
		assert_eq!(faces.len(), solid.iter_face().count());
	}
}

#[test]
fn invalid_and_cancelled_revolutions_return_errors() {
	let edge = Edge::circle(1., DVec3::Z).unwrap().translate(DVec3::X * 3.);
	for angle in [0., f64::NAN, f64::INFINITY, TAU + 0.1] {
		assert!(Solid::revolve_wires_cancelable([vec![&edge]], DVec3::ZERO, DVec3::Y, angle, &CancellationToken::new()).is_err());
	}
	let token = CancellationToken::new();
	token.cancel();
	assert!(matches!(Solid::revolve_wires_cancelable([vec![&edge]], DVec3::ZERO, DVec3::Y, 1., &token), Err(Error::Cancelled)));
	assert!(Solid::revolve_wires_cancelable([Vec::<&Edge>::new()], DVec3::ZERO, DVec3::Y, 1., &CancellationToken::new()).is_err());
	let line = Edge::line(DVec3::X, DVec3::new(2., 1., 0.)).unwrap();
	assert!(Solid::revolve_wires_cancelable([vec![&line]], DVec3::ZERO, DVec3::Y, 1., &CancellationToken::new()).is_err());
}
