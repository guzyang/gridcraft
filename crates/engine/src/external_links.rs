//! Refreshing the values that come from links to other workbooks.
//!
//! A workbook that reads cells of another file keeps two things: the formula text
//! (`[1]Sheet1!A1`, the index of the link in the file's `<externalReferences>`) and the linked
//! workbook's last known values, which Excel caches in the link's own part
//! (`xl/externalLinks/externalLink1.xml`). GridCraft keeps both parts as they were, so a file
//! opened here shows the values Excel last saw — but nothing ever updates them.
//!
//! This module refreshes them: the user names the file a link points at, the file is opened, and
//! every cell whose formula reads that link takes the linked workbook's current value. The
//! formula text is left alone: the link stays a link, and Excel still resolves it when the file
//! is opened there.
//!
//! Only the values are taken, never the linked workbook's formulas: a link reads a cell's result,
//! which is what Excel refreshes too.

use std::path::Path;

use gridcraft_core::{CellRef, Value};
use gridcraft_model::{Cell, Workbook};
use serde_json::{Value as Json, json};

use crate::{Result, Session};

/// A reference to another workbook in a formula: the link's index, the sheet it reads and the
/// cell there.
///
/// Excel writes the index form (`[1]Sheet1!A1`); a formula typed by hand may name the file
/// (`'[Book2.xlsx]Sheet1'!A1`), which names no link part and so cannot be resolved here.
fn external_reference(text: &str) -> Option<(u32, String, String)> {
    let start = text.find('[')?;
    let end = text[start..].find(']')?;
    let index: u32 = text[start + 1..end].trim().parse().ok()?;
    let rest = &text[end + 1..];
    let sheet_end = rest.find('!')?;
    let sheet = rest[..sheet_end].trim().trim_matches('\'').to_string();
    // The cell the reference reads: `A1`, `$A$1`.
    let cell_part = rest[sheet_end + 1..].trim_start_matches('=');
    let cell_end = cell_part.find(|c: char| !c.is_ascii_alphanumeric() && c != '$').unwrap_or(cell_part.len());
    Some((index, sheet, cell_part[..cell_end].to_string()))
}

/// A cell of the workbook whose formula reads another workbook's cell.
struct ExternalRead {
    /// The sheet and cell that hold the formula.
    sheet: usize,
    cell: CellRef,
    /// The link's index and the sheet and cell it reads.
    link: u32,
    in_sheet: String,
    in_cell: CellRef,
}

/// Every cell of the workbook whose formula reads another workbook.
fn external_reads(wb: &Workbook) -> Vec<ExternalRead> {
    let mut out = Vec::new();
    for (si, sh) in wb.sheets.iter().enumerate() {
        for (cell, value) in sh.cells.iter() {
            let Some(formula) = &value.formula else { continue };
            let Some((link, in_sheet, in_cell)) = external_reference(&formula.text) else { continue };
            let Some(in_cell) = CellRef::parse(&in_cell) else { continue };
            out.push(ExternalRead { sheet: si, cell, link, in_sheet, in_cell });
        }
    }
    out
}

/// The workbook a link points at, opened and read.
///
/// The path is the user's: a link's target may be relative to the file that holds it, so a path
/// that does not exist as given is tried next to the file that was open.
fn open_linked(path: &str, base: Option<&Path>) -> Option<Workbook> {
    let read = |p: &Path| {
        let bytes = std::fs::read(p).ok()?;
        let (wb, _) = gridcraft_xlsx::read_xlsx(&bytes).ok()?;
        Some(wb)
    };
    let direct = Path::new(path);
    if direct.is_file()
        && let Some(wb) = read(direct)
    {
        return Some(wb);
    }
    // A relative target is relative to the workbook that holds the link.
    let base = base?;
    let beside = base.join(path);
    if beside.is_file() { read(&beside) } else { None }
}

/// The value of a cell of another workbook, read from the linked file.
fn linked_value(wb: &Workbook, sheet: &str, cell: CellRef) -> Option<Value> {
    let sh = wb.sheets.iter().find(|s| s.name.eq_ignore_ascii_case(sheet))?;
    Some(sh.value(cell))
}

/// Refreshes the values of the cells that read another workbook.
///
/// `path` is the file the link points at: the user names it, since a link's target may be
/// relative to the file that holds it and may name a file that has moved. Returns how many cells
/// were updated and how many the linked file did not have.
pub fn refresh(s: &mut Session, path: &str) -> Result<Json> {
    let wb = s.doc()?.wb.clone();
    let base = s.doc()?.path.as_deref().and_then(|p| Path::new(p).parent().map(Path::to_path_buf));
    let Some(linked) = open_linked(path, base.as_deref()) else {
        return Ok(json!({"cells": 0, "warning": format!("the linked file '{path}' could not be opened")}));
    };
    let reads = external_reads(&wb);
    let mut missing = 0usize;
    let mut changes: Vec<(usize, CellRef, Value)> = Vec::new();
    for read in &reads {
        let Some(value) = linked_value(&linked, &read.in_sheet, read.in_cell) else {
            missing += 1;
            continue;
        };
        let current = wb.sheet(read.sheet).map(|sh| sh.value(read.cell)).unwrap_or(Value::Empty);
        if current == value {
            continue;
        }
        changes.push((read.sheet, read.cell, value));
    }
    let changed = changes.len();
    // The cells are written with their formula kept: a link stays a link.
    if changed > 0 {
        let mut wb = (*s.doc()?.wb).clone();
        for (sheet, cell, value) in changes {
            let Some(sh) = wb.sheet_mut(sheet) else { continue };
            let formula = sh.cell(cell).and_then(|c| c.formula.clone());
            let style = sh.cell(cell).map(|c| c.style).unwrap_or_default();
            let mut new = Cell::value(value);
            new.formula = formula;
            new.style = style;
            sh.set_cell(cell, new);
        }
        s.doc_mut()?.wb = std::sync::Arc::new(wb);
    }
    Ok(json!({"cells": changed, "missing": missing}))
}

/// The links of a workbook: their index and the sheet and cell they read, for the user to see.
pub fn links(s: &mut Session) -> Result<Json> {
    let wb = s.doc()?.wb.clone();
    let mut out: Vec<Json> = Vec::new();
    for read in external_reads(&wb) {
        out.push(json!({"link": read.link, "sheet": read.in_sheet, "cell": read.in_cell.to_string()}));
    }
    out.sort_by_key(|v| v["link"].as_u64().unwrap_or(0));
    Ok(json!({"links": out}))
}
