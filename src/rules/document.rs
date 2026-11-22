//! Document-level WCAG rules: `document-title`, `html-has-lang`, `html-lang-valid`,
//! `html-xml-lang-mismatch`, `valid-lang`, `meta-viewport`, `meta-viewport-large`,
//! `meta-refresh`, `meta-refresh-no-exceptions`, `bypass`, `skip-link`, and
//! `avoid-inline-spacing`.

use std::collections::{BTreeMap, HashSet};

use crate::accname::collect_raw_text;
use crate::aria::{effective_role, is_sequentially_focusable, is_valid_bcp47_primary};
use crate::ir::{Document, Node, NodeId, normalize_whitespace};
use crate::rule::{Impact, Rule, RuleMeta, Violation};

pub struct DocumentTitle;
pub struct HtmlHasLang;
pub struct HtmlLangValid;
pub struct HtmlXmlLangMismatch;
pub struct ValidLang;
pub struct MetaViewport;
pub struct MetaViewportLarge;
pub struct MetaRefresh;
pub struct MetaRefreshNoExceptions;
pub struct Bypass;
pub struct SkipLink;
pub struct AvoidInlineSpacing;

impl DocumentTitle {
    pub const META: RuleMeta = RuleMeta {
        id: "document-title",
        impact: Impact::Serious,
        wcag: "2.4.2 (A)",
        tags: &[
            "cat.text-alternatives",
            "wcag2a",
            "wcag242",
            "TTv5",
            "TT12.a",
            "EN-301-549",
            "EN-9.2.4.2",
            "ACT",
            "RGAAv4",
            "RGAA-8.5.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Documents must have a non-empty <title> element",
        description: "Provide a descriptive <title> inside <head> so users and assistive technologies can identify the page.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/page-titled.html",
    };
}

impl Rule for DocumentTitle {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let Some(html_el) = doc.elements().find(|n| n.tag == "html") else {
            return;
        };
        // Per HTML spec (`document.title`), only the first <title> element in
        // document order (outside <svg>) is the document title.
        let first_title = doc.elements().find(|n| {
            if n.tag != "title" {
                return false;
            }
            !doc.ancestors(n.id)
                .into_iter()
                .any(|aid| doc.node(aid).is_some_and(|a| a.tag == "svg"))
        });
        let has_valid_title = first_title
            .is_some_and(|n| !normalize_whitespace(&collect_raw_text(doc, n.id)).is_empty());
        if !has_valid_title {
            out.push(Violation::from_node(&self.meta(), doc, html_el.id));
        }
    }
}

impl HtmlHasLang {
    pub const META: RuleMeta = RuleMeta {
        id: "html-has-lang",
        impact: Impact::Serious,
        wcag: "3.1.1 (A)",
        tags: &[
            "cat.language",
            "wcag2a",
            "wcag311",
            "TTv5",
            "TT11.a",
            "EN-301-549",
            "EN-9.3.1.1",
            "ACT",
            "RGAAv4",
            "RGAA-8.3.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "<html> element must have a lang attribute",
        description: "Set a non-empty lang attribute on <html> so screen readers select the right pronunciation rules.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/language-of-page.html",
    };
}

impl Rule for HtmlHasLang {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let Some(html_el) = doc.elements().find(|n| n.tag == "html") else {
            return;
        };
        // Per HTML spec, HTML documents require `lang` (`xml:lang` alone is ignored).
        let lang = html_el.attr("lang").map_or("", str::trim);
        if lang.is_empty() {
            out.push(Violation::from_node(&self.meta(), doc, html_el.id));
        }
    }
}

impl HtmlLangValid {
    pub const META: RuleMeta = RuleMeta {
        id: "html-lang-valid",
        impact: Impact::Serious,
        wcag: "3.1.1 (A)",
        tags: &[
            "cat.language",
            "wcag2a",
            "wcag311",
            "TTv5",
            "TT11.a",
            "EN-301-549",
            "EN-9.3.1.1",
            "ACT",
            "RGAAv4",
            "RGAA-8.4.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "<html> lang attribute must have a valid BCP 47 language tag",
        description: "Use a valid BCP 47 language tag (such as 'en', 'en-US', or 'es-419') on <html>.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/language-of-page.html",
    };
}

impl Rule for HtmlLangValid {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let Some(html_el) = doc.elements().find(|n| n.tag == "html") else {
            return;
        };
        let Some(raw) = html_el
            .attr("lang")
            .or_else(|| html_el.attr("xml:lang"))
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return;
        };
        if !is_valid_bcp47_primary(raw) {
            out.push(Violation::with_detail(
                &self.meta(),
                doc,
                html_el.id,
                format!("Invalid BCP 47 language tag '{raw}' on <html>."),
            ));
        }
    }
}

