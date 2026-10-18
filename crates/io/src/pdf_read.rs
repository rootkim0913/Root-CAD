//! PDF vector outlines, in paper millimetres. Text, raster data and clipping are not traced.
use crate::{IoError, Result};
use cadcraft_doc::{Common, Drawing, EntityKind, LwPolyline, Space};
use cadcraft_geom::{Mat3, PolyVertex, Vec2};
use lopdf::{Dictionary, Document, Object, content::Content};

const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_POINTS: usize = 200_000;

fn error(e: impl std::fmt::Display) -> IoError {
    IoError::Format(format!("PDF: {e}"))
}
fn nums<const N: usize>(objects: &[Object]) -> Result<[f64; N]> {
    if objects.len() != N {
        return Err(error("invalid path operand count"));
    }
    let mut result = [0.0; N];
    for (out, o) in result.iter_mut().zip(objects) {
        let v = match o {
            Object::Integer(n) => *n as f64,
            Object::Real(n) => f64::from(*n),
            _ => return Err(error("non-numeric path operand")),
        };
        if !v.is_finite() {
            return Err(error("non-finite path operand"));
        }
        *out = v;
    }
    Ok(result)
}

/// Import a one-based page. These are plotted paper lengths, NOT inferred model dimensions.
pub fn read_pdf(bytes: &[u8], page: u32) -> Result<Drawing> {
    if bytes.len() > MAX_BYTES {
        return Err(error("file exceeds 32 MiB"));
    }
    let doc = Document::load_mem_with_options(bytes, lopdf::LoadOptions::with_max_decompressed_size(MAX_BYTES)).map_err(error)?;
    if doc.is_encrypted() {
        return Err(error("encrypted PDFs are not supported"));
    }
    let id = doc.get_pages().get(&page).copied().ok_or_else(|| error("page does not exist (pages start at 1)"))?;
    let data = doc.get_page_content_with_limit(id, MAX_BYTES).map_err(error)?;
    let resources = page_resources(&doc, id)?;
    let mut p =
        Parser { doc: &doc, drawing: Drawing::new_metric(), points: 0, operations: 0, decoded: data.len(), notes: std::collections::BTreeSet::new() };
    p.drawing.ensure_layer("PDF_VECTOR");
    p.parse(&data, &resources, Mat3::scale(25.4 / 72.0, 25.4 / 72.0), 0)?;
    if p.drawing.model.is_empty() {
        return Err(error("no vector paths on this page; scanned images need manual/AI tracing, not vector import"));
    }
    p.drawing.read_notes.push(format!("PDF page {page}: paper millimetres. Calibrate from a known dimension before using model measurements. Page rotation/crop are not applied; curves are approximated by chords."));
    p.drawing.read_notes.extend(p.notes);
    Ok(p.drawing)
}

/// Resources are inherited as a whole, and may be inline dictionaries or references.
fn page_resources(doc: &Document, mut id: lopdf::ObjectId) -> Result<Dictionary> {
    for _ in 0..64 {
        let node = doc.get_dictionary(id).map_err(error)?;
        if let Ok(resources) = node.get(b"Resources") {
            let (_, resources) = doc.dereference(resources).map_err(error)?;
            return resources.as_dict().cloned().map_err(error);
        }
        match node.get(b"Parent") {
            Ok(parent) => id = parent.as_reference().map_err(error)?,
            Err(_) => return Ok(Dictionary::new()),
        }
    }
    Err(error("page resource inheritance limit exceeded"))
}

struct Parser<'a> {
    doc: &'a Document,
    drawing: Drawing,
    points: usize,
    operations: usize,
    decoded: usize,
    notes: std::collections::BTreeSet<String>,
}

