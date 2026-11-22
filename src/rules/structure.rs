//! Structural and navigation rules: `heading-order`, `empty-heading`,
//! `landmark-one-main`, `page-has-heading-one`, `list`, `listitem`,
//! `definition-list`, `dlitem`, `tabindex`, `nested-interactive`,
//! `duplicate-id`, and `duplicate-id-active`.

use std::collections::{BTreeMap, HashSet};

use crate::accname::accessible_name;
use crate::aria::{
    effective_role, explicit_role, is_interactive_control, is_natively_focusable,
    is_sequentially_focusable,
};
use crate::ir::{Document, Node, NodeId};
use crate::rule::{Impact, Rule, RuleMeta, Violation};

pub struct HeadingOrder;
pub struct EmptyHeading;
pub struct LandmarkOneMain;
pub struct PageHasHeadingOne;
pub struct ListRule;
pub struct ListItem;
pub struct DefinitionList;
pub struct DlItem;
pub struct TabIndex;
pub struct NestedInteractive;
pub struct DuplicateId;
pub struct DuplicateIdActive;

fn heading_level(doc: &Document, node: &Node) -> Option<u8> {
    if effective_role(doc, node) != Some("heading") {
        return None;
    }
    if let Some(lvl) = node
        .attr("aria-level")
        .and_then(|v| v.trim().parse::<u8>().ok())
        .filter(|&n| (1..=6).contains(&n))
    {
        return Some(lvl);
    }
    match node.tag.as_str() {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

fn body_has_visible_content(doc: &Document) -> bool {
    let Some(body) = doc.elements().find(|n| n.tag == "body") else {
        return false;
    };
    has_visible_descendant(doc, body.id)
}

fn has_visible_descendant(doc: &Document, id: NodeId) -> bool {
    let mut visited = HashSet::new();
    visited.insert(id);
    let mut stack: Vec<NodeId> = doc.node(id).map_or_default(|n| n.children.clone());
    while let Some(cid) = stack.pop() {
        if !visited.insert(cid) {
            continue;
        }
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if child.is_text() && !child.text.trim().is_empty() {
            return true;
        }
        if child.is_element() && doc.is_in_accessibility_tree(cid) {
            if matches!(
                child.tag.as_str(),
                "img" | "svg" | "input" | "iframe" | "video" | "canvas"
            ) {
                return true;
            }
            stack.extend(&child.children);
        }
    }
    false
}

impl HeadingOrder {
    pub const META: RuleMeta = RuleMeta {
        id: "heading-order",
        impact: Impact::Moderate,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.semantics", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Heading levels should only increase by one",
        description: "Avoid skipping heading levels (for example, jumping from <h1> directly to <h3>).",
        docs_url: "https://www.w3.org/WAI/tutorials/page-structure/headings/",
    };
}

impl Rule for HeadingOrder {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let mut prev_level: Option<u8> = None;
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let Some(level) = heading_level(doc, el) else {
                continue;
            };
            if let Some(prev) = prev_level
                && level > prev + 1
            {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!("Heading level skipped from h{prev} to h{level}."),
                ));
            }
            prev_level = Some(level);
        }
    }
}