impl MetaViewport {
    pub const META: RuleMeta = RuleMeta {
        id: "meta-viewport",
        impact: Impact::Critical,
        wcag: "1.4.4 (AA)",
        tags: &[
            "cat.sensory-and-visual-cues",
            "wcag2aa",
            "wcag144",
            "EN-301-549",
            "EN-9.1.4.4",
            "ACT",
            "RGAAv4",
            "RGAA-10.4.2",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Zooming and scaling must not be disabled",
        description: "Remove user-scalable=no and ensure maximum-scale is at least 2.0 on <meta name=\"viewport\">.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/resize-text.html",
    };
}

impl Rule for MetaViewport {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "meta" {
                continue;
            }
            if !el
                .attr("name")
                .is_some_and(|n| n.trim().eq_ignore_ascii_case("viewport"))
            {
                continue;
            }
            let Some(content) = el.attr("content") else {
                continue;
            };
            if viewport_restricts_zoom(content, 2.0, None) {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

fn viewport_restricts_zoom(content: &str, min_max_scale: f64, lower_bound: Option<f64>) -> bool {
    let mut user_scalable_blocks: Option<bool> = None;
    let mut max_scale_blocks: Option<bool> = None;

    for part in content.split([',', ';']) {
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        let key = k.trim().to_ascii_lowercase();
        let val = v.trim().to_ascii_lowercase();
        if key == "user-scalable" {
            let allows_zoom =
                val == "yes" || val.parse::<f64>().is_ok_and(|n| n.is_finite() && n >= 1.0);
            user_scalable_blocks = Some(!allows_zoom);
        } else if key == "maximum-scale" && !val.is_empty() {
            let parsed_scale = if let Ok(scale) = val.parse::<f64>()
                && scale.is_finite()
            {
                if scale < 0.0 {
                    // Negative values are ignored by browsers.
                    continue;
                }
                Some(scale)
            } else if matches!(val.as_str(), "device-width" | "device-height") {
                // Device dimension keywords resolve to >= 320px, allowing zoom.
                max_scale_blocks = Some(false);
                None
            } else if val == "yes" {
                // WebKit/Blink coerce "yes" to 1.0.
                Some(1.0)
            } else {
                // WebKit/Blink coerce "no" and other non-numeric values (e.g. "invalid") to 0.0.
                Some(0.0)
            };
            if let Some(scale) = parsed_scale {
                let in_band = scale < min_max_scale && lower_bound.is_none_or(|lb| scale >= lb);
                max_scale_blocks = Some(in_band);
            }
        }
    }

    if lower_bound.is_some() {
        // Best-practice 500% rule avoids double-reporting violations already caught by `meta-viewport` (< 2.0 / user-scalable=no).
        max_scale_blocks.unwrap_or(false)
    } else {
        user_scalable_blocks.unwrap_or(false) || max_scale_blocks.unwrap_or(false)
    }
}

impl MetaRefresh {
    pub const META: RuleMeta = RuleMeta {
        id: "meta-refresh",
        impact: Impact::Critical,
        wcag: "2.2.1 (A)",
        tags: &[
            "cat.time-and-media",
            "wcag2a",
            "wcag221",
            "TTv5",
            "TT8.a",
            "EN-301-549",
            "EN-9.2.2.1",
            "RGAAv4",
            "RGAA-13.1.2",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Timed <meta http-equiv=\"refresh\"> must not be used",
        description: "Avoid <meta http-equiv=\"refresh\"> with a non-zero delay, which disorients users and screen readers.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/timing-adjustable.html",
    };
}

/// Parses `<meta http-equiv="refresh" content="...">` delay per HTML5 §4.2.5.3.
/// Delay must begin with an ASCII digit (leading `+` or `-` is invalid).
fn parse_meta_refresh_delay(content: &str) -> Option<f64> {
    let trimmed = content.trim_start();
    if !trimmed.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        return None;
    }
    let delay_token = trimmed.split([';', ',']).next().map_or("", str::trim);
    let num_part = delay_token.split_ascii_whitespace().next()?;
    if !num_part.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return None;
    }
    num_part.parse::<f64>().ok().filter(|n| n.is_finite())
}

/// Per HTML5 spec, only the first valid `<meta http-equiv="refresh">` takes effect.
fn first_valid_meta_refresh(doc: &Document) -> Option<(NodeId, f64)> {
    for el in doc.elements() {
        if el.tag != "meta" {
            continue;
        }
        if !el
            .attr("http-equiv")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("refresh"))
        {
            continue;
        }
        let Some(content) = el.attr("content") else {
            continue;
        };
        if let Some(delay) = parse_meta_refresh_delay(content) {
            return Some((el.id, delay));
        }
    }
    None
}

impl Rule for MetaRefresh {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        if let Some((id, delay)) = first_valid_meta_refresh(doc)
            && delay > 0.0
            && delay <= 72_000.0
        {
            out.push(Violation::from_node(&self.meta(), doc, id));
        }
    }
}

