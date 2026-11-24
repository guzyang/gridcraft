//! The formatting the clipboard's HTML carries: what a cell's font, fill and alignment were.
//!
//! A spreadsheet puts an HTML table on the clipboard next to the tab-separated text. The formatting
//! lives in the markup's `<style>` block, in the classes it defines (Excel writes `.xl65{…}` and
//! puts `class="xl65"` on the cell), and in the cell's own `style` attribute. Reading it lets a
//! paste keep the source's bold, underline, colours and font, instead of dropping every format
//! and leaving plain text on a plain background.

use std::collections::HashMap;

/// The formatting one cell carries in the clipboard's HTML. Every part is optional: what the
/// markup does not say stays as the destination has it.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct HtmlStyle {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    /// 0xRRGGBB.
    pub font_color: Option<u32>,
    /// 0xRRGGBB.
    pub fill_color: Option<u32>,
    pub font_name: Option<String>,
    /// In points.
    pub font_size: Option<f32>,
    /// "left", "center" or "right".
    pub align: Option<String>,
    /// The number format the source cell showed, when the markup names one.
    pub num_fmt: Option<String>,
}

/// The declarations each class in the markup's `<style>` blocks carries, by class name.
pub(crate) fn css_classes(html: &str) -> HashMap<String, Vec<(String, String)>> {
    let mut out: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut rest = html;
    while let Some(start) = find_ci(rest, "<style") {
        let after = rest.get(start..).unwrap_or_default();
        let Some(open_end) = find_ci(after, ">") else { break };
        let body_start = open_end + 1;
        let Some(close) = find_ci(after.get(body_start..).unwrap_or_default(), "</style") else { break };
        let block = after.get(body_start..body_start + close).unwrap_or_default();
        collect_rules(block, &mut out);
        rest = after.get(body_start + close..).unwrap_or_default();
    }
    out
}

/// The formatting a cell's tag carries: the classes it names, then its own `style` attribute.
pub(crate) fn cell_style(attrs: &str, classes: &HashMap<String, Vec<(String, String)>>) -> Option<HtmlStyle> {
    let mut decls: Vec<(String, String)> = Vec::new();
    if let Some(class_attr) = attr(attrs, "class") {
        for name in class_attr.split_whitespace() {
            if let Some(d) = classes.get(name) {
                decls.extend(d.iter().cloned());
            }
        }
    }
    if let Some(inline) = attr(attrs, "style") {
        decls.extend(parse_decls(inline));
    }
    let mut style = style_from_decls(&decls);
    if let Some(fmt) = attr(attrs, "mso-number-format") {
        style.num_fmt = Some(clean_num_fmt(fmt));
    }
    if style == HtmlStyle::default() { None } else { Some(style) }
}

/// The formatting the declarations describe.
fn style_from_decls(decls: &[(String, String)]) -> HtmlStyle {
    let mut style = HtmlStyle::default();
    for (prop, value) in decls {
        match prop.as_str() {
            "font-weight" => {
                let v = value.to_ascii_lowercase();
                style.bold = Some(v == "bold" || v == "bolder" || v.parse::<u32>().is_ok_and(|n| n >= 600));
            }
            "font-style" => {
                let v = value.to_ascii_lowercase();
                style.italic = Some(v.starts_with("italic") || v.starts_with("oblique"));
            }
            "text-decoration" | "text-decoration-line" => {
                let v = value.to_ascii_lowercase();
                if v.contains("underline") {
                    style.underline = Some(true);
                } else if v.contains("none") {
                    style.underline = Some(false);
                }
            }
            "color" => style.font_color = css_color(value),
            "background-color" => style.fill_color = css_color(value),
            "font-family" => style.font_name = first_family(value),
            "font-size" => style.font_size = font_size(value),
            "text-align" => {
                let v = value.to_ascii_lowercase();
                if matches!(v.as_str(), "left" | "center" | "right" | "justify") {
                    style.align = Some(v);
                }
            }
            _ => {}
        }
    }
    style
}

