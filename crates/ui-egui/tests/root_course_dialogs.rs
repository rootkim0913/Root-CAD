use cadcraft_engine::Session;
use cadcraft_ui_egui::{CadApp, Services, i18n};
use serde_json::json;

fn app() -> CadApp {
    let mut a = CadApp::new(Session::new(), Services::default());
    a.ui.interface_language = i18n::Preference::English;
    a
}

#[test]
fn typed_dimension_style_aliases_and_find_open_dialogs_without_mutating() {
    for command in ["D", "DDIM", "DIMSTYLE", "dimstyle.dimension", "FIND"] {
        let mut a = app();
        let before = a.session.doc().unwrap().entity_count();
        assert!(a.cmdline(command).is_none());
        assert_eq!(a.ui.dialog.as_deref(), Some(if command == "FIND" { "find" } else { "dimstyle" }));
        assert_eq!(a.session.doc().unwrap().entity_count(), before);
        assert!(a.session.running.is_none());
    }
}

#[test]
fn explicit_parameters_keep_programmatic_behavior() {
    let mut a = app();
    a.run("dimstyle", json!({"name":"Course","textHeight":3.5,"decimals":2.0})).unwrap();
    assert!(a.ui.dialog.is_none());
    assert_eq!(a.session.doc().unwrap().dim_style("Course").unwrap().decimals, 2);
    a.run("text", json!({"at":[0,0],"height":3.5,"text":"room"})).unwrap();
    assert_eq!(a.run("find", json!({"find":"room","replace":"hall"})).unwrap()["total"], 1);
    assert!(a.ui.dialog.is_none());
}

#[test]
fn root_cad_name_appears_in_help_menu_and_korean_catalog() {
    let a = app();
    let menu = format!("{:?}", cadcraft_ui_egui::menus::tree(&a));
    assert!(menu.contains("About Root CAD"));
    assert!(!menu.contains("About CADCraft"));
    assert_eq!(i18n::tr("ko", "About Root CAD"), "Root CAD 정보");
}

#[test]
fn new_dialogs_paint_their_editable_controls() {
    fn collect(shape: &egui::Shape, text: &mut String) {
        match shape {
            egui::Shape::Text(s) => {
                text.push_str(s.galley.text());
                text.push('\n');
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, text)),
            _ => {}
        }
    }
    for (command, expected) in [
        ("D", vec!["Dimension Style", "Text height (DIMTXT)", "Precision (DIMDEC)", "New"]),
        ("FIND", vec!["Find and Replace", "Replace with", "Match case", "Find All", "Replace All"]),
    ] {
        let mut a = app();
        a.cmdline(command);
        let ctx = egui::Context::default();
        a.logic(&ctx);
        let mut text = String::new();
        for _ in 0..3 {
            let input =
                egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0))), ..Default::default() };
            let mut output = ctx.run_ui(input, |ui| a.ui(ui));
            output.textures_delta.clear();
            text.clear();
            for shape in output.shapes {
                collect(&shape.shape, &mut text);
            }
        }
        for label in expected {
            assert!(text.contains(label), "{label} missing in {command}: {text}");
        }
    }
}