impl EmptyHeading {
    pub const META: RuleMeta = RuleMeta {
        id: "empty-heading",
        impact: Impact::Minor,
        wcag: "2.4.6 (AA)",
        tags: &["cat.name-role-value", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Headings must have discernible text",
        description: "Ensure every visible heading contains text or an accessible label.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/headings-and-labels.html",
    };
}

impl Rule for EmptyHeading {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if heading_level(doc, el).is_none() {
                continue;
            }
            if accessible_name(doc, el.id).is_empty() {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl LandmarkOneMain {
    pub const META: RuleMeta = RuleMeta {
        id: "landmark-one-main",
        impact: Impact::Moderate,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.semantics", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Document should have one main landmark",
        description: "Wrap primary page content in a <main> element (or role=\"main\").",
        docs_url: "https://www.w3.org/WAI/ARIA/apg/patterns/landmarks/examples/main.html",
    };
}

impl Rule for LandmarkOneMain {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        if !body_has_visible_content(doc) {
            return;
        }
        let Some(html_el) = doc.elements().find(|n| n.tag == "html") else {
            return;
        };
        let has_main = doc.elements().any(|el| {
            doc.is_in_accessibility_tree(el.id) && effective_role(doc, el) == Some("main")
        });
        if !has_main {
            out.push(Violation::from_node(&self.meta(), doc, html_el.id));
        }
    }
}

impl PageHasHeadingOne {
    pub const META: RuleMeta = RuleMeta {
        id: "page-has-heading-one",
        impact: Impact::Moderate,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.semantics", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Page should contain a level-one heading",
        description: "Include a visible <h1> (or role=\"heading\" aria-level=\"1\") identifying the page topic.",
        docs_url: "https://www.w3.org/WAI/tutorials/page-structure/headings/",
    };
}

impl Rule for PageHasHeadingOne {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        if !body_has_visible_content(doc) {
            return;
        }
        let Some(html_el) = doc.elements().find(|n| n.tag == "html") else {
            return;
        };
        let has_h1 = doc
            .elements()
            .any(|el| doc.is_in_accessibility_tree(el.id) && heading_level(doc, el) == Some(1));
        if !has_h1 {
            out.push(Violation::from_node(&self.meta(), doc, html_el.id));
        }
    }
}

impl ListRule {
    pub const META: RuleMeta = RuleMeta {
        id: "list",
        impact: Impact::Serious,
        wcag: "1.3.1 (A)",
        tags: &[
            "cat.structure",
            "wcag2a",
            "wcag131",
            "EN-301-549",
            "EN-9.1.3.1",
            "RGAAv4",
            "RGAA-9.3.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "<ul> and <ol> must only directly contain <li>, <script>, or <template> elements",
        description: "Ensure direct element children of <ul> and <ol> are <li> items so screen readers announce list counts accurately.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/info-and-relationships.html",
    };
}

impl Rule for ListRule {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !matches!(el.tag.as_str(), "ul" | "ol") || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if let Some(role) = explicit_role(el)
                && role != "list"
            {
                continue;
            }
            let bad_child = el.children.iter().find_map(|&cid| {
                let child = doc.node(cid)?;
                if !child.is_element() {
                    return None;
                }
                match child.tag.as_str() {
                    "script" | "template" => None,
                    "li" => {
                        let role = explicit_role(child);
                        if role.is_none() || role == Some("listitem") {
                            None
                        } else {
                            Some(child)
                        }
                    }
                    _ => Some(child),
                }
            });
            if let Some(bad) = bad_child {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!(
                        "List <{}> has disallowed direct child <{}>.",
                        el.tag, bad.tag
                    ),
                ));
            }
        }
    }
}

impl TabIndex {
    pub const META: RuleMeta = RuleMeta {
        id: "tabindex",
        impact: Impact::Serious,
        wcag: "2.4.3 (Best Practice)",
        tags: &["cat.keyboard", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Elements should not have tabindex greater than zero",
        description: "Avoid positive tabindex values; use DOM order and tabindex=\"0\" or \"-1\" instead.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/focus-order.html",
    };
}

impl Rule for TabIndex {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_rendered(el.id) {
                continue;
            }
            if let Some(raw) = el.attr("tabindex")
                && let Ok(val) = raw.trim().parse::<i32>()
                && val > 0
            {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!("Element has positive tabindex=\"{val}\"."),
                ));
            }
        }
    }
}

impl NestedInteractive {
    pub const META: RuleMeta = RuleMeta {
        id: "nested-interactive",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.keyboard",
            "wcag2a",
            "wcag412",
            "TTv5",
            "TT6.a",
            "EN-301-549",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-7.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Interactive controls must not be nested",
        description: "Do not nest links, buttons, or form controls inside another interactive control.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/name-role-value.html",
    };
}

