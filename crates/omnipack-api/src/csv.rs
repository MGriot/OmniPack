//! CSV in and out: the load list of a plan, and item lists in the simple
//! format (for drop folders and spreadsheets).

use crate::erp::{unit_of, ErpItem};
use omnipack_core::PackResult;

fn field(v: &str) -> String {
    if v.contains([',', ';', '"', '\n']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

/// The load list: one row per unit in loading order, mm and kg (the same
/// columns as the app's CSV export).
pub fn load_list(result: &PackResult) -> String {
    let mut rows = vec![
        "container,seq,unit,item,unit_no,shape,x_mm,y_mm,z_mm,w_mm,h_mm,d_mm,orientation,mass_kg,load_on_top_kg,floor_kg_m2,stop,chocks,securing,lashings,issues".to_string(),
    ];
    for c in &result.containers {
        let mut ps: Vec<_> = c.placements.iter().collect();
        ps.sort_by_key(|p| p.seq);
        for p in ps {
            let issues: Vec<_> = c.transport.iter().flat_map(|t| t.issues.iter().map(move |i| (t, i))).filter(|(_, i)| i.item == p.instance_id).collect();
            let text: Vec<String> = issues
                .iter()
                .map(|(t, i)| {
                    let kind = serde_json::to_value(i.kind).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default();
                    let dir = i.direction.and_then(|d| serde_json::to_value(d).ok()).and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default();
                    format!("{}: {kind} {dir} {:.2} kN", t.case, i.required)
                })
                .collect();
            let lashings = issues.iter().map(|(_, i)| i.lashings).max().unwrap_or(0);
            let row = [
                c.id.clone(),
                (p.seq + 1).to_string(),
                p.instance_id.clone(),
                p.item_id.clone(),
                unit_of(&p.instance_id).to_string(),
                name(&p.shape),
                format!("{:.1}", p.position[0]),
                format!("{:.1}", p.position[1]),
                format!("{:.1}", p.position[2]),
                format!("{:.1}", p.size[0]),
                format!("{:.1}", p.size[1]),
                format!("{:.1}", p.size[2]),
                name(&p.orientation),
                format!("{:.2}", p.mass),
                format!("{:.2}", p.load_on_top),
                format!("{:.0}", p.floor_pressure),
                p.stop.to_string(),
                if p.needs_chocks { "yes".into() } else { String::new() },
                name(&p.securing),
                lashings.to_string(),
                text.join("; "),
            ];
            rows.push(row.iter().map(|v| field(v)).collect::<Vec<_>>().join(","));
        }
    }
    rows.join("\n") + "\n"
}

/// Name of a serde enum value (`"WHD"`, `"lashing"`) or tagged enum (`"box"`).
fn name<T: serde::Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(serde_json::Value::String(s)) => s,
        Ok(serde_json::Value::Object(m)) => m.get("kind").and_then(|k| k.as_str()).unwrap_or("").to_string(),
        _ => String::new(),
    }
}

/// Splits one CSV line (comma or semicolon separated, quotes allowed).
fn split(line: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            c if c == sep && !quoted => out.push(std::mem::take(&mut cur).trim().to_string()),
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

fn truthy(v: &str) -> bool {
    matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "y" | "x" | "ja" | "si" | "sì")
}

fn number(v: &str) -> Option<f64> {
    let v = v.trim();
    if v.is_empty() {
        return None;
    }
    // Decimal comma (common in SAP exports): "1,5" with ';' as the separator.
    v.parse().ok().or_else(|| v.replace(',', ".").parse().ok())
}

/// SAP field names accepted as column names (delivery item / material master).
fn canonical(col: &str) -> &str {
    match col {
        "material" | "matnr" => "id",
        "maktx" => "description",
        "qty" | "lfimg" | "menge" => "quantity",
        "laeng" => "length",
        "breit" => "width",
        "hoehe" => "height",
        "brgew" => "weight",
        c => c,
    }
}

/// Items from CSV with a header row. Columns (any order, case-insensitive):
/// id, description, quantity, length, width, height, weight, and optionally
/// shape, diameter, stackable, max_load_on_top, fragile, this_side_up,
/// floor_only, stop, zone, friction. The SAP names MATNR, MAKTX, LFIMG/MENGE,
/// LAENG, BREIT, HOEHE and BRGEW work too. Separator: `,` or `;`. Booleans:
/// 1/0, true/false, yes/no or X (SAP).
pub fn parse_items(text: &str) -> Result<Vec<ErpItem>, String> {
    let mut lines = text.lines().map(|l| l.trim_start_matches('\u{feff}')).filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'));
    let header = lines.next().ok_or("empty CSV")?;
    let sep = if header.matches(';').count() > header.matches(',').count() { ';' } else { ',' };
    let cols: Vec<String> = split(header, sep).into_iter().map(|c| canonical(&c.to_ascii_lowercase().replace([' ', '-'], "_")).to_string()).collect();
    let col = |name: &str| cols.iter().position(|c| c == name);
    let id_col = col("id").ok_or("CSV: no `id` column")?;
    let mut items = Vec::new();
    for (n, line) in lines.enumerate() {
        let f = split(line, sep);
        let get = |name: &str| col(name).and_then(|i| f.get(i)).map(String::as_str).unwrap_or("");
        let num = |name: &str| number(get(name));
        let row = n + 2;
        let id = f.get(id_col).cloned().unwrap_or_default();
        if id.is_empty() {
            return Err(format!("CSV line {row}: empty id"));
        }
        let need = |name: &str| num(name).ok_or_else(|| format!("CSV line {row}: missing {name}"));
        let shape = Some(get("shape").to_string()).filter(|s| !s.is_empty());
        let round = matches!(shape.as_deref().map(str::to_ascii_lowercase).as_deref(), Some("cylinder" | "drum" | "roll" | "sphere" | "ball"));
        items.push(ErpItem {
            id,
            description: Some(get("description").to_string()).filter(|s| !s.is_empty()),
            quantity: num("quantity").map_or(1, |q| q.max(0.0).round() as u32),
            shape,
            length: if round { num("length").unwrap_or(0.0) } else { need("length")? },
            width: if round { num("width").unwrap_or(0.0) } else { need("width")? },
            height: if round { num("height").unwrap_or(0.0) } else { need("height")? },
            diameter: num("diameter"),
            weight: need("weight")?,
            stackable: col("stackable").is_none() || truthy(get("stackable")),
            max_load_on_top: num("max_load_on_top"),
            fragile: truthy(get("fragile")),
            this_side_up: truthy(get("this_side_up")),
            floor_only: truthy(get("floor_only")),
            stop: num("stop").map_or(0, |s| s.max(0.0) as u32),
            zone: Some(get("zone").to_string()).filter(|s| !s.is_empty()),
            friction: num("friction"),
            color: None,
        });
    }
    if items.is_empty() {
        return Err("CSV: no items".into());
    }
    Ok(items)
}
