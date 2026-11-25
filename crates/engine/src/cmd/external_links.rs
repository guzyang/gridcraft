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
            "{path: the file the link points at} → the number of cells updated",
            crate::cmd::has_doc,
            refresh
        ),
    ]
}

/// The links this workbook has, for the user to see what a refresh would read.
fn list(s: &mut Session, _: &Json) -> Result<Json> {
    crate::external_links::links(s)
}

/// Refreshes the values that come from a link to another workbook.
fn refresh(s: &mut Session, p: &Json) -> Result<Json> {
    let path = crate::cmd::str_param(p, "path").ok_or_else(|| crate::EngineError::Other("links.refreshExternal needs the file the link points at".into()))?;
    crate::external_links::refresh(s, path)
}