/// The rules in a `<style>` block: each selector's declarations, filed under the class names the
/// selector mentions (Excel writes `table.xl65{…}` and `td.xl65{…}`).
fn collect_rules(block: &str, out: &mut HashMap<String, Vec<(String, String)>>) {
    let mut rest = block;
    while let Some(open) = rest.find('{') {
        let selector = rest.get(..open).unwrap_or_default().trim();
        let after = rest.get(open..).unwrap_or_default();
        let Some(close) = after.find('}') else { break };
        // `after` starts at the rule's `{`, so the declarations start after it.
        let decls = parse_decls(after.get(1..close).unwrap_or_default());
        for sel in selector.split(',') {
            for name in class_names(sel) {
                out.entry(name).or_default().extend(decls.iter().cloned());
            }
        }
        rest = after.get(close + 1..).unwrap_or_default();
    }
}

/// A rule's declarations, as (property, value) pairs; a declaration separated by a `;` outside
/// quotes, a property and its value split at the first `:`.
fn parse_decls(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for part in split_on(text, ';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((prop, value)) = part.split_once(':') else { continue };
        let value = value.trim();
        // A value may be quoted (`font-family:"Calibri"`); the quotes are not part of it.
        let value = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(value);
        out.push((prop.trim().to_ascii_lowercase(), value.to_string()));
    }
    out
}

/// The class names a selector mentions.
fn class_names(selector: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = selector.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'.' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b'-') {
                j += 1;
            }
            if j > start {
                out.push(selector.get(start..j).unwrap_or_default().to_string());
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// A colour as 0xRRGGBB: `#rgb`, `#rrggbb`, `rgb(r, g, b)` or one of the names a spreadsheet
/// writes. `transparent` and an automatic colour are no colour at all.
fn css_color(value: &str) -> Option<u32> {
    let v = value.trim().to_ascii_lowercase();
    if v.is_empty() || v == "transparent" || v == "auto" || v == "inherit" || v == "none" {
        return None;
    }
    if let Some(hex) = v.strip_prefix('#') {
        let hex = hex.trim();
        return match hex.len() {
            3 => {
                let d: Vec<u32> = hex.chars().map(|c| c.to_digit(16).unwrap_or(0)).collect();
                Some((d[0] * 17) << 16 | (d[1] * 17) << 8 | d[2] * 17)
            }
            6 => u32::from_str_radix(hex, 16).ok(),
            _ => None,
        };
    }
    if let Some(inner) = v.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() >= 3 {
            let r = parts[0].parse::<u32>().ok()?;
            let g = parts[1].parse::<u32>().ok()?;
            let b = parts[2].parse::<u32>().ok()?;
            if (r..=255).contains(&r) && (0..=255).contains(&g) && (0..=255).contains(&b) {
                return Some(r << 16 | g << 8 | b);
            }
        }
        return None;
    }
    let named = match v.as_str() {
        "black" => 0x000000,
        "white" => 0xFFFFFF,
        "red" => 0xFF0000,
        "green" => 0x008000,
        "lime" => 0x00FF00,
        "blue" => 0x0000FF,
        "yellow" => 0xFFFF00,
        "gray" | "grey" => 0x808080,
        "silver" => 0xC0C0C0,
        "maroon" => 0x800000,
        "navy" => 0x000080,
        "teal" => 0x008080,
        "olive" => 0x808000,
        "purple" => 0x800080,
        "orange" => 0xFFA500,
        "aqua" | "cyan" => 0x00FFFF,
        "fuchsia" | "magenta" => 0xFF00FF,
        _ => return None,
    };
    Some(named)
}

/// The first family in a `font-family` list, without its quotes.
fn first_family(value: &str) -> Option<String> {
    let first = value.split(',').next()?.trim();
    let name = first.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(first).trim();
    if name.is_empty() || name.eq_ignore_ascii_case("inherit") || name.eq_ignore_ascii_case("initial") {
        return None;
    }
    Some(name.to_string())
}

/// A font size in points: `11.0pt` as it is, `14.667px` as the points it is.
fn font_size(value: &str) -> Option<f32> {
    let v = value.trim().to_ascii_lowercase();
    let (num, unit) = v.split_at(v.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(v.len()));
    let n: f32 = num.parse().ok()?;
    match unit.trim() {
        "pt" | "" => Some(n),
        "px" => Some(n * 0.75),
        "em" | "rem" => Some(n * 12.0),
        _ => None,
    }
}

/// The number format a cell shows, as the source wrote it: `"General"` → General, `\@` → @.
fn clean_num_fmt(value: &str) -> String {
    let v = value.trim().trim_matches('"').trim();
    let v = v.strip_prefix('\\').unwrap_or(v);
    v.to_string()
}