impl Rule for NestedInteractive {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let is_img_role = effective_role(doc, el) == Some("img");
            if !is_interactive_control(doc, el) && !is_img_role {
                continue;
            }
            // <select> legitimately contains <option> children.
            if el.tag == "select" {
                continue;
            }
            let has_nested = has_interactive_descendant(doc, el.id);
            if has_nested {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

fn has_interactive_descendant(doc: &Document, parent_id: NodeId) -> bool {
    let mut visited = HashSet::new();
    visited.insert(parent_id);
    let mut stack: Vec<NodeId> = doc.node(parent_id).map_or_default(|n| n.children.clone());
    while let Some(cid) = stack.pop() {
        if !visited.insert(cid) {
            continue;
        }
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if !child.is_element() || !doc.is_rendered(cid) {
            continue;
        }
        if is_interactive_control(doc, child) {
            return true;
        }
        stack.extend(&child.children);
    }
    false
}

impl DefinitionList {
    pub const META: RuleMeta = RuleMeta {
        id: "definition-list",
        impact: Impact::Serious,
        wcag: "1.3.1 (A)",
        tags: &[
            "cat.structure",
            "wcag2a",
            "wcag131",
            "EN-301-549",
            "EN-9.1.3.1",
            "RGAAv4",
            "RGAA-9.3.3",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "<dl> elements must only directly contain properly-ordered <dt> and <dd> groups, <script>, <template> or <div> elements",
        description: "Ensure direct element children of <dl> are <dt>, <dd>, <script>, <template>, or <div> wrapping <dt>/<dd>.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/info-and-relationships.html",
    };
}

impl Rule for DefinitionList {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "dl" || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if let Some(role) = explicit_role(el)
                && role != "list"
            {
                continue;
            }
            let bad_child = find_disallowed_dl_child(doc, el);
            if let Some(bad_tag) = bad_child {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!("Definition list <dl> has disallowed child <{bad_tag}>."),
                ));
            }
        }
    }
}

fn has_disallowed_dl_role(node: &Node) -> bool {
    let Some(role) = explicit_role(node) else {
        return false;
    };
    match node.tag.as_str() {
        "dt" => !matches!(role, "term" | "listitem"),
        "dd" => !matches!(role, "definition"),
        "div" => !matches!(role, "generic" | "none" | "presentation"),
        _ => true,
    }
}

fn find_disallowed_dl_child<'a>(doc: &'a Document, dl: &Node) -> Option<&'a str> {
    let mut has_div_group = false;
    let mut has_bare_item = false;
    let mut saw_dt = false;

    for &cid in &dl.children {
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if !child.is_element() {
            continue;
        }
        match child.tag.as_str() {
            "script" | "template" => {}
            "dt" => {
                if has_disallowed_dl_role(child) {
                    return Some("dt");
                }
                has_bare_item = true;
                saw_dt = true;
            }
            "dd" => {
                if has_disallowed_dl_role(child) || !saw_dt {
                    return Some("dd");
                }
                has_bare_item = true;
            }
            "div" => {
                if has_disallowed_dl_role(child) {
                    return Some("div");
                }
                has_div_group = true;
                let mut div_saw_dt = false;
                let mut div_has_item = false;
                for &gcid in &child.children {
                    let Some(gc) = doc.node(gcid) else {
                        continue;
                    };
                    if !gc.is_element() {
                        continue;
                    }
                    match gc.tag.as_str() {
                        "script" | "template" => {}
                        "dt" => {
                            if has_disallowed_dl_role(gc) {
                                return Some("dt");
                            }
                            div_saw_dt = true;
                            div_has_item = true;
                        }
                        "dd" => {
                            if has_disallowed_dl_role(gc) || !div_saw_dt {
                                return Some("dd");
                            }
                            div_has_item = true;
                        }
                        _ => return Some(gc.tag.as_str()),
                    }
                }
                if !div_has_item {
                    return Some("div");
                }
            }
            _ => return Some(child.tag.as_str()),
        }
    }
    if has_div_group && has_bare_item {
        return Some("div");
    }
    None
}

