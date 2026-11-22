//! Landmark and region rules: `landmark-banner-is-top-level`,
//! `landmark-complementary-is-top-level`, `landmark-contentinfo-is-top-level`,
//! `landmark-main-is-top-level`, `landmark-no-duplicate-banner`,
//! `landmark-no-duplicate-contentinfo`, `landmark-no-duplicate-main`,
//! `landmark-unique`, and `region`.

use std::collections::BTreeMap;

use crate::accname::accessible_name;
use crate::aria::explicit_role;
use crate::ir::{Document, Node, NodeId};
use crate::rule::{Impact, Rule, RuleMeta, Violation};

pub struct LandmarkBannerIsTopLevel;
pub struct LandmarkComplementaryIsTopLevel;
pub struct LandmarkContentinfoIsTopLevel;
pub struct LandmarkMainIsTopLevel;
pub struct LandmarkNoDuplicateBanner;
pub struct LandmarkNoDuplicateContentinfo;
pub struct LandmarkNoDuplicateMain;
pub struct LandmarkUnique;
pub struct Region;

/// Returns `true` if `node` has an HTML sectioning ancestor (`<article>`,
/// `<aside>`, `<main>`, `<nav>`, or `<section>`), which suppresses implicit
/// `banner` / `contentinfo` landmark roles on `<header>` and `<footer>` per HTML-AAM.
fn has_sectioning_ancestor(doc: &Document, node: &Node) -> bool {
    doc.ancestors(node.id).into_iter().any(|anc_id| {
        doc.node(anc_id).is_some_and(|anc| {
            matches!(
                anc.tag.as_str(),
                "article" | "aside" | "main" | "nav" | "section"
            )
        })
    })
}

/// Returns the WAI-ARIA landmark role of `node`, if any, following HTML-AAM rules:
/// - `<header>` is `banner` only when not inside `<article>`, `<aside>`, `<main>`, `<nav>`, `<section>`.
/// - `<footer>` is `contentinfo` only when not inside those same sectioning elements.
/// - `<aside>` is `complementary`, `<main>` is `main`, `<nav>` is `navigation`, `<search>` is `search`.
/// - `<section>` and `<form>` are landmarks only when they have an accessible name.
pub(crate) fn landmark_role(doc: &Document, node: &Node) -> Option<&'static str> {
    if let Some(role) = explicit_role(node) {
        return match role {
            "banner" => Some("banner"),
            "complementary" => Some("complementary"),
            "contentinfo" => Some("contentinfo"),
            "form" => Some("form"),
            "main" => Some("main"),
            "navigation" => Some("navigation"),
            "region" => Some("region"),
            "search" => Some("search"),
            _ => None,
        };
    }

    match node.tag.as_str() {
        "header" => (!has_sectioning_ancestor(doc, node)).then_some("banner"),
        "footer" => (!has_sectioning_ancestor(doc, node)).then_some("contentinfo"),
        "aside" => Some("complementary"),
        "main" => Some("main"),
        "nav" => Some("navigation"),
        "search" => Some("search"),
        "section" => (!accessible_name(doc, node.id).is_empty()).then_some("region"),
        "form" => (!accessible_name(doc, node.id).is_empty()).then_some("form"),
        _ => None,
    }
}

fn has_landmark_ancestor(doc: &Document, id: NodeId) -> bool {
    doc.ancestors(id).into_iter().any(|anc_id| {
        doc.node(anc_id).is_some_and(|anc| {
            doc.is_in_accessibility_tree(anc_id) && landmark_role(doc, anc).is_some()
        })
    })
}

fn check_landmark_top_level(
    doc: &Document,
    meta: &RuleMeta,
    target_role: &str,
    out: &mut Vec<Violation>,
) {
    for el in doc.elements() {
        if !doc.is_in_accessibility_tree(el.id) {
            continue;
        }
        if landmark_role(doc, el) == Some(target_role) && has_landmark_ancestor(doc, el.id) {
            out.push(Violation::from_node(meta, doc, el.id));
        }
    }
}

fn check_no_duplicate_landmark(
    doc: &Document,
    meta: &RuleMeta,
    target_role: &str,
    out: &mut Vec<Violation>,
) {
    let nodes: Vec<NodeId> = doc
        .elements()
        .filter(|el| {
            doc.is_in_accessibility_tree(el.id) && landmark_role(doc, el) == Some(target_role)
        })
        .map(|el| el.id)
        .collect();
    if nodes.len() > 1 {
        for &nid in &nodes[1..] {
            out.push(Violation::with_detail(
                meta,
                doc,
                nid,
                format!(
                    "Document has {} '{target_role}' landmarks; at most one is recommended.",
                    nodes.len()
                ),
            ));
        }
    }
}

