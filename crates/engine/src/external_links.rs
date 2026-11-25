//! Refreshing the values that come from links to other workbooks.
//!
//! A workbook that reads cells of another file keeps two things: the formula text
//! (`[1]Sheet1!A1`, the index of the link in the file's `<externalReferences>`) and the linked
//! workbook's last known values, which Excel caches in the link's own part
//! (`xl/externalLinks/externalLink1.xml`). GridCraft reads both, so a file opened here shows the
//! values Excel last saw even when the source is not there.
//!
//! Opening a workbook follows the links it can find, as Excel does by default: the values are
//! replaced by the linked files' own and the cells that read them are calculated again. A link
//! whose file is missing keeps the values the file saved, which is what Excel shows too.
//!
//! Only the values are taken, never the linked workbook's formulas: a link reads a cell's result,
//! which is what Excel refreshes too.

use std::path::{Path, PathBuf};

use gridcraft_core::{CellRef, Value};
use gridcraft_model::{Cell, Workbook};
use serde_json::{Value as Json, json};

use crate::{DocState, Result, Session};

/// What a formula names: the link's position, or the file it points at.
#[derive(Clone, PartialEq)]
enum LinkRef {
    Index(u32),
    File(String),
}

/// A reference to another workbook in a formula: what it names, the sheet it reads and the cell
/// there.
///
/// Excel writes the index form (`[1]Sheet1!A1`); a formula typed by hand may name the file
/// (`'[Book2.xlsx]Sheet1'!A1`), which is matched to a link by the file's name.
fn external_reference(text: &str) -> Option<(LinkRef, String, String)> {
    let start = text.find('[')?;
    let end = text[start..].find(']')?;
    let inner = text[start + 1..end].trim();
    let link = match inner.parse::<u32>() {
        Ok(i) => LinkRef::Index(i),
        Err(_) => LinkRef::File(inner.rsplit(['\\', '/']).next()?.to_string()),
    };
    let rest = &text[end + 1..];
    let sheet_end = rest.find('!')?;
    let sheet = rest[..sheet_end].trim().trim_matches('\'').to_string();
    // The cell the reference reads: `A1`, `$A$1`.
    let cell_part = rest[sheet_end + 1..].trim_start_matches('=');
    let cell_end = cell_part.find(|c: char| !c.is_ascii_alphanumeric() && c != '$').unwrap_or(cell_part.len());
    Some((link, sheet, cell_part[..cell_end].to_string()))
}

/// A cell of the workbook whose formula reads another workbook's cell.
struct ExternalRead {
    /// The sheet and cell that hold the formula.
    sheet: usize,
    cell: CellRef,
    /// What the formula names, and the sheet and cell it reads.
    link: LinkRef,
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
    // A formula that names its file reads the link whose target has that file name.
    for read in &mut out {
        if let LinkRef::File(name) = &read.link
            && let Some(link) = wb.external_links.iter().find(|l| l.target.as_deref().is_some_and(|t| file_name_is(t, name)))
        {
            read.link = LinkRef::Index(link.index);
        }
    }
    out
}

/// Whether `target` (a path or a bare name) ends with the file name `name`.
fn file_name_is(target: &str, name: &str) -> bool {
    let base = target.rsplit(['\\', '/']).next().unwrap_or(target);
    base.eq_ignore_ascii_case(name)
}

/// The workbook a link points at, opened and read.
///
/// The path is the user's or the link's: a link's target may be relative to the file that holds
/// it, so a path that does not exist as given is tried next to the file that was open.
fn open_linked(path: &str, base: Option<&Path>) -> Option<Workbook> {
    let read = |p: &Path| {
        let bytes = std::fs::read(p).ok()?;
        let (wb, _) = gridcraft_xlsx::read_xlsx(&bytes).ok()?;
        Some(wb)
    };
    for candidate in candidates(path, base) {
        if candidate.is_file()
            && let Some(wb) = read(&candidate)
        {
            return Some(wb);
        }
    }
    None
}

/// The paths a link target may mean: as written, without its `file://` prefix or URI escapes, and
/// with the separators of this system.
fn candidates(target: &str, base: Option<&Path>) -> Vec<PathBuf> {
    let trimmed = target.trim();
    let mut forms = vec![trimmed.to_string()];
    if let Some(rest) = trimmed.strip_prefix("file://") {
        forms.push(rest.to_string());
        // `file:///C:/dir/Book.xlsx` comes back to a path without the leading slash.
        if let Some(rest) = rest.strip_prefix('/')
            && rest.get(1..2) == Some(":")
        {
            forms.push(rest.to_string());
        }
    }
    if trimmed.contains('\\') && !cfg!(windows) {
        forms.push(trimmed.replace('\\', "/"));
    }
    let mut out = Vec::new();
    for form in forms {
        let decoded = form.replace("%20", " ").replace("%25", "%");
        let p = PathBuf::from(&decoded);
        if p.is_absolute() {
            out.push(p);
        } else {
            if let Some(base) = base {
                out.push(base.join(&decoded));
            }
            out.push(p);
        }
    }
    out
}

/// The value of a cell of another workbook, read from the linked file.
fn linked_value(wb: &Workbook, sheet: &str, cell: CellRef) -> Option<Value> {
    let sh = wb.sheets.iter().find(|s| s.name.eq_ignore_ascii_case(sheet))?;
    Some(sh.value(cell))
}

