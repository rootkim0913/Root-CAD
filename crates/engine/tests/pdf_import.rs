use cadcraft_doc::{Common, Drawing, EntityKind, Line, Space};
use cadcraft_engine::{Session, cmd::file::set_pdf_reader};
use cadcraft_geom::Vec3;
use serde_json::json;

#[test]
fn pdf_page_calibration_is_undoable_and_reports_assumptions() {
    set_pdf_reader(|_, page| {
        if page != 2 {
            return Err("missing page".into());
        }
        let mut d = Drawing::new_metric();
        d.add(&Space::Model, Common::default(), EntityKind::Line(Line { a: Vec3::ZERO, b: Vec3::new(50.0, 0.0, 0.0) })).map_err(|e| e.to_string())?;
        d.read_notes.push("Paper lengths only".into());
        Ok(d)
    });
    let mut s = Session::new();
    let params = json!({"data":"","page":2,"reference":{"from":[0,0],"to":[50,0],"length":1000},"at":[10,20]});
    let r = s.execute("pdfimport", &params).unwrap();
    assert_eq!(r["scale"], 20.0);
    assert_eq!(r["warnings"][0], "Paper lengths only");
    let e = s.doc().unwrap().model.iter().next().unwrap();
    let EntityKind::Line(line) = &e.kind else { panic!("expected line") };
    assert_eq!(line.a, Vec3::new(10.0, 20.0, 0.0));
    assert_eq!(line.b, Vec3::new(1010.0, 20.0, 0.0));
    s.execute("undo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().entity_count(), 0);
    for bad in [
        json!({"data":"","page":0}),
        json!({"data":"","page":1}),
        json!({"data":"","page":2,"scale":-1}),
        json!({"data":"","page":2,"reference":{"from":[0,0],"to":[0,0],"length":5}}),
    ] {
        assert!(s.execute("pdfimport", &bad).is_err());
        assert_eq!(s.doc().unwrap().entity_count(), 0);
    }
}