impl LandmarkBannerIsTopLevel {
    pub const META: RuleMeta = RuleMeta {
        id: "landmark-banner-is-top-level",
        impact: Impact::Moderate,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.semantics", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Banner landmark should not be contained in another landmark",
        description: "Ensure the banner landmark is at top level and not nested inside another landmark.",
        docs_url: "https://www.w3.org/WAI/ARIA/apg/patterns/landmarks/examples/banner.html",
    };
}

impl Rule for LandmarkBannerIsTopLevel {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_landmark_top_level(doc, &self.meta(), "banner", out);
    }
}

impl LandmarkComplementaryIsTopLevel {
    pub const META: RuleMeta = RuleMeta {
        id: "landmark-complementary-is-top-level",
        impact: Impact::Moderate,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.semantics", "best-practice", "deprecated"],
        enabled_by_default: false,
        needs_review: false,
        help: "Aside should not be contained in another landmark",
        description: "Ensure the complementary landmark or aside is at top level and not nested inside another landmark.",
        docs_url: "https://www.w3.org/WAI/ARIA/apg/patterns/landmarks/examples/complementary.html",
    };
}

impl Rule for LandmarkComplementaryIsTopLevel {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_landmark_top_level(doc, &self.meta(), "complementary", out);
    }
}

impl LandmarkContentinfoIsTopLevel {
    pub const META: RuleMeta = RuleMeta {
        id: "landmark-contentinfo-is-top-level",
        impact: Impact::Moderate,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.semantics", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Contentinfo landmark should not be contained in another landmark",
        description: "Ensure the contentinfo landmark is at top level and not nested inside another landmark.",
        docs_url: "https://www.w3.org/WAI/ARIA/apg/patterns/landmarks/examples/contentinfo.html",
    };
}

impl Rule for LandmarkContentinfoIsTopLevel {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_landmark_top_level(doc, &self.meta(), "contentinfo", out);
    }
}

impl LandmarkMainIsTopLevel {
    pub const META: RuleMeta = RuleMeta {
        id: "landmark-main-is-top-level",
        impact: Impact::Moderate,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.semantics", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Main landmark should not be contained in another landmark",
        description: "Ensure the main landmark is at top level and not nested inside another landmark.",
        docs_url: "https://www.w3.org/WAI/ARIA/apg/patterns/landmarks/examples/main.html",
    };
}

impl Rule for LandmarkMainIsTopLevel {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_landmark_top_level(doc, &self.meta(), "main", out);
    }
}

impl LandmarkNoDuplicateBanner {
    pub const META: RuleMeta = RuleMeta {
        id: "landmark-no-duplicate-banner",
        impact: Impact::Moderate,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.semantics", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Document should not have more than one banner landmark",
        description: "Ensure the document has at most one banner landmark.",
        docs_url: "https://www.w3.org/WAI/ARIA/apg/patterns/landmarks/examples/banner.html",
    };
}

impl Rule for LandmarkNoDuplicateBanner {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_no_duplicate_landmark(doc, &self.meta(), "banner", out);
    }
}

impl LandmarkNoDuplicateContentinfo {
    pub const META: RuleMeta = RuleMeta {
        id: "landmark-no-duplicate-contentinfo",
        impact: Impact::Moderate,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.semantics", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Document should not have more than one contentinfo landmark",
        description: "Ensure the document has at most one contentinfo landmark.",
        docs_url: "https://www.w3.org/WAI/ARIA/apg/patterns/landmarks/examples/contentinfo.html",
    };
}

impl Rule for LandmarkNoDuplicateContentinfo {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_no_duplicate_landmark(doc, &self.meta(), "contentinfo", out);
    }
}

impl LandmarkNoDuplicateMain {
    pub const META: RuleMeta = RuleMeta {
        id: "landmark-no-duplicate-main",
        impact: Impact::Moderate,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.semantics", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Document should not have more than one main landmark",
        description: "Ensure the document has at most one main landmark.",
        docs_url: "https://www.w3.org/WAI/ARIA/apg/patterns/landmarks/examples/main.html",
    };
}

impl Rule for LandmarkNoDuplicateMain {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_no_duplicate_landmark(doc, &self.meta(), "main", out);
    }
}

impl LandmarkUnique {
    pub const META: RuleMeta = RuleMeta {
        id: "landmark-unique",
        impact: Impact::Moderate,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.semantics", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Landmarks should have a unique role or role/label/title (i.e. accessible name) combination",
        description: "When multiple landmarks share the same role, distinguish each with a unique accessible name.",
        docs_url: "https://www.w3.org/WAI/ARIA/apg/practices/landmark-regions/",
    };
}

