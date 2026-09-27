#include <Geom_BSplineCurve.hxx>
#include <Geom_Circle.hxx>
#include <Geom_Line.hxx>
#include <Geom_RectangularTrimmedSurface.hxx>
#include <Geom_SurfaceOfLinearExtrusion.hxx>
#include <GeomLib_IsPlanarSurface.hxx>
#include <NCollection_Array1.hxx>
#include <gp_Ax2.hxx>
#include <gp_Pln.hxx>
#include <cmath>
#include <iostream>

int main()
{
    int failures = 0;
    int checks = 0;
    const double tolerance = 1.e-7;
    const gp_Pnt origin(1.3, 2.7, 3.2);
    const gp_Vec along(0.521488830096102, 0.5214888300961019, 0.6753508718954896);
    auto check = [&](const Handle(Geom_Surface)& surface, bool expected, int index) {
        ++checks;
        const GeomLib_IsPlanarSurface result(surface, tolerance);
        if (result.IsPlanar() != expected) {
            std::cerr << "case " << index << ": expected planar=" << expected << '\n';
            ++failures;
        }
        if (expected && result.IsPlanar()) {
            for (int u = 0; u <= 4; ++u) {
                for (double v : {-10.0, 0.0, 10.0}) {
                    if (result.Plan().Distance(surface->Value(u / 4.0, v)) >= tolerance) {
                        std::cerr << "case " << index << ": point outside returned plane\n";
                        ++failures;
                    }
                }
            }
        }
    };

    for (int index = 0; index < 100; ++index) {
        const gp_Vec across(std::sin(index + 0.1), std::cos(index + 0.2), 0.3);
        const gp_Vec normal = across.Crossed(along).Normalized();
        for (bool nonplanar : {false, true}) {
            NCollection_Array1<gp_Pnt> poles(1, 4);
            poles(1) = origin;
            poles(2) = origin.Translated(across * 0.3 + along * 0.2
                + normal * (nonplanar ? 1.e-3 : 0.0));
            poles(3) = origin.Translated(across * 0.7 - along * 0.1);
            poles(4) = origin.Translated(across + along * 0.4);
            NCollection_Array1<double> knots(1, 2);
            knots(1) = 0.0;
            knots(2) = 1.0;
            NCollection_Array1<int> multiplicities(1, 2);
            multiplicities(1) = multiplicities(2) = 4;
            Handle(Geom_Curve) curve = new Geom_BSplineCurve(poles, knots, multiplicities, 3);
            Handle(Geom_Surface) surface = new Geom_SurfaceOfLinearExtrusion(curve, gp_Dir(along));
            check(surface, !nonplanar, index);
            Handle(Geom_Surface) bounded = new Geom_RectangularTrimmedSurface(surface, 0.0, 1.0, -10.0, 10.0);
            check(bounded, !nonplanar, index);
        }
    }

    Handle(Geom_Curve) circle = new Geom_Circle(gp_Ax2(origin, gp_Dir(along)), 2.0);
    check(new Geom_SurfaceOfLinearExtrusion(circle, gp_Dir(along)), false, 100);
    Handle(Geom_Curve) line = new Geom_Line(origin, gp_Dir(1.0, 0.0, 0.0));
    check(new Geom_SurfaceOfLinearExtrusion(line, gp_Dir(1.0, 0.0, 0.0)), false, 101);
    std::cout << checks << " planarity checks, " << failures << " failures\n";
    return failures == 0 ? 0 : 1;
}