impl HtmlXmlLangMismatch {
    pub const META: RuleMeta = RuleMeta {
        id: "html-xml-lang-mismatch",
        impact: Impact::Moderate,
        wcag: "3.1.1 (A)",
        tags: &[
            "cat.language",
            "wcag2a",
            "wcag311",
            "EN-301-549",
            "EN-9.3.1.1",
            "ACT",
            "RGAAv4",
            "RGAA-8.3.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "HTML elements with lang and xml:lang must have the same base language",
        description: "Ensure that <html> elements with both valid lang and xml:lang attributes agree on the primary language subtag.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/language-of-page.html",
    };
}

impl Rule for HtmlXmlLangMismatch {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let Some(html_el) = doc.elements().find(|n| n.tag == "html") else {
            return;
        };
        let Some(lang) = html_el
            .attr("lang")
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return;
        };
        let Some(xml_lang) = html_el
            .attr("xml:lang")
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return;
        };
        if !is_valid_bcp47_primary(lang) || !is_valid_bcp47_primary(xml_lang) {
            return;
        }
        let primary_lang = lang.split('-').next().unwrap_or("");
        let primary_xml = xml_lang.split('-').next().unwrap_or("");
        if !primary_lang.eq_ignore_ascii_case(primary_xml) {
            out.push(Violation::with_detail(
                &self.meta(),
                doc,
                html_el.id,
                format!(
                    "<html> lang='{lang}' and xml:lang='{xml_lang}' have different primary language subtags."
                ),
            ));
        }
    }
}

impl ValidLang {
    pub const META: RuleMeta = RuleMeta {
        id: "valid-lang",
        impact: Impact::Serious,
        wcag: "3.1.2 (AA)",
        tags: &[
            "cat.language",
            "wcag2aa",
            "wcag312",
            "TTv5",
            "TT11.b",
            "EN-301-549",
            "EN-9.3.1.2",
            "ACT",
            "RGAAv4",
            "RGAA-8.8.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "lang attribute must have a valid value",
        description: "Ensure every lang attribute has a valid BCP 47 language tag.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/language-of-parts.html",
    };
}

impl Rule for ValidLang {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag == "html" || !doc.is_rendered(el.id) {
                continue;
            }
            if !owns_visible_text(doc, el.id) {
                continue;
            }
            for attr_name in ["lang", "xml:lang"] {
                if let Some(raw) = el.attr(attr_name) {
                    if raw.is_empty() {
                        continue;
                    }
                    if raw.trim().is_empty() || !is_valid_bcp47_primary(raw) {
                        out.push(Violation::with_detail(
                            &self.meta(),
                            doc,
                            el.id,
                            format!(
                                "Invalid BCP 47 language tag '{raw}' in {attr_name} attribute."
                            ),
                        ));
                        break;
                    }
                }
            }
        }
    }
}

fn owns_visible_text(doc: &Document, id: NodeId) -> bool {
    let Some(node) = doc.node(id) else {
        return false;
    };
    if node
        .attr("aria-label")
        .is_some_and(|v| !v.trim().is_empty())
        || node.attr("title").is_some_and(|v| !v.trim().is_empty())
        || (matches!(node.tag.as_str(), "img" | "area" | "input")
            && node.attr("alt").is_some_and(|v| !v.trim().is_empty()))
    {
        return true;
    }
    for &cid in &node.children {
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if child.is_text() && !child.text.trim().is_empty() {
            return true;
        }
        if !child.is_element() || !doc.is_rendered(cid) {
            continue;
        }
        // A descendant with its own non-empty lang attribute overrides this element's lang.
        let overrides_lang = child
            .attr("lang")
            .or_else(|| child.attr("xml:lang"))
            .is_some_and(|v| !v.is_empty());
        if overrides_lang {
            continue;
        }
        if owns_visible_text(doc, cid) {
            return true;
        }
    }
    false
}