impl DlItem {
    pub const META: RuleMeta = RuleMeta {
        id: "dlitem",
        impact: Impact::Serious,
        wcag: "1.3.1 (A)",
        tags: &[
            "cat.structure",
            "wcag2a",
            "wcag131",
            "EN-301-549",
            "EN-9.1.3.1",
            "RGAAv4",
            "RGAA-9.3.3",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "<dt> and <dd> elements must be contained by a <dl>",
        description: "Place every <dt> and <dd> directly inside a <dl> (or a <div> child of a <dl>).",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/info-and-relationships.html",
    };
}

impl Rule for DlItem {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !matches!(el.tag.as_str(), "dt" | "dd") || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if explicit_role(el).is_some() {
                continue;
            }
            let valid_parent = el.parent.and_then(|pid| doc.node(pid)).is_some_and(|p| {
                if p.tag == "dl" {
                    return explicit_role(p).is_none_or(|r| r == "list");
                }
                if p.tag == "div" && explicit_role(p).is_none() {
                    return p.parent.and_then(|gpid| doc.node(gpid)).is_some_and(|gp| {
                        gp.tag == "dl" && explicit_role(gp).is_none_or(|r| r == "list")
                    });
                }
                false
            });
            if !valid_parent {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl ListItem {
    pub const META: RuleMeta = RuleMeta {
        id: "listitem",
        impact: Impact::Serious,
        wcag: "1.3.1 (A)",
        tags: &[
            "cat.structure",
            "wcag2a",
            "wcag131",
            "EN-301-549",
            "EN-9.1.3.1",
            "RGAAv4",
            "RGAA-9.3.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "<li> elements must be contained in a <ul> or <ol>",
        description: "Place every <li> element inside a <ul>, <ol>, <menu>, or [role=\"list\"] parent.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/info-and-relationships.html",
    };
}

impl Rule for ListItem {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "li" || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if let Some(role) = explicit_role(el)
                && role != "listitem"
            {
                continue;
            }
            let valid_parent = el.parent.and_then(|pid| doc.node(pid)).is_some_and(|p| {
                if let Some(role) = explicit_role(p) {
                    role == "list"
                        || (matches!(p.tag.as_str(), "ul" | "ol" | "menu")
                            && matches!(role, "none" | "presentation"))
                } else {
                    matches!(p.tag.as_str(), "ul" | "ol" | "menu")
                }
            });
            if !valid_parent {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl DuplicateId {
    pub const META: RuleMeta = RuleMeta {
        id: "duplicate-id",
        impact: Impact::Minor,
        wcag: "4.1.1 (A - Obsolete)",
        tags: &["cat.parsing", "wcag2a-obsolete", "wcag411", "deprecated"],
        enabled_by_default: false,
        needs_review: false,
        help: "id attribute value must be unique",
        description: "Ensure every non-empty id attribute value is unique across the document.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/parsing.html",
    };
}

impl Rule for DuplicateId {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let mut id_nodes: BTreeMap<&str, Vec<NodeId>> = BTreeMap::new();
        for el in doc.elements() {
            if let Some(id_val) = el.id_attr() {
                id_nodes.entry(id_val).or_default().push(el.id);
            }
        }
        for (id_val, nodes) in id_nodes {
            if nodes.len() > 1 {
                for &nid in &nodes[1..] {
                    out.push(Violation::with_detail(
                        &self.meta(),
                        doc,
                        nid,
                        format!("Duplicate id='{id_val}' appears multiple times in the document."),
                    ));
                }
            }
        }
    }
}

impl DuplicateIdActive {
    pub const META: RuleMeta = RuleMeta {
        id: "duplicate-id-active",
        impact: Impact::Serious,
        wcag: "4.1.1 (A - Obsolete)",
        tags: &["cat.parsing", "wcag2a-obsolete", "wcag411", "deprecated"],
        enabled_by_default: false,
        needs_review: false,
        help: "IDs of active elements must be unique",
        description: "Ensure every non-empty id attribute value on focusable or interactive elements is unique.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/parsing.html",
    };
}

impl Rule for DuplicateIdActive {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let mut id_nodes: BTreeMap<&str, Vec<NodeId>> = BTreeMap::new();
        for el in doc.elements() {
            if let Some(id_val) = el.id_attr() {
                id_nodes.entry(id_val).or_default().push(el.id);
            }
        }
        for (id_val, nodes) in id_nodes {
            if nodes.len() <= 1 {
                continue;
            }
            let has_active = nodes.iter().any(|&nid| {
                doc.node(nid).is_some_and(|n| {
                    is_sequentially_focusable(n)
                        || is_natively_focusable(n)
                        || n.has_attr("tabindex")
                        || is_interactive_control(doc, n)
                })
            });
            if has_active {
                for &nid in &nodes[1..] {
                    out.push(Violation::with_detail(
                        &self.meta(),
                        doc,
                        nid,
                        format!(
                            "Duplicate id='{id_val}' on active/focusable element appears multiple times."
                        ),
                    ));
                }
            }
        }
    }
}
