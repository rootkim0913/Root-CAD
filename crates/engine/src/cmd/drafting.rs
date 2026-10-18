//! Small, bounded drafting plans for external AI clients. No network or file side effects.
use super::helpers::{circle, line, lwpoly, rect_vertices};
use super::*;

const MAX_STEPS: usize = 256;
const EDITS: &[&str] = &[
    "line",
    "pline",
    "circle",
    "arc",
    "rectang",
    "polygon",
    "ellipse",
    "point",
    "spline",
    "text",
    "mtext",
    "dimlinear",
    "dimaligned",
    "dimdiameter",
    "dimradius",
    "dimangular",
    "erase",
    "move",
    "copy",
    "rotate",
    "scale",
    "mirror",
    "offset",
    "trim",
    "extend",
    "fillet",
    "chamfer",
    "stretch",
    "break",
    "join",
    "explode",
    "layer.new",
    "flange",
];

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("draft", "Drafting Plan", run_plan)
            .params("{steps: [{command, params?: object}], dryRun?: bool, expectedRevision?: integer} → {results, dryRun}; at most 256 drawing-only edits, atomic rollback on error, one undo step. dryRun validates by executing then restoring the drawing; returned handles are provisional. No file, script, undo or nested draft commands."),
        CommandSpec::new("flange", "Flange and Pipe", run_flange)
            .params("{outerDiameter, boltCircle, holeDiameter, holes: 2..360, center?: [x,y], angle?: degrees, pipeDiameter?, pipeLength?, flangeThickness?} → {handles, dimensions, warnings}. Front view with bolt holes; pipe side view requires pipeDiameter, pipeLength AND flangeThickness. All lengths in current drawing units; no unspecified wall/bore dimensions are invented."),
    ]
}

fn run_plan(s: &mut Session, p: &Value) -> Result<Value> {
    if s.running.is_some() {
        return Err(bad("draft", "finish or cancel the active interactive command first"));
    }
    let steps = p
        .get("steps")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty() && a.len() <= MAX_STEPS)
        .ok_or_else(|| bad("draft", "`steps` must contain 1..256 edits"))?;
    let dry = match p.get("dryRun") {
        None => false,
        Some(v) => v.as_bool().ok_or_else(|| bad("draft", "`dryRun` must be boolean"))?,
    };
    if let Some(v) = p.get("expectedRevision") {
        let rev = v.as_u64().ok_or_else(|| bad("draft", "`expectedRevision` must be an unsigned integer"))?;
        if s.state()?.revision != rev {
            return Err(bad("draft", "the drawing changed; inspect it and rebuild the plan"));
        }
    }
    // Validate every command before the first mutation. The allowlist rules out external effects.
    let mut edits = Vec::with_capacity(steps.len());
    for (i, step) in steps.iter().enumerate() {
        let id = step
            .get("command")
            .and_then(Value::as_str)
            .map(str::to_ascii_lowercase)
            .filter(|id| EDITS.contains(&id.as_str()))
            .ok_or_else(|| bad("draft", format!("step {}: command is not an allowed drawing edit", i + 1)))?;
        let params = step.get("params").cloned().unwrap_or_else(|| serde_json::json!({}));
        if !params.is_object() {
            return Err(bad("draft", format!("step {}: params must be an object", i + 1)));
        }
        edits.push((id, params));
    }
    let before = s.state()?.clone();
    let settings = s.settings.clone();
    let last_dim = s.last_dim;
    let mut results = Vec::with_capacity(edits.len());
    for (i, (id, params)) in edits.iter().enumerate() {
        match s.execute(id, params) {
            Ok(v) => results.push(v),
            Err(e) => {
                *s.state_mut()? = before;
                s.settings = settings;
                s.last_dim = last_dim;
                return Err(bad("draft", format!("step {} ({id}): {e}; all edits rolled back", i + 1)));
            }
        }
    }
    if dry {
        *s.state_mut()? = before;
        s.settings = settings;
        s.last_dim = last_dim;
    } else {
        // The enclosing Session::execute records the whole plan as one undo step.
        let st = s.state_mut()?;
        st.undo = before.undo;
        st.redo = before.redo;
        st.undo_ctl = before.undo_ctl;
        st.revision = before.revision;
    }
    Ok(serde_json::json!({"results": results, "dryRun": dry, "steps": edits.len()}))
}

fn run_flange(s: &mut Session, p: &Value) -> Result<Value> {
    let size = |key| size_param("flange", p, key, false);
    let required = |key| size(key)?.ok_or_else(|| bad("flange", format!("`{key}` is required")));
    let od = required("outerDiameter")?;
    let pcd = required("boltCircle")?;
    let hole = required("holeDiameter")?;
    let count = p
        .get("holes")
        .and_then(Value::as_u64)
        .filter(|n| (2..=360).contains(n))
        .ok_or_else(|| bad("flange", "`holes` must be an integer in 2..360"))?;
    let center = match p.get("center") {
        None => Vec2::ZERO,
        Some(v) => point_value(v).ok_or_else(|| bad("flange", "`center` must be a finite point"))?,
    };
    let angle = match p.get("angle") {
        None => 0.0,
        Some(_) => f64_req("flange", p, "angle")?.to_radians(),
    };
    let pipe = size("pipeDiameter")?;
    let length = size("pipeLength")?;
    let thickness = size("flangeThickness")?;
    if pcd + hole >= od || hole >= pcd * (std::f64::consts::PI / count as f64).sin() {
        return Err(bad("flange", "bolt holes must be inside the flange and must not overlap"));
    }
    if pipe.is_some_and(|d| d >= pcd - hole) {
        return Err(bad("flange", "pipe diameter intersects the bolt holes"));
    }
    if length.is_some() && pipe.is_none() {
        return Err(bad("flange", "pipeLength requires pipeDiameter"));
    }
    let mut kinds = vec![circle(center, od / 2.0)];
    if let Some(d) = pipe {
        kinds.push(circle(center, d / 2.0));
    }
    for i in 0..count {
        kinds.push(circle(Vec2::polar(center, pcd / 2.0, angle + cadcraft_geom::TAU * i as f64 / count as f64), hole / 2.0));
    }
    let r = od * 0.6;
    kinds.push(line(center + Vec2::new(-r, 0.0), center + Vec2::new(r, 0.0)));
    kinds.push(line(center + Vec2::new(0.0, -r), center + Vec2::new(0.0, r)));
    let mut warnings = vec!["Schematic only: bore, pipe wall, tolerances and material are unspecified; no manufacturing standard is implied."];
    if let (Some(d), Some(l), Some(t)) = (pipe, length, thickness) {
        let x = center.x + od;
        kinds.push(lwpoly(rect_vertices(Vec2::new(x, center.y - od / 2.0), Vec2::new(x + t, center.y + od / 2.0)), true));
        kinds.push(lwpoly(rect_vertices(Vec2::new(x + t, center.y - d / 2.0), Vec2::new(x + t + l, center.y + d / 2.0)), true));
    } else if length.is_some() {
        warnings.push("Side view omitted: supply flangeThickness; no thickness has been guessed.");
    }
    let mut handles = Vec::with_capacity(kinds.len());
    for kind in kinds {
        handles.push(s.add_entity(kind)?.hex());
    }
    Ok(serde_json::json!({"handles": handles, "dimensions": p, "warnings": warnings}))
}