/// Replaces the values of the cells that read the linked file `path`: their formula is kept (a
/// link stays a link) and only the value changes. Returns how many cells changed and how many
/// cells the linked file did not have.
pub fn refresh_values(wb: &mut Workbook, path: &str, base: Option<&Path>) -> (usize, usize) {
    let Some(linked) = open_linked(path, base) else { return (0, 0) };
    let mut missing = 0usize;
    // Where the change goes, what it reads, and the value.
    struct Change {
        sheet: usize,
        cell: CellRef,
        link: u32,
        in_sheet: String,
        in_cell: CellRef,
        value: Value,
    }
    let mut changes: Vec<Change> = Vec::new();
    for read in external_reads(wb) {
        let LinkRef::Index(link) = read.link else { continue };
        let Some(value) = linked_value(&linked, &read.in_sheet, read.in_cell) else {
            missing += 1;
            continue;
        };
        let current = wb.sheet(read.sheet).map(|sh| sh.value(read.cell)).unwrap_or(Value::Empty);
        if current != value {
            changes.push(Change { sheet: read.sheet, cell: read.cell, link, in_sheet: read.in_sheet, in_cell: read.in_cell, value });
        }
    }
    let changed = changes.len();
    for change in &changes {
        // The link's own cache is what a calculation reads, so it takes the new value too.
        if let Some(l) = wb.external_links.iter_mut().find(|l| l.index == change.link) {
            l.values.entry(change.in_sheet.clone()).or_default().insert(change.in_cell, change.value.clone());
        }
    }
    for change in changes {
        let Some(sh) = wb.sheet_mut(change.sheet) else { continue };
        let formula = sh.cell(change.cell).and_then(|c| c.formula.clone());
        let style = sh.cell(change.cell).map(|c| c.style).unwrap_or_default();
        let mut new = Cell::value(change.value);
        new.formula = formula;
        new.style = style;
        sh.set_cell(change.cell, new);
    }
    (changed, missing)
}

/// Refreshes the values of the cells that read another workbook, then calculates what reads them.
///
/// `path` is the file the link points at: the user names it, since a link's target may be
/// relative to the file that holds it and may name a file that has moved. Returns how many cells
/// were updated and how many the linked file did not have.
pub fn refresh(s: &mut Session, path: &str) -> Result<Json> {
    let doc = s.doc_mut()?;
    let base = doc.path.as_deref().and_then(|p| Path::new(p).parent().map(Path::to_path_buf));
    if open_linked(path, base.as_deref()).is_none() {
        return Ok(json!({"cells": 0, "missing": 0, "warning": format!("the linked file '{path}' could not be opened")}));
    }
    let mut wb = (*doc.wb).clone();
    let (changed, missing) = refresh_values(&mut wb, path, base.as_deref());
    if changed > 0 {
        doc.calc.recalc_all(&mut wb);
        doc.wb = std::sync::Arc::new(wb);
    }
    Ok(json!({"cells": changed, "missing": missing}))
}

/// Follows every link whose target can be found, as opening a workbook in Excel does: the values
/// the file saved are replaced by the linked files' own, and the cells that read them are
/// calculated again. A link whose file is missing keeps the values the file saved.
///
/// Returns how many cells changed.
pub fn auto_refresh_doc(doc: &mut DocState) -> usize {
    let base = doc.path.as_deref().and_then(|p| Path::new(p).parent().map(Path::to_path_buf));
    let targets: Vec<String> = doc.wb.external_links.iter().filter_map(|l| l.target.clone()).collect();
    if targets.is_empty() {
        return 0;
    }
    let mut wb = (*doc.wb).clone();
    let mut changed = 0;
    for target in targets {
        changed += refresh_values(&mut wb, &target, base.as_deref()).0;
    }
    if changed > 0 {
        doc.calc.recalc_all(&mut wb);
        doc.wb = std::sync::Arc::new(wb);
    }
    changed
}

/// Drops the cells' link, keeping the value each formula held: what Excel's Break Link does.
/// `link` names one link; `None` drops every link the workbook has.
pub fn break_link(wb: &mut Workbook, link: Option<u32>) -> usize {
    let reads = external_reads(wb);
    let mut broken = 0;
    for read in reads {
        let hit = match (&read.link, link) {
            (LinkRef::Index(i), Some(l)) => *i == l,
            (LinkRef::Index(_), None) => true,
            _ => false,
        };
        if !hit {
            continue;
        }
        let Some(sh) = wb.sheet_mut(read.sheet) else { continue };
        let Some(cell) = sh.cells.get(read.cell) else { continue };
        let (value, style) = (cell.value.clone(), cell.style);
        let mut new = Cell::value(value);
        new.style = style;
        sh.set_cell(read.cell, new);
        broken += 1;
    }
    broken
}

/// The links of a workbook: the file each points at, the sheet and cell it reads, and whether the
/// file was found.
pub fn links(s: &mut Session) -> Result<Json> {
    let wb = s.doc()?.wb.clone();
    let base = s.doc()?.path.as_deref().and_then(|p| Path::new(p).parent().map(Path::to_path_buf));
    let mut out: Vec<Json> = Vec::new();
    for read in external_reads(&wb) {
        let (link, target) = match &read.link {
            LinkRef::Index(i) => (*i, wb.external_links.iter().find(|l| l.index == *i).and_then(|l| l.target.clone())),
            LinkRef::File(name) => (0, Some(name.clone())),
        };
        let found = target.as_deref().is_some_and(|t| open_linked(t, base.as_deref()).is_some());
        out.push(json!({
            "link": link,
            "target": target,
            "sheet": read.in_sheet,
            "cell": read.in_cell.to_string(),
            "found": found,
        }));
    }
    out.sort_by_key(|v| v["link"].as_u64().unwrap_or(0));
    Ok(json!({"links": out}))
}