impl Rule for LandmarkUnique {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let mut groups: BTreeMap<(&'static str, String), Vec<NodeId>> = BTreeMap::new();
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let Some(role) = landmark_role(doc, el) else {
                continue;
            };
            let key = (role, accessible_name(doc, el.id).to_ascii_lowercase());
            groups.entry(key).or_default().push(el.id);
        }

        for ((role, name), nodes) in groups {
            if nodes.len() <= 1 {
                continue;
            }
            let detail = if name.is_empty() {
                format!(
                    "Multiple '{role}' landmarks exist without unique accessible names (add aria-label or aria-labelledby)."
                )
            } else {
                format!(
                    "Landmark role '{role}' with accessible name '{name}' is duplicated across multiple landmarks."
                )
            };
            out.push(Violation::with_detail(&self.meta(), doc, nodes[0], detail));
        }
    }
}

impl Region {
    pub const META: RuleMeta = RuleMeta {
        id: "region",
        impact: Impact::Moderate,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.keyboard", "best-practice", "RGAAv4", "RGAA-9.2.1"],
        enabled_by_default: true,
        needs_review: false,
        help: "All page content should be contained by landmarks",
        description: "Ensure all visible page content is contained within a landmark region.",
        docs_url: "https://www.w3.org/WAI/ARIA/apg/practices/landmark-regions/",
    };
}

impl Rule for Region {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let Some(body) = doc.elements().find(|n| n.tag == "body") else {
            return;
        };
        if !doc.is_in_accessibility_tree(body.id) {
            return;
        }
        if doc.has_direct_text(body.id) {
            out.push(Violation::from_node(&self.meta(), doc, body.id));
        }
        let mut flags = vec![0u8; doc.nodes.len()];
        let mut computed = vec![false; doc.nodes.len()];
        for &cid in &body.children {
            compute_region_flags(doc, cid, &mut flags, &mut computed);
            check_region_subtree(doc, cid, &flags, &self.meta(), out);
        }
    }
}

const REGION_HAS_LANDMARK: u8 = 1;
const REGION_HAS_CONTENT: u8 = 2;

fn is_skip_link(doc: &Document, node: &Node) -> bool {
    node.tag == "a"
        && node.attr("href").map(str::trim).is_some_and(|href| {
            href.starts_with('#') && href.len() > 1 && doc.find_by_id(&href[1..]).is_some()
        })
}

fn is_region_exempt_container(doc: &Document, node: &Node) -> bool {
    landmark_role(doc, node).is_some()
        || matches!(explicit_role(node), Some("dialog" | "alertdialog"))
        || node.tag == "dialog"
}

fn is_leaf_content_element(node: &Node) -> bool {
    match node.tag.as_str() {
        "img" | "svg" | "button" | "select" | "textarea" | "iframe" | "video" | "audio"
        | "canvas" | "object" => true,
        "input" => !node
            .attr("type")
            .is_some_and(|t| t.trim().eq_ignore_ascii_case("hidden")),
        _ => false,
    }
}

fn compute_region_flags(doc: &Document, id: NodeId, flags: &mut [u8], computed: &mut [bool]) -> u8 {
    if computed.get(id).copied().unwrap_or(true) {
        return flags.get(id).copied().unwrap_or(0);
    }
    computed[id] = true;
    let Some(node) = doc.node(id) else {
        return 0;
    };
    if !node.is_element() || !doc.is_in_accessibility_tree(id) {
        return 0;
    }
    if is_region_exempt_container(doc, node) {
        flags[id] = REGION_HAS_LANDMARK;
        return REGION_HAS_LANDMARK;
    }
    if is_skip_link(doc, node) {
        return 0;
    }
    let mut node_flags = if doc.has_direct_text(id) || is_leaf_content_element(node) {
        REGION_HAS_CONTENT
    } else {
        0
    };
    for &cid in &node.children {
        node_flags |= compute_region_flags(doc, cid, flags, computed);
    }
    flags[id] = node_flags;
    node_flags
}

fn check_region_subtree(
    doc: &Document,
    id: NodeId,
    flags: &[u8],
    meta: &RuleMeta,
    out: &mut Vec<Violation>,
) {
    let Some(node) = doc.node(id) else {
        return;
    };
    if !node.is_element() || !doc.is_in_accessibility_tree(id) {
        return;
    }
    if is_region_exempt_container(doc, node) || is_skip_link(doc, node) {
        return;
    }
    let node_flags = flags.get(id).copied().unwrap_or(0);
    if (node_flags & REGION_HAS_LANDMARK) == 0 {
        if (node_flags & REGION_HAS_CONTENT) != 0 {
            out.push(Violation::from_node(meta, doc, id));
        }
        return;
    }
    if doc.has_direct_text(id) {
        out.push(Violation::from_node(meta, doc, id));
    }
    for &cid in &node.children {
        check_region_subtree(doc, cid, flags, meta, out);
    }
}
