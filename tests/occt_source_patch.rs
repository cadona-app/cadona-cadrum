#[path = "../build_support/occt_planarity.rs"]
mod occt_planarity;

const EXTRUSION_CHECK: &str = "        Dn /= norm;
        double angmax = Tol / (Vmax - Vmin);
        gp_Dir D(Dn);
        Essai = (D.IsNormal(AS.Direction(), angmax));
      }
      if (Essai)
      {
        gp_Ax3 axe(P, Dn, Du);
        myPlan.SetPosition(axe);
        myPlan.SetLocation(P);
        occ::handle<Geom_Curve> C;
        C      = S->VIso((Vmin + Vmax) / 2);
        IsPlan = Controle(C, myPlan, Tol);
      }";

#[test]
fn patch_is_idempotent_and_preserves_linear_validation() {
	let patched = occt_planarity::patch_extrusion_planarity(EXTRUSION_CHECK).unwrap();
	assert!(patched.starts_with("        Dn /= norm;"));
	assert!(patched.ends_with(EXTRUSION_CHECK.split_once("      if (Essai)").unwrap().1));
	assert_eq!(occt_planarity::patch_extrusion_planarity(&patched).unwrap(), patched);
}

#[test]
fn changed_or_duplicate_source_requires_review() {
	for source in [String::new(), EXTRUSION_CHECK.replace("angmax", "angular_tolerance"), EXTRUSION_CHECK.repeat(2)] {
		assert!(occt_planarity::patch_extrusion_planarity(&source).is_err());
	}
}