/// Split on a separator that is not inside quotes.
fn split_on(text: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut quote: Option<char> = None;
    for (i, c) in text.char_indices() {
        match (quote, c) {
            (Some(q), c) if q == c => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, c) if c == sep => {
                out.push(text.get(start..i).unwrap_or_default());
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(text.get(start..).unwrap_or_default());
    out
}

/// Case-insensitive ASCII search.
fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    let (h, n) = (hay.as_bytes(), needle.as_bytes());
    if n.is_empty() || h.len() < n.len() {
        return None;
    }
    (0..=h.len() - n.len()).find(|&i| h.get(i..i + n.len()).is_some_and(|w| w.eq_ignore_ascii_case(n)))
}

/// The value of attribute `name` in a tag's attribute text (quoted or bare).
fn attr<'a>(attrs: &'a str, name: &str) -> Option<&'a str> {
    let mut s = attrs;
    loop {
        s = s.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
        if s.is_empty() {
            return None;
        }
        let key_end = s.find(|c: char| c.is_whitespace() || c == '=' || c == '/').unwrap_or(s.len());
        let key = s.get(..key_end).unwrap_or_default();
        s = s.get(key_end..).unwrap_or_default().trim_start();
        let mut value = None;
        if let Some(after) = s.strip_prefix('=') {
            let after = after.trim_start();
            let (v, next) = match after.chars().next() {
                Some(q @ ('"' | '\'')) => {
                    let inner = after.get(1..).unwrap_or_default();
                    let end = inner.find(q).unwrap_or(inner.len());
                    (inner.get(..end).unwrap_or_default(), inner.get(end + 1..).unwrap_or_default())
                }
                _ => {
                    let end = after.find(char::is_whitespace).unwrap_or(after.len());
                    (after.get(..end).unwrap_or_default(), after.get(end..).unwrap_or_default())
                }
            };
            value = Some(v);
            s = next;
        }
        if key.eq_ignore_ascii_case(name) {
            return value;
        }
        if key.is_empty() && value.is_none() {
            let mut chars = s.chars();
            chars.next();
            s = chars.as_str();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Excel's own markup: a `<style>` block with the classes, and cells naming them.
    const EXCEL: &str = r#"<style><!--
table.xl65{border-color:#808080;font-size:11.0pt;font-family:"Calibri",Arial}
td.xl65{font-weight:bold;font-size:11.0pt;color:rgb(0,0,255);background-color:#D9E1F2;text-align:center}
.xl70{font-weight:400;text-decoration:underline}
--></style>
<table><tr><td class="xl65" style="font-family:&#34;SimSun&#34;">中文</td><td class="xl70">x</td></tr></table>"#;

    #[test]
    fn a_class_rule_carries_the_cells_formatting() {
        let classes = css_classes(EXCEL);
        let style = cell_style(r#"class="xl65""#, &classes).expect("a style");
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.font_color, Some(0x0000FF));
        assert_eq!(style.fill_color, Some(0xD9E1F2));
        assert_eq!(style.align.as_deref(), Some("center"));
        assert_eq!(style.font_name.as_deref(), Some("Calibri"));
        assert_eq!(style.font_size, Some(11.0));
    }

    #[test]
    fn a_later_rule_and_the_inline_style_win() {
        let classes = css_classes(EXCEL);
        let style = cell_style(r#"class="xl70" style="text-decoration:none""#, &classes).expect("a style");
        assert_eq!(style.bold, Some(false));
        assert_eq!(style.underline, Some(false));
    }

    #[test]
    fn a_cell_without_formatting_has_none() {
        let classes = css_classes(EXCEL);
        assert!(cell_style("", &classes).is_none());
        assert!(cell_style(r#"class="nosuch""#, &classes).is_none());
    }

    #[test]
    fn colours_and_sizes_are_read_in_the_shapes_spreadsheets_write() {
        assert_eq!(css_color("#fff"), Some(0xFFFFFF));
        assert_eq!(css_color("#D9E1F2"), Some(0xD9E1F2));
        assert_eq!(css_color("rgb(255, 0, 0)"), Some(0xFF0000));
        assert_eq!(css_color("transparent"), None);
        assert_eq!(font_size("11.0pt"), Some(11.0));
        assert!((font_size("14.667px").unwrap() - 11.0).abs() < 0.001);
        assert_eq!(first_family("\"Calibri\",Arial").as_deref(), Some("Calibri"));
        assert_eq!(first_family("宋体").as_deref(), Some("宋体"));
    }
}

