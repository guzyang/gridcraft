//! Refreshing the values that come from a link to another workbook.

use std::io::{Cursor, Write};

use gridcraft_core::Value;
use serde_json::json;

const NS_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// A workbook whose B1 reads `=[1]Sheet1!A1` from a linked file, with the value 10 cached.
fn linked_workbook(link: &str) -> Vec<u8> {
    let types = r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/externalLinks/externalLink1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml"/></Types>"#.to_string();
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{NS_REL}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#
    );
    let book = format!(
        r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="{NS_REL}"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets><externalReferences><externalReference r:id="rId2"/></externalReferences></workbook>"#
    );
    let book_rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{NS_REL}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{NS_REL}/externalLink" Target="externalLinks/externalLink1.xml"/></Relationships>"#
    );
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="B1"><f>[1]Sheet1!A1</f><v>10</v></c></row></sheetData></worksheet>"#.to_string();
    let link_part = format!(
        r#"<externalLink xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="{NS_REL}"><externalBook r:id="rId1"><sheetNames><sheetName val="Sheet1"/></sheetNames><sheetDataSet><sheetData sheetId="0"><row r="1"><cell r="A1"><v>10</v></cell></row></sheetDataSet></externalBook></externalLink>"#
    );
    let link_rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{NS_REL}/externalLinkPath" Target="{link}" TargetMode="External"/></Relationships>"#
    );
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in [
        ("[Content_Types].xml", types),
        ("_rels/.rels", rels),
        ("xl/workbook.xml", book),
        ("xl/_rels/workbook.xml.rels", book_rels),
        ("xl/worksheets/sheet1.xml", sheet),
        ("xl/externalLinks/externalLink1.xml", link_part),
        ("xl/externalLinks/_rels/externalLink1.xml.rels", link_rels),
    ] {
        zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(data.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

/// A plain workbook whose Sheet1!A1 holds a number.
fn source_workbook(value: &str) -> Vec<u8> {
    let types = r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#.to_string();
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{NS_REL}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#
    );
    let book = format!(
        r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="{NS_REL}"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>"#
    );
    let book_rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{NS_REL}/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#
    );
    let sheet = format!(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1"><v>{value}</v></c></row></sheetData></worksheet>"#
    );
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in [
        ("[Content_Types].xml", types),
        ("_rels/.rels", rels),
        ("xl/workbook.xml", book),
        ("xl/_rels/workbook.xml.rels", book_rels),
        ("xl/worksheets/sheet1.xml", sheet),
    ] {
        zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(data.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}


/// A session with the linked workbook open.
fn open_linked() -> gridcraft_engine::Session {
    let mut s = gridcraft_engine::Session::new();
    let (wb, _) = gridcraft_engine::io::open_bytes("Linked.xlsx", &linked_workbook("Source.xlsx")).unwrap();
    s.add_document(gridcraft_engine::DocState::new(wb, None, "Linked".into(), gridcraft_engine::Trust::FromFile));
    s
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("gridcraft-extlink-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A link's cell shows the value the linked file holds: refreshing reads the file and updates it.
#[test]
fn refreshing_an_external_link_reads_the_linked_file() {
    let dir = temp_dir("refresh");
    let source = dir.join("Source.xlsx");
    std::fs::write(&source, source_workbook("42")).unwrap();
    let linked = dir.join("Linked.xlsx");
    std::fs::write(&linked, linked_workbook("Source.xlsx")).unwrap();

    let mut s = open_linked();
    let at = gridcraft_core::CellRef::parse("B1").unwrap();
    assert_eq!(s.doc().unwrap().wb.sheet(0).unwrap().value(at), Value::Number(10.0), "the value the file cached");

    let out = s.execute("links.refreshExternal", json!({"path": source.to_string_lossy()})).unwrap();
    assert_eq!(out["cells"], json!(1), "{out}");
    assert_eq!(s.doc().unwrap().wb.sheet(0).unwrap().value(at), Value::Number(42.0), "the linked file's value");
    // The link stays a link: the formula is kept, so Excel still resolves it.
    let formula = s.doc().unwrap().wb.sheet(0).unwrap().cell(at).and_then(|c| c.formula.as_ref().map(|f| f.text.clone()));
    assert_eq!(formula.as_deref(), Some("[1]Sheet1!A1"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The links of a workbook are listed, so the user can see what a refresh would read.
#[test]
fn the_links_of_a_workbook_are_listed() {
    let mut s = open_linked();
    let out = s.execute("links.externalList", json!({})).unwrap();
    assert_eq!(out["links"][0]["link"], json!(1));
    assert_eq!(out["links"][0]["sheet"], json!("Sheet1"));
    assert_eq!(out["links"][0]["cell"], json!("A1"));
}

/// A link to a file that cannot be opened changes nothing and says so.
#[test]
fn a_link_to_a_missing_file_changes_nothing() {
    let mut s = open_linked();
    let at = gridcraft_core::CellRef::parse("B1").unwrap();
    let out = s.execute("links.refreshExternal", json!({"path": "nowhere.xlsx"})).unwrap();
    assert_eq!(out["cells"], json!(0));
    assert!(out["warning"].as_str().is_some_and(|w| w.contains("could not be opened")), "{out}");
    assert_eq!(s.doc().unwrap().wb.sheet(0).unwrap().value(at), Value::Number(10.0), "the cached value stays");
}

/// A workbook without links has no links to list and nothing to refresh.
#[test]
fn a_workbook_without_links_has_nothing_to_refresh() {
    let mut s = gridcraft_engine::Session::new();
    let (wb, _) = gridcraft_engine::io::open_bytes("Plain.xlsx", &source_workbook("1")).unwrap();
    s.add_document(gridcraft_engine::DocState::new(wb, None, "Plain".into(), gridcraft_engine::Trust::FromFile));
    let out = s.execute("links.externalList", json!({})).unwrap();
    assert_eq!(out["links"].as_array().unwrap().len(), 0);
    let out = s.execute("links.refreshExternal", json!({"path": "Source.xlsx"})).unwrap();
    assert_eq!(out["cells"], json!(0));
}