impl MetaRefreshNoExceptions {
    pub const META: RuleMeta = RuleMeta {
        id: "meta-refresh-no-exceptions",
        impact: Impact::Critical,
        wcag: "2.2.4 / 3.2.5 (AAA)",
        tags: &["cat.time-and-media", "wcag2aaa", "wcag224", "wcag325"],
        enabled_by_default: false,
        needs_review: false,
        help: "Delayed refresh must not be used",
        description: "Ensure <meta http-equiv=\"refresh\"> is not used for any delayed refresh (no 20-hour exception).",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/interruptions.html",
    };
}

impl Rule for MetaRefreshNoExceptions {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        if let Some((id, delay)) = first_valid_meta_refresh(doc)
            && delay > 0.0
        {
            out.push(Violation::from_node(&self.meta(), doc, id));
        }
    }
}

impl MetaViewportLarge {
    pub const META: RuleMeta = RuleMeta {
        id: "meta-viewport-large",
        impact: Impact::Minor,
        wcag: "1.4.4 (Best Practice)",
        tags: &["cat.sensory-and-visual-cues", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Users should be able to zoom and scale the text up to 500%",
        description: "Ensure <meta name=\"viewport\"> does not disable zooming and allows maximum-scale of at least 5.0.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/resize-text.html",
    };
}

impl Rule for MetaViewportLarge {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "meta" {
                continue;
            }
            if !el
                .attr("name")
                .is_some_and(|n| n.trim().eq_ignore_ascii_case("viewport"))
            {
                continue;
            }
            let Some(content) = el.attr("content") else {
                continue;
            };
            if viewport_restricts_zoom(content, 5.0, Some(2.0)) {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl Bypass {
    pub const META: RuleMeta = RuleMeta {
        id: "bypass",
        impact: Impact::Serious,
        wcag: "2.4.1 (A)",
        tags: &[
            "cat.keyboard",
            "wcag2a",
            "wcag241",
            "section508",
            "section508.22.o",
            "TTv5",
            "TT9.a",
            "EN-301-549",
            "EN-9.2.4.1",
            "RGAAv4",
            "RGAA-12.7.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Page must have means to bypass repeated blocks",
        description: "Provide a <main> landmark, heading, or same-page skip link so keyboard users can bypass repeated navigation.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/bypass-blocks.html",
    };
}

impl Rule for Bypass {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let Some(html_el) = doc.elements().find(|n| n.tag == "html") else {
            return;
        };
        let Some(body_el) = doc.elements().find(|n| n.tag == "body") else {
            return;
        };
        if !has_visible_body_content(doc, body_el.id) {
            return;
        }
        // Only require a bypass mechanism when a repeated/navigation block
        // appears before main content in document order.
        let repeated_blocks: Vec<NodeId> = doc
            .elements()
            .filter(|el| {
                doc.is_in_accessibility_tree(el.id)
                    && doc.ancestors(el.id).contains(&body_el.id)
                    && is_repeated_block(doc, el)
                    && has_visible_body_content(doc, el.id)
            })
            .map(|el| el.id)
            .collect();
        let Some(&first_block_id) = repeated_blocks.first() else {
            return;
        };
        let is_inside_repeated_block = |id: NodeId| -> bool {
            repeated_blocks.contains(&id)
                || doc
                    .ancestors(id)
                    .into_iter()
                    .any(|aid| repeated_blocks.contains(&aid))
        };
        // Check whether any rendered content appears outside repeated blocks
        // after the first repeated block (otherwise the block is after main content).
        let has_content_after_block = doc.elements().any(|el| {
            el.id > first_block_id
                && doc.is_rendered(el.id)
                && doc.ancestors(el.id).contains(&body_el.id)
                && !is_inside_repeated_block(el.id)
                && (doc.has_direct_text(el.id)
                    || matches!(
                        el.tag.as_str(),
                        "img" | "svg" | "input" | "iframe" | "video" | "canvas"
                    ))
        });
        if !has_content_after_block {
            return;
        }
        let has_bypass = doc.elements().any(|el| {
            if !doc.is_in_accessibility_tree(el.id) {
                return false;
            }
            let role = effective_role(doc, el);
            if role == Some("main") && !is_inside_repeated_block(el.id) {
                return true;
            }
            if role == Some("heading") && el.id > first_block_id && !is_inside_repeated_block(el.id)
            {
                return true;
            }
            if el.tag == "a"
                && let Some(href) = el.attr("href").map(str::trim)
                && let Some(target_id) = href.strip_prefix('#').filter(|id| !id.is_empty())
                && let Some(tid) = doc.find_by_id(target_id)
                && doc.is_in_accessibility_tree(tid)
                && tid > first_block_id
            {
                // Valid if the target is outside repeated blocks, or at the trailing edge
                // of a repeated block with no remaining visible/focusable content after it.
                if !is_inside_repeated_block(tid) {
                    return true;
                }
                let has_later_block_content = doc.elements().any(|other| {
                    other.id >= tid
                        && is_inside_repeated_block(other.id)
                        && doc.is_in_accessibility_tree(other.id)
                        && (doc.has_direct_text(other.id) || is_sequentially_focusable(other))
                });
                if !has_later_block_content {
                    return true;
                }
            }
            false
        });
        if !has_bypass {
            // If an interactive JS `onclick` control precedes the first repeated block,
            // static analysis cannot prove whether it hides/skips the block — mark incomplete.
            let has_js_bypass_candidate = doc.elements().any(|el| {
                el.id < first_block_id
                    && doc.is_in_accessibility_tree(el.id)
                    && doc.ancestors(el.id).contains(&body_el.id)
                    && el.attr("onclick").is_some_and(|v| !v.trim().is_empty())
                    && (is_sequentially_focusable(el)
                        || matches!(el.tag.as_str(), "button" | "a")
                        || matches!(effective_role(doc, el), Some("button" | "link")))
            });
            let mut v = Violation::from_node(&self.meta(), doc, html_el.id);
            if has_js_bypass_candidate {
                v = v.into_incomplete();
            }
            out.push(v);
        }
    }
}

fn is_repeated_block(doc: &Document, node: &Node) -> bool {
    matches!(node.tag.as_str(), "nav" | "aside" | "header")
        || matches!(
            effective_role(doc, node),
            Some("navigation" | "banner" | "complementary")
        )
}

fn has_visible_body_content(doc: &Document, id: NodeId) -> bool {
    let Some(node) = doc.node(id) else {
        return false;
    };
    for &cid in &node.children {
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if child.is_text() && !child.text.trim().is_empty() {
            return true;
        }
        if child.is_element()
            && doc.is_in_accessibility_tree(cid)
            && (matches!(
                child.tag.as_str(),
                "img" | "svg" | "input" | "iframe" | "video" | "canvas"
            ) || has_visible_body_content(doc, cid))
        {
            return true;
        }
    }
    false
}

impl SkipLink {
    pub const META: RuleMeta = RuleMeta {
        id: "skip-link",
        impact: Impact::Moderate,
        wcag: "2.4.1 (Best Practice)",
        tags: &["cat.keyboard", "best-practice", "RGAAv4", "RGAA-12.7.1"],
        enabled_by_default: true,
        needs_review: false,
        help: "The skip-link target should exist and be focusable",
        description: "Ensure the first focusable skip link points to an existing, visible target ID in the document.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/bypass-blocks.html",
    };
}

impl Rule for SkipLink {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let Some(body_id) = doc.elements().find(|n| n.tag == "body").map(|n| n.id) else {
            return;
        };
        let first_focusable = doc.elements().find(|el| {
            doc.is_rendered(el.id)
                && is_sequentially_focusable(el)
                && doc.ancestors(el.id).contains(&body_id)
        });
        let Some(link) = first_focusable else {
            return;
        };
        if link.tag != "a" {
            return;
        }
        let Some(href) = link.attr("href").map(str::trim) else {
            return;
        };
        let Some(target_id) = href.strip_prefix('#').filter(|id| !id.is_empty()) else {
            return;
        };
        let valid_target = doc
            .find_by_id(target_id)
            .is_some_and(|tid| doc.is_in_accessibility_tree(tid));
        if !valid_target {
            out.push(Violation::with_detail(
                &self.meta(),
                doc,
                link.id,
                format!("Skip link target '#{target_id}' is missing or hidden."),
            ));
        }
    }
}

impl AvoidInlineSpacing {
    pub const META: RuleMeta = RuleMeta {
        id: "avoid-inline-spacing",
        impact: Impact::Serious,
        wcag: "1.4.12 (AA)",
        tags: &[
            "cat.structure",
            "wcag21aa",
            "wcag1412",
            "EN-301-549",
            "EN-9.1.4.12",
            "ACT",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Inline text spacing must be adjustable with custom stylesheets",
        description: "Avoid !important on inline line-height, letter-spacing, or word-spacing styles so users can override text spacing.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/text-spacing.html",
    };
}

impl Rule for AvoidInlineSpacing {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let style_font_sizes = StyleFontSizes::from_doc(doc);
        let mut reported: HashSet<NodeId> = HashSet::new();

