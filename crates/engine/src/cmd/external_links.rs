//! The commands that work with links to other workbooks.

use serde_json::Value as Json;

use crate::{Result, Session};

pub fn specs() -> Vec<crate::cmd::CommandSpec> {
    vec![
        crate::cmd::cmd!(
            query "links.externalList",
            "External Links…",
            ["Data", "Links"],
            None,
            "{} → the links to other workbooks this file has",
            crate::cmd::has_doc,
            list
        ),
        crate::cmd::cmd!(
            noundo "links.refreshExternal",
            "Refresh External Links",
            ["Data", "Links"],
            None,
            "{path?: the file the link points at; the link's own target when left out} → the number of cells updated",
            crate::cmd::has_doc,
            refresh
        ),
        crate::cmd::cmd!(
            "links.changeSource",
            "Change Source…",
            ["Data", "Links"],
            None,
            "{link?: 1-based link index, path: the file it should read} → the link's new target and the cells updated",
            crate::cmd::has_doc,
            change_source
        ),
        crate::cmd::cmd!(
            "links.breakLink",
            "Break Link",
            ["Data", "Links"],
            None,
            "{link?: 1-based link index} → the values the formulas held, now without a link",
            crate::cmd::has_doc,
            break_link
        ),
    ]
}

/// The links this workbook has, for the user to see what a refresh would read.
fn list(s: &mut Session, _: &Json) -> Result<Json> {
    crate::external_links::links(s)
}

/// Refreshes the values that come from a link to another workbook: the file named, or the target
/// the link itself records.
fn refresh(s: &mut Session, p: &Json) -> Result<Json> {
    let path = match crate::cmd::str_param(p, "path") {
        Some(path) => path.to_string(),
        None => {
            let link = crate::cmd::u32_param(p, "link");
            let wb = s.doc()?.wb.clone();
            let target = wb
                .external_links
                .iter()
                .find(|l| link.is_none_or(|i| l.index == i))
                .and_then(|l| l.target.clone());
            target.ok_or_else(|| crate::EngineError::Other("links.refreshExternal needs the file the link points at".into()))?
        }
    };
    crate::external_links::refresh(s, &path)
}

/// Points a link at another file and reads it.
fn change_source(s: &mut Session, p: &Json) -> Result<Json> {
    let path = crate::cmd::str_param(p, "path")
        .ok_or_else(|| crate::EngineError::Other("links.changeSource needs the file the link should read".into()))?
        .to_string();
    let link = crate::cmd::u32_param(p, "link");
    let doc = s.doc_mut()?;
    let mut wb = (*doc.wb).clone();
    let Some(entry) = wb.external_links.iter_mut().find(|l| link.is_none_or(|i| l.index == i)) else {
        return Err(crate::EngineError::Other("this workbook has no such link".into()));
    };
    entry.target = Some(path.clone());
    let index = entry.index;
    let base = doc.path.as_deref().and_then(|p| std::path::Path::new(p).parent().map(std::path::Path::to_path_buf));
    let (cells, _) = crate::external_links::refresh_values(&mut wb, &path, base.as_deref());
    doc.calc.recalc_all(&mut wb);
    doc.wb = std::sync::Arc::new(wb);
    Ok(serde_json::json!({"link": index, "target": path, "cells": cells}))
}

/// Drops a link, keeping the values its formulas held.
fn break_link(s: &mut Session, p: &Json) -> Result<Json> {
    let link = crate::cmd::u32_param(p, "link");
    let doc = s.doc_mut()?;
    let mut wb = (*doc.wb).clone();
    let cells = crate::external_links::break_link(&mut wb, link);
    doc.wb = std::sync::Arc::new(wb);
    Ok(serde_json::json!({"cells": cells}))
}
