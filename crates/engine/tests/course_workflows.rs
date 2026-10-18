//! Synthetic lesson workflows; no tutorial drawings or vendor assets are used.
use cadcraft_engine::doc::{EntityKind, Handle};
use cadcraft_engine::geom::Vec2;
use cadcraft_engine::{Input, Session};
use serde_json::{Value, json};

fn metric() -> Session {
    let mut s = Session::empty();
    s.new_drawing(true);
    s
}
fn run(s: &mut Session, id: &str, p: Value) -> Value {
    s.execute(id, &p).unwrap_or_else(|e| panic!("{id}: {e}"))
}
fn first(v: &Value) -> String {
    v["handle"].as_str().or_else(|| v["handles"][0].as_str()).unwrap().to_string()
}

#[test]
fn absolute_relative_and_polar_coordinates_close_exact_geometry() {
    let mut s = metric();
    s.cmdline("line 10,10 @100,0 @50<90 c").unwrap();
    let d = s.doc().unwrap();
    assert_eq!(d.entity_count(), 3);
    let endpoints: Vec<_> =
        d.model.iter().filter_map(|e| if let EntityKind::Line(l) = &e.kind { Some((l.a.xy(), l.b.xy())) } else { None }).collect();
    assert!(endpoints.iter().any(|(a, b)| a.near(Vec2::new(110.0, 10.0), 1e-9) && b.near(Vec2::new(110.0, 60.0), 1e-9)));
}

#[test]
fn drawing_layers_lengthen_copy_mirror_and_properties_work_together() {
    let mut s = metric();
    run(&mut s, "layer.new", json!({"name":"Walls","color":"red","current":true}));
    let line = first(&run(&mut s, "line", json!({"points":[[0,0],[100,0]]})));
    run(&mut s, "lengthen", json!({"handle":line,"total":120,"pick":[100,0]}));
    let h = Handle::parse_hex(&line).unwrap();
    let EntityKind::Line(l) = &s.doc().unwrap().entity(h).unwrap().kind else { panic!("line") };
    assert!((l.a.xy().dist(l.b.xy()) - 120.0).abs() < 1e-8);
    run(&mut s, "copy", json!({"handles":[line],"delta":[0,20]}));
    run(&mut s, "mirror", json!({"handles":[line],"p1":[0,0],"p2":[0,100],"erase":false}));
    let q = run(&mut s, "entities", json!({"layer":"Walls"}));
    assert_eq!(q["count"], 3);
}

#[test]
fn circle_arc_polygon_spline_hatch_and_quick_select_are_available() {
    let mut s = metric();
    run(&mut s, "circle", json!({"p1":[0,0],"p2":[20,0]}));
    run(&mut s, "arc", json!({"p1":[30,0],"p2":[40,10],"p3":[50,0]}));
    let boundary = first(&run(&mut s, "polygon", json!({"sides":6,"center":[80,0],"radius":10})));
    run(&mut s, "hatch", json!({"handles":[boundary],"pattern":"ANSI31","scale":1}));
    run(&mut s, "spline", json!({"fit":[[0,40],[20,60],[40,40]]}));
    let q = run(&mut s, "qselect", json!({"type":"Circle"}));
    assert_eq!(s.selection().len(), 1, "{q}");
    assert_eq!(run(&mut s, "entities", json!({"type":"Hatch"}))["count"], 1);
}

#[test]
fn text_find_replace_handles_korean_text_and_undo() {
    let mut s = metric();
    run(&mut s, "text", json!({"at":[0,0],"height":3.5,"text":"거실 101"}));
    run(&mut s, "mtext", json!({"at":[0,10],"height":3.5,"width":50,"text":"거실\\P방"}));
    assert_eq!(run(&mut s, "find", json!({"find":"거실"}))["total"], 2);
    assert_eq!(run(&mut s, "find", json!({"find":"거실","replace":"침실"}))["total"], 2);
    assert_eq!(run(&mut s, "find", json!({"find":"침실"}))["total"], 2);
    run(&mut s, "undo", json!({}));
    assert_eq!(run(&mut s, "find", json!({"find":"거실"}))["total"], 2);
}

#[test]
fn dimension_style_and_annotated_quick_leader_survive_dxf() {
    let mut s = metric();
    run(&mut s, "dimstyle", json!({"name":"Root-mm","DIMTXT":3.5,"DIMASZ":2.5,"DIMDEC":0,"current":true}));
    run(&mut s, "dimlinear", json!({"p1":[0,0],"p2":[100,0],"at":[50,-15]}));
    s.cmdline("qleader 100,0 120,20 150,20").unwrap();
    s.cmdline("확인 치수").unwrap();
    assert!(s.running.is_none());
    assert_eq!(run(&mut s, "entities", json!({"type":"Leader"}))["count"], 1);
    let bytes = cadcraft_io::write(s.doc().unwrap(), "lesson.dxf").unwrap();
    let d = cadcraft_io::read(&bytes, "lesson.dxf").unwrap();
    assert_eq!(d.entity_count(), 3);
    assert_eq!(d.dim_style("Root-mm").unwrap().text_height, 3.5);
}

#[test]
fn quick_leader_two_points_no_text_cancel_and_single_undo() {
    let mut s = metric();
    for input in ["le", "0,0", "10,10", "", ""] {
        s.cmdline(input).unwrap();
    }
    assert_eq!(s.doc().unwrap().entity_count(), 1);
    run(&mut s, "undo", json!({}));
    assert_eq!(s.doc().unwrap().entity_count(), 0);
    s.cmdline("qleader 0,0 20,20").unwrap();
    s.input(Input::Cancel).unwrap();
    assert_eq!(s.doc().unwrap().entity_count(), 0);
}

#[test]
fn tables_formulas_and_clipboard_keep_editable_data() {
    let mut s = metric();
    let h = first(&run(&mut s, "table", json!({"at":[0,0],"cells":[["2",""],["3",""],["",""]],"rowHeight":8,"colWidth":30})));
    run(&mut s, "table.set", json!({"handle":h,"row":2,"col":0,"text":"=Sum(A1:A2)"}));
    let csv = run(&mut s, "tableexport", json!({"handle":h}));
    assert!(csv["csv"].as_str().unwrap().contains('5'));
    run(&mut s, "copyclip", json!({"handles":[h]}));
    run(&mut s, "pasteclip", json!({"at":[100,0]}));
    assert_eq!(run(&mut s, "entities", json!({"type":"Table"}))["count"], 2);
}

#[test]
fn synthetic_plan_can_export_to_a_real_pdf() {
    let mut s = metric();
    run(&mut s, "rectang", json!({"p1":[0,0],"p2":[6000,4000]}));
    run(&mut s, "text", json!({"at":[200,4200],"height":100,"text":"Root CAD - synthetic floor plan"}));
    let bytes = cadcraft_io::write(s.doc().unwrap(), "lesson.pdf").unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    assert!(!cadcraft_io::read_pdf(&bytes, 1).unwrap().model.is_empty());
}