        for el in doc.elements() {
            if !doc.is_rendered(el.id) || !doc.has_direct_text(el.id) {
                continue;
            }
            // Build ancestor chain from root down to `el`.
            let mut chain = doc.ancestors(el.id);
            chain.reverse();
            chain.push(el.id);

            // Skip elements that are hidden off-screen (e.g. position: absolute; left: -9999px).
            if chain
                .iter()
                .any(|&nid| doc.node(nid).is_some_and(is_offscreen_hidden))
            {
                continue;
            }

            // Cascade font-size and inline spacing declarations from ancestors to `el`.
            let mut cur_font_size = 16.0_f64;
            let mut eff_line_height: Option<EffectiveSpacing> = None;
            let mut eff_letter_spacing: Option<EffectiveSpacing> = None;
            let mut eff_word_spacing: Option<EffectiveSpacing> = None;

            for &nid in &chain {
                let Some(node) = doc.node(nid) else {
                    continue;
                };
                if !node.is_element() {
                    continue;
                }
                cur_font_size = resolve_node_font_size(node, cur_font_size, &style_font_sizes);
                if let Some(style) = node.attr("style") {
                    apply_inline_spacing(
                        nid,
                        style,
                        cur_font_size,
                        &mut eff_line_height,
                        &mut eff_letter_spacing,
                        &mut eff_word_spacing,
                    );
                }
            }

            let checks = [
                ("line-height", eff_line_height, 1.5),
                ("letter-spacing", eff_letter_spacing, 0.12),
                ("word-spacing", eff_word_spacing, 0.16),
            ];
            for (prop_name, effective, min_ratio) in checks {
                let Some(eff) = effective else {
                    continue;
                };
                if !eff.important {
                    continue;
                }
                let ratio = match eff.value {
                    ResolvedSpacing::Ratio(r) => r,
                    ResolvedSpacing::Px(px) => px / cur_font_size,
                };
                if ratio < min_ratio - 1e-6 && reported.insert(eff.node_id) {
                    out.push(Violation::with_detail(
                        &self.meta(),
                        doc,
                        eff.node_id,
                        format!(
                            "Inline style sets '{prop_name}' with !important below WCAG 1.4.12 minimum."
                        ),
                    ));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct EffectiveSpacing {
    node_id: NodeId,
    important: bool,
    value: ResolvedSpacing,
}

#[derive(Debug, Clone, Copy)]
enum ResolvedSpacing {
    /// Unitless ratio (e.g. `line-height: 1.5` or `normal` -> `0.0`).
    Ratio(f64),
    /// Computed length in CSS pixels (inherits as computed px).
    Px(f64),
}

#[derive(Debug, Clone, Copy)]
enum ParsedSpacing {
    Value(ResolvedSpacing),
    Inherit,
}

#[derive(Default)]
struct StyleFontSizes {
    tags: BTreeMap<String, f64>,
    classes: BTreeMap<String, f64>,
    ids: BTreeMap<String, f64>,
}

impl StyleFontSizes {
    fn from_doc(doc: &Document) -> Self {
        let mut map = Self::default();
        for el in doc.elements() {
            if el.tag != "style" {
                continue;
            }
            let css = strip_css_comments_doc(&collect_raw_text(doc, el.id));
            for rule in css.split('}') {
                let Some((sel_list, decls)) = rule.split_once('{') else {
                    continue;
                };
                let Some(fs_px) = parse_inline_font_size_px(decls, 16.0) else {
                    continue;
                };
                for sel in sel_list.split(',') {
                    let s = sel.trim();
                    if let Some(id_sel) = s.strip_prefix('#')
                        && !id_sel.is_empty()
                        && id_sel
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                    {
                        map.ids.insert(id_sel.to_owned(), fs_px);
                    } else if let Some(cls) = s.strip_prefix('.')
                        && !cls.is_empty()
                        && cls
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                    {
                        map.classes.insert(cls.to_ascii_lowercase(), fs_px);
                    } else if !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric()) {
                        map.tags.insert(s.to_ascii_lowercase(), fs_px);
                    }
                }
            }
        }
        map
    }
}

fn strip_css_comments_doc(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find("*/") {
            rest = &after[end + 2..];
        } else {
            return out;
        }
    }
    out.push_str(rest);
    out
}

fn resolve_node_font_size(node: &Node, parent_px: f64, sheet: &StyleFontSizes) -> f64 {
    if let Some(fs) = node.computed.as_ref().and_then(|c| c.font_size_px)
        && fs > 0.0
    {
        return f64::from(fs);
    }
    let mut size = parent_px;
    if let Some(&px) = sheet.tags.get(&node.tag) {
        size = px;
    }
    if let Some(classes) = node.attr("class") {
        for cls in classes.split_ascii_whitespace() {
            if let Some(&px) = sheet.classes.get(&cls.to_ascii_lowercase()) {
                size = px;
            }
        }
    }
    if let Some(id_val) = node.id_attr()
        && let Some(&px) = sheet.ids.get(id_val)
    {
        size = px;
    }
    if let Some(style) = node.attr("style")
        && let Some(px) = parse_inline_font_size_px(style, parent_px)
    {
        size = px;
    }
    size
}

fn is_offscreen_hidden(node: &Node) -> bool {
    let Some(style) = node.attr("style") else {
        return false;
    };
    let mut pos_abs = false;
    let mut neg_offset = false;
    for decl in style.split(';') {
        let Some((prop, val)) = decl.split_once(':') else {
            continue;
        };
        let p = prop.trim().to_ascii_lowercase();
        let v = val
            .split('!')
            .next()
            .map_or("", str::trim)
            .to_ascii_lowercase();
        match p.as_str() {
            "position" if matches!(v.as_str(), "absolute" | "fixed") => pos_abs = true,
            "left" | "top" => {
                if let Some(px) = parse_css_length_px(&v, 16.0)
                    && px <= -100.0
                {
                    neg_offset = true;
                }
            }
            "clip-path" if v.contains("inset(50%") || v.contains("inset(100%") => return true,
            "clip" if v.starts_with("rect(0") || v.starts_with("rect(1px") => return true,
            _ => {}
        }
    }
    pos_abs && neg_offset
}

fn apply_inline_spacing(
    nid: NodeId,
    style: &str,
    font_size_px: f64,
    eff_line: &mut Option<EffectiveSpacing>,
    eff_letter: &mut Option<EffectiveSpacing>,
    eff_word: &mut Option<EffectiveSpacing>,
) {
    let mut line_norm: Option<ParsedSpacing> = None;
    let mut line_imp: Option<ParsedSpacing> = None;
    let mut letter_norm: Option<ParsedSpacing> = None;
    let mut letter_imp: Option<ParsedSpacing> = None;
    let mut word_norm: Option<ParsedSpacing> = None;
    let mut word_imp: Option<ParsedSpacing> = None;

    for decl in style.split(';') {
        let Some((prop, val)) = decl.split_once(':') else {
            continue;
        };
        let is_imp = has_important_flag(val);
        let prop_lower = prop.trim().to_ascii_lowercase();
        let raw_val = val.split('!').next().map_or("", str::trim);
        match prop_lower.as_str() {
            "line-height" => {
                if let Some(p) = parse_spacing_value("line-height", raw_val, font_size_px) {
                    if is_imp {
                        line_imp = Some(p);
                    } else {
                        line_norm = Some(p);
                    }
                }
            }
            "letter-spacing" => {
                if let Some(p) = parse_spacing_value("letter-spacing", raw_val, font_size_px) {
                    if is_imp {
                        letter_imp = Some(p);
                    } else {
                        letter_norm = Some(p);
                    }
                }
            }
            "word-spacing" => {
                if let Some(p) = parse_spacing_value("word-spacing", raw_val, font_size_px) {
                    if is_imp {
                        word_imp = Some(p);
                    } else {
                        word_norm = Some(p);
                    }
                }
            }
            _ => {}
        }
    }

    let apply_one = |eff: &mut Option<EffectiveSpacing>,
                     imp: Option<ParsedSpacing>,
                     norm: Option<ParsedSpacing>| {
        if let Some(p) = imp {
            match p {
                ParsedSpacing::Value(value) => {
                    *eff = Some(EffectiveSpacing {
                        node_id: nid,
                        important: true,
                        value,
                    });
                }
                ParsedSpacing::Inherit => {}
            }
        } else if let Some(p) = norm {
            match p {
                ParsedSpacing::Value(value) => {
                    *eff = Some(EffectiveSpacing {
                        node_id: nid,
                        important: false,
                        value,
                    });
                }
                ParsedSpacing::Inherit => {}
            }
        }
    };

    apply_one(eff_line, line_imp, line_norm);
    apply_one(eff_letter, letter_imp, letter_norm);
    apply_one(eff_word, word_imp, word_norm);
}

fn parse_inline_font_size_px(style: &str, base_px: f64) -> Option<f64> {
    let mut normal_fs: Option<f64> = None;
    let mut important_fs: Option<f64> = None;
    for decl in style.split(';') {
        let Some((prop, val)) = decl.split_once(':') else {
            continue;
        };
        if !prop.trim().eq_ignore_ascii_case("font-size") {
            continue;
        }
        let is_imp = has_important_flag(val);
        let raw = val.split('!').next().map_or("", str::trim);
        if let Some(px) = parse_css_length_px(raw, base_px)
            && px > 0.0
        {
            if is_imp {
                important_fs = Some(px);
            } else {
                normal_fs = Some(px);
            }
        }
    }
    important_fs.or(normal_fs)
}

fn parse_css_length_px(raw: &str, base_px: f64) -> Option<f64> {
    let s = raw.trim().to_ascii_lowercase();
    let parse_num = |prefix: &str| prefix.trim().parse::<f64>().ok().filter(|n| n.is_finite());
    if let Some(v) = s.strip_suffix("px") {
        parse_num(v)
    } else if let Some(v) = s.strip_suffix("rem") {
        parse_num(v).map(|n| n * 16.0)
    } else if let Some(v) = s.strip_suffix("em") {
        parse_num(v).map(|n| n * base_px)
    } else if let Some(v) = s.strip_suffix('%') {
        parse_num(v).map(|n| (n / 100.0) * base_px)
    } else if let Some(v) = s.strip_suffix("pt") {
        parse_num(v).map(|n| n * (96.0 / 72.0))
    } else if let Some(v) = s.strip_suffix("pc") {
        parse_num(v).map(|n| n * 16.0)
    } else if let Some(v) = s.strip_suffix("in") {
        parse_num(v).map(|n| n * 96.0)
    } else if let Some(v) = s.strip_suffix("cm") {
        parse_num(v).map(|n| n * (96.0 / 2.54))
    } else if let Some(v) = s.strip_suffix("mm") {
        parse_num(v).map(|n| n * (96.0 / 25.4))
    } else {
        None
    }
}

fn parse_spacing_value(prop: &str, raw: &str, font_size_px: f64) -> Option<ParsedSpacing> {
    let s = raw.trim().to_ascii_lowercase();
    if s.is_empty() {
        return None;
    }
    match s.as_str() {
        "inherit" | "unset" | "revert" => return Some(ParsedSpacing::Inherit),
        "normal" | "initial" => return Some(ParsedSpacing::Value(ResolvedSpacing::Ratio(0.0))),
        _ => {}
    }
    if let Ok(n) = s.parse::<f64>()
        && n.is_finite()
    {
        if prop == "line-height" {
            return (n >= 0.0).then_some(ParsedSpacing::Value(ResolvedSpacing::Ratio(n)));
        }
        return (n == 0.0).then_some(ParsedSpacing::Value(ResolvedSpacing::Ratio(0.0)));
    }
    let px = parse_css_length_px(&s, font_size_px)?;
    if prop == "line-height" && px < 0.0 {
        return None;
    }
    Some(ParsedSpacing::Value(ResolvedSpacing::Px(px)))
}

fn has_important_flag(val: &str) -> bool {
    val.split_once('!')
        .is_some_and(|(_, after)| after.trim().eq_ignore_ascii_case("important"))
}
