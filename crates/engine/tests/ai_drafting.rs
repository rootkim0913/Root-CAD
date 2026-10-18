use cadcraft_engine::Session;
use serde_json::json;
use std::sync::Arc;

#[test]
fn plans_preview_rollback_and_undo_as_one_edit() {
    let mut s = Session::new();
    let original = s.state().unwrap().clone();
    let steps = json!([{"command":"circle","params":{"center":[0,0],"radius":5}}, {"command":"line","params":{"points":[[0,0],[10,0]]}}]);
    s.execute("draft", &json!({"steps":steps,"dryRun":true})).unwrap();
    assert!(Arc::ptr_eq(&s.state().unwrap().doc, &original.doc));
    assert_eq!(s.state().unwrap().revision, original.revision);
    assert!(s.state().unwrap().undo.is_empty());
    let bad = json!({"steps":[{"command":"circle","params":{"center":[0,0],"radius":5}}, {"command":"circle","params":{"radius":-1}}]});
    assert!(s.execute("draft", &bad).is_err());
    assert!(Arc::ptr_eq(&s.state().unwrap().doc, &original.doc));
    assert!(s.state().unwrap().undo.is_empty());
    s.execute("draft", &json!({"steps":steps,"expectedRevision":original.revision})).unwrap();
    assert_eq!(s.doc().unwrap().entity_count(), 2);
    assert_eq!(s.state().unwrap().undo.len(), 1);
    assert!(s.execute("draft", &json!({"steps":steps,"expectedRevision":original.revision})).is_err());
    s.execute("undo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().entity_count(), 0);
    s.execute("redo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().entity_count(), 2);
}

#[test]
fn plans_cannot_save_or_recurse_and_preserve_redo_on_failure() {
    let mut s = Session::new();
    s.execute("circle", &json!({"center":[0,0],"radius":3})).unwrap();
    s.execute("undo", &json!({})).unwrap();
    for command in ["saveas", "new", "open", "draft", "script", "undo", "setvar"] {
        assert!(s.execute("draft", &json!({"steps":[{"command":command}]})).is_err());
    }
    assert_eq!(s.state().unwrap().redo.len(), 1);
    assert!(
        s.execute("draft", &json!({"steps":[{"command":"circle","params":{"center":[10,0],"radius":1}}, {"command":"circle","params":{}}]})).is_err()
    );
    assert_eq!(s.state().unwrap().redo.len(), 1);
    s.execute("redo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().entity_count(), 1);
}

#[test]
fn plans_leave_an_interactive_command_running_when_refused() {
    let mut s = Session::new();
    s.start("line").unwrap();
    assert!(s.execute("draft", &json!({"steps":[{"command":"circle","params":{"center":[0,0],"radius":2}}]})).is_err());
    assert_eq!(s.running.as_ref().unwrap().id, "line");
    assert_eq!(s.doc().unwrap().entity_count(), 0);
}

#[test]
fn grouped_moves_match_individual_edits_with_associative_dimensions() {
    let mut grouped = Session::new();
    let mut individual = Session::new();
    for s in [&mut grouped, &mut individual] {
        s.execute("setvar", &json!({"name":"DIMASSOC","value":2})).unwrap();
        let line = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap();
        let handle = &line["handles"][0];
        s.execute("dimlinear", &json!({"object":handle,"at":[5,5]})).unwrap();
        s.set_selection(vec![cadcraft_engine::doc::Handle::parse_hex(handle.as_str().unwrap()).unwrap()]);
    }
    let params = json!({"delta":[3,2]});
    grouped.execute("draft", &json!({"steps":[{"command":"move","params":params},{"command":"move","params":params}]})).unwrap();
    individual.execute("move", &params).unwrap();
    individual.execute("move", &params).unwrap();
    let geometry = |s: &Session| s.doc().unwrap().model.iter().map(|e| serde_json::to_value(&e.kind).unwrap()).collect::<Vec<_>>();
    assert_eq!(geometry(&grouped), geometry(&individual));
}

#[test]
fn flange_matches_video_without_inventing_thickness_and_rejects_overlap() {
    let mut s = Session::new();
    let p = json!({"outerDiameter":100,"boltCircle":85,"holeDiameter":10,"holes":12,"pipeDiameter":70,"pipeLength":500});
    let r = s.execute("flange", &p).unwrap();
    assert_eq!(r["handles"].as_array().unwrap().len(), 16);
    assert_eq!(r["warnings"].as_array().unwrap().len(), 2);
    s.execute("undo", &json!({})).unwrap();
    let mut side = p.clone();
    side["flangeThickness"] = json!(12);
    assert_eq!(s.execute("flange", &side).unwrap()["handles"].as_array().unwrap().len(), 18);
    let before = s.doc().unwrap().entity_count();
    for (key, value) in [("holes", json!(361)), ("holeDiameter", json!(30)), ("pipeDiameter", json!(90)), ("outerDiameter", json!(-1))] {
        let mut bad = p.clone();
        bad[key] = value;
        assert!(s.execute("flange", &bad).is_err());
        assert_eq!(s.doc().unwrap().entity_count(), before);
    }
}