impl Parser<'_> {
    fn push(&mut self, path: &mut Vec<Vec2>, p: Vec2) -> Result<()> {
        self.points += 1;
        if self.points > MAX_POINTS || !p.is_finite() {
            return Err(error("path coordinate/point limit exceeded"));
        }
        path.push(p);
        Ok(())
    }
    fn emit(&mut self, paths: &mut Vec<(Vec<Vec2>, bool)>) -> Result<()> {
        for (pts, closed) in paths.drain(..) {
            if pts.len() < 2 {
                continue;
            }
            let kind = EntityKind::LwPolyline(LwPolyline {
                vertices: pts.into_iter().map(PolyVertex::new).collect(),
                closed,
                const_width: 0.0,
                elevation: 0.0,
                plinegen: false,
            });
            self.drawing.add(&Space::Model, Common { layer: "PDF_VECTOR".into(), ..Common::default() }, kind).map_err(error)?;
        }
        Ok(())
    }
    fn parse(&mut self, data: &[u8], resources: &Dictionary, initial: Mat3, depth: usize) -> Result<()> {
        if depth > 16 {
            return Err(error("nested/cyclic form limit exceeded"));
        }
        let content = Content::decode(data).map_err(error)?;
        let mut matrix = initial;
        let mut stack = Vec::new();
        let mut paths = Vec::new();
        let mut path = Vec::new();
        let mut current = Vec2::ZERO;
        for op in content.operations {
            self.operations += 1;
            if self.operations > MAX_POINTS * 4 {
                return Err(error("operation limit exceeded"));
            }
            match op.operator.as_str() {
                "q" => {
                    if stack.len() >= 64 {
                        return Err(error("graphics stack limit exceeded"));
                    }
                    stack.push(matrix);
                }
                "Q" => matrix = stack.pop().ok_or_else(|| error("unbalanced graphics stack"))?,
                "cm" => {
                    let [a, b, c, d, e, f] = nums(&op.operands)?;
                    matrix = Mat3 { a, b, c, d, e, f }.then(matrix);
                }
                "m" => {
                    if !path.is_empty() {
                        paths.push((std::mem::take(&mut path), false));
                    }
                    let [x, y] = nums(&op.operands)?;
                    current = matrix.apply(Vec2::new(x, y));
                    self.push(&mut path, current)?;
                }
                "l" => {
                    let [x, y] = nums(&op.operands)?;
                    current = matrix.apply(Vec2::new(x, y));
                    self.push(&mut path, current)?;
                }
                "c" | "v" | "y" => {
                    let (a, b, end) = if op.operator == "c" {
                        let [x1, y1, x2, y2, x3, y3] = nums(&op.operands)?;
                        (matrix.apply(Vec2::new(x1, y1)), matrix.apply(Vec2::new(x2, y2)), matrix.apply(Vec2::new(x3, y3)))
                    } else {
                        let [x1, y1, x2, y2] = nums(&op.operands)?;
                        let first = matrix.apply(Vec2::new(x1, y1));
                        let end = matrix.apply(Vec2::new(x2, y2));
                        if op.operator == "v" { (current, first, end) } else { (first, end, end) }
                    };
                    let start = current;
                    for i in 1..=32 {
                        let t = f64::from(i) / 32.0;
                        let u = 1.0 - t;
                        self.push(&mut path, start * (u * u * u) + a * (3.0 * u * u * t) + b * (3.0 * u * t * t) + end * (t * t * t))?;
                    }
                    current = end;
                }
                "re" => {
                    if !path.is_empty() {
                        paths.push((std::mem::take(&mut path), false));
                    }
                    let [x, y, w, h] = nums(&op.operands)?;
                    current = matrix.apply(Vec2::new(x, y));
                    for p in [Vec2::new(x, y), Vec2::new(x + w, y), Vec2::new(x + w, y + h), Vec2::new(x, y + h)] {
                        self.push(&mut path, matrix.apply(p))?;
                    }
                    paths.push((std::mem::take(&mut path), true));
                }
                "h" => {
                    if let Some(first) = path.first().copied() {
                        current = first;
                        self.push(&mut path, first)?;
                    }
                }
                "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" => {
                    if !path.is_empty() {
                        paths.push((std::mem::take(&mut path), op.operator != "S"));
                    }
                    if matches!(op.operator.as_str(), "f" | "F" | "f*" | "B" | "B*" | "b" | "b*") {
                        for (_, closed) in &mut paths {
                            *closed = true;
                        }
                    }
                    self.emit(&mut paths)?;
                }
                "n" => {
                    path.clear();
                    paths.clear();
                }
                "W" | "W*" => {
                    self.notes.insert("Clipping is not applied: paths outside clipping boundaries may be present.".into());
                }
                "Tj" | "TJ" | "'" | "\"" => {
                    self.notes.insert("PDF text was omitted; labels and dimensions need review/recreation.".into());
                }
                "BI" | "ID" | "EI" => {
                    self.notes.insert("Inline raster images were omitted.".into());
                }
                "Do" => {
                    let name = op.operands.first().and_then(|o| o.as_name().ok()).ok_or_else(|| error("invalid XObject name"))?;
                    let objects =
                        resources.get(b"XObject").and_then(|o| self.doc.dereference(o).map(|(_, v)| v)).and_then(Object::as_dict).map_err(error)?;
                    let object =
                        objects.get(name).and_then(|o| self.doc.dereference(o).map(|(_, v)| v)).and_then(Object::as_stream).map_err(error)?;
                    if object.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Form") {
                        self.notes.insert("Form bounding-box clipping is not applied.".into());
                        let mut form_matrix = matrix;
                        if let Ok(a) = object.dict.get(b"Matrix").and_then(Object::as_array) {
                            let [a, b, c, d, e, f] = nums(a)?;
                            form_matrix = Mat3 { a, b, c, d, e, f }.then(matrix);
                        }
                        let res = object
                            .dict
                            .get(b"Resources")
                            .ok()
                            .and_then(|o| self.doc.dereference(o).ok())
                            .and_then(|(_, o)| o.as_dict().ok())
                            .unwrap_or(resources);
                        let bytes = if object.dict.has(b"Filter") {
                            object.decompressed_content_with_limit(MAX_BYTES.saturating_sub(self.decoded)).map_err(error)?
                        } else {
                            object.content.clone()
                        };
                        self.decoded = self.decoded.saturating_add(bytes.len());
                        if self.decoded > MAX_BYTES {
                            return Err(error("decompressed content limit exceeded"));
                        }
                        self.parse(&bytes, res, form_matrix, depth + 1)?;
                    } else {
                        self.notes.insert("Raster images were omitted; scanned geometry needs manual/AI tracing.".into());
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
