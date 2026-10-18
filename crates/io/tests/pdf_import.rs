use cadcraft_io::read_pdf;
use lopdf::{Document, Object, Stream, dictionary};

fn pdf(data: &[u8]) -> Vec<u8> {
    let mut d = Document::with_version("1.5");
    let pages = d.new_object_id();
    let content = d.add_object(Stream::new(dictionary! {}, data.to_vec()));
    let page = d.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),300.into(),300.into()],"Contents"=>content,"Resources"=>dictionary!{} });
    d.objects.insert(pages, Object::Dictionary(dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1}));
    let root = d.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    d.trailer.set("Root", root);
    let mut out = Vec::new();
    d.save_to(&mut out).unwrap();
    out
}

#[test]
fn vectors_transform_to_mm_and_unpainted_clipping_paths_are_omitted() {
    let bytes = pdf(b"q 2 0 0 2 10 20 cm 0 0 m 72 0 l S Q 0 0 10 10 re W n");
    let d = read_pdf(&bytes, 1).unwrap();
    assert_eq!(d.entity_count(), 1);
    let e = d.model.iter().next().unwrap();
    let cadcraft_doc::EntityKind::LwPolyline(p) = &e.kind else { panic!("polyline required") };
    assert!((p.vertices[0].p.x - 10.0 * 25.4 / 72.0).abs() < 1e-8);
    assert!((p.vertices[1].p.x - 154.0 * 25.4 / 72.0).abs() < 1e-8);
    assert!(d.read_notes.iter().any(|n| n.contains("Clipping")));
    assert!(read_pdf(&bytes, 0).is_err());
    assert!(read_pdf(&bytes, 2).is_err());
}

#[test]
fn curves_are_editable_and_text_omission_is_reported() {
    let bytes = pdf(b"0 0 m 0 72 72 72 72 0 c S BT (label) Tj ET");
    let d = read_pdf(&bytes, 1).unwrap();
    assert_eq!(d.entity_count(), 1);
    assert!(d.read_notes.iter().any(|n| n.contains("text was omitted")));
    assert!(read_pdf(&pdf(b""), 1).is_err());
    for data in [b"not pdf".as_slice(), b"%PDF-1.5\n", b"%PDF-1.5\ntrailer <<>>"] {
        assert!(std::panic::catch_unwind(|| read_pdf(data, 1)).unwrap().is_err());
    }
}

#[test]
fn transformed_forms_import_and_cycles_fail_without_recursing_forever() {
    let mut d = Document::load_mem(&pdf(b"/Part Do")).unwrap();
    let form = d.new_object_id();
    d.objects.insert(
        form,
        Object::Stream(Stream::new(
            dictionary! {
                "Type"=>"XObject", "Subtype"=>"Form", "BBox"=>vec![0.into(),0.into(),100.into(),100.into()],
                "Matrix"=>vec![2.into(),0.into(),0.into(),2.into(),0.into(),0.into()]
            },
            b"0 0 m 72 0 l S".to_vec(),
        )),
    );
    let page = *d.get_pages().get(&1).unwrap();
    let parent = d.get_dictionary(page).unwrap().get(b"Parent").unwrap().as_reference().unwrap();
    d.get_object_mut(page).unwrap().as_dict_mut().unwrap().remove(b"Resources");
    d.get_object_mut(parent).unwrap().as_dict_mut().unwrap().set("Resources", dictionary! {"XObject"=>dictionary!{"Part"=>form}});
    let mut bytes = Vec::new();
    d.save_to(&mut bytes).unwrap();
    let imported = read_pdf(&bytes, 1).unwrap();
    let e = imported.model.iter().next().unwrap();
    let cadcraft_doc::EntityKind::LwPolyline(p) = &e.kind else { panic!("polyline required") };
    assert!((p.vertices[1].p.x - 50.8).abs() < 1e-8);
    // A hostile form refers to itself; shared depth and decoded-byte budgets bound the work.
    let stream = d.get_object_mut(form).unwrap().as_stream_mut().unwrap();
    stream.set_content(b"/Self Do".to_vec());
    stream.dict.set("Resources", dictionary! {"XObject"=>dictionary!{"Self"=>form}});
    let mut cycle = Vec::new();
    d.save_to(&mut cycle).unwrap();
    assert!(read_pdf(&cycle, 1).unwrap_err().to_string().contains("nested/cyclic"));
}
