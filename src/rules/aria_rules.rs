//! WAI-ARIA 1.2 validation rules: `aria-roles`, `aria-valid-attr`,
//! `aria-required-attr`, `aria-hidden-focus`, `duplicate-id-aria`,
//! `aria-allowed-attr`, `aria-allowed-role`, `aria-braille-equivalent`,
//! `aria-conditional-attr`, `aria-deprecated-role`, `aria-hidden-body`,
//! `aria-prohibited-attr`, `aria-required-children`, `aria-required-parent`,
//! `aria-roledescription`, `aria-text`, `aria-valid-attr-value`, and
//! `presentation-role-conflict`.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::accname::accessible_name;
use crate::aria::{
    ARIA_IDREF_ATTRS, allowed_attrs_for_role, effective_role, explicit_role,
    has_presentational_role_conflict, implicit_role, is_attr_allowed_for_role,
    is_case_sensitive_aria_attr, is_deprecated_role, is_global_aria_attr, is_inert,
    is_natively_focusable, is_sequentially_focusable, is_valid_aria_attr, is_valid_role,
    native_element_satisfies_attr, prohibited_attrs_for_role, required_attrs_for_role,
    required_children_for_role, required_parent_for_role, validate_aria_attr_value,
};
use crate::ir::{Document, MAX_TRAVERSAL_DEPTH, Node, NodeId};
use crate::rule::{Impact, Rule, RuleMeta, Violation};

pub struct AriaRoles;
pub struct AriaValidAttr;
pub struct AriaRequiredAttr;
pub struct AriaHiddenFocus;
pub struct DuplicateIdAria;
pub struct AriaAllowedAttr;
pub struct AriaAllowedRole;
pub struct AriaBrailleEquivalent;
pub struct AriaConditionalAttr;
pub struct AriaDeprecatedRole;
pub struct AriaHiddenBody;
pub struct AriaProhibitedAttr;
pub struct AriaRequiredChildren;
pub struct AriaRequiredParent;
pub struct AriaRoledescription;
pub struct AriaText;
pub struct AriaValidAttrValue;
pub struct PresentationRoleConflict;

impl AriaRoles {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-roles",
        impact: Impact::Critical,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag412",
            "EN-301-549",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-7.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "ARIA roles used must conform to valid WAI-ARIA 1.2 values",
        description: "Ensure every token in a role attribute is a valid, non-abstract WAI-ARIA 1.2 role.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/name-role-value.html",
    };
}

impl Rule for AriaRoles {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_rendered(el.id) {
                continue;
            }
            let Some(raw) = el.attr("role") else {
                continue;
            };
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let has_valid = trimmed.split_ascii_whitespace().any(is_valid_role);
            if !has_valid {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!("Invalid ARIA role(s): {trimmed}"),
                ));
            }
        }
    }
}

impl AriaValidAttr {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-valid-attr",
        impact: Impact::Critical,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag412",
            "EN-301-549",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-7.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "ARIA attributes must conform to valid WAI-ARIA 1.2 names",
        description: "Fix misspelled or non-standard aria-* attributes.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/name-role-value.html",
    };
}

impl Rule for AriaValidAttr {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_rendered(el.id) {
                continue;
            }
            let invalid: Vec<&str> = el
                .attrs
                .keys()
                .map(String::as_str)
                .filter(|k| k.starts_with("aria-") && !is_valid_aria_attr(k))
                .collect();
            if !invalid.is_empty() {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!("Invalid ARIA attribute(s): {}", invalid.join(", ")),
                ));
            }
        }
    }
}

impl AriaRequiredAttr {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-required-attr",
        impact: Impact::Critical,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag412",
            "EN-301-549",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-7.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Required ARIA attributes must be provided",
        description: "Elements with an explicit ARIA role must specify all required state/property attributes for that role.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/name-role-value.html",
    };
}

impl Rule for AriaRequiredAttr {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let Some(role) = explicit_role(el) else {
                continue;
            };
            let required: &[&str] = if role == "separator" && is_sequentially_focusable(el) {
                &["aria-valuenow"]
            } else {
                required_attrs_for_role(role)
            };
            if required.is_empty() {
                continue;
            }
            let missing: Vec<&str> = required
                .iter()
                .copied()
                .filter(|attr| {
                    if el.has_attr(attr) || native_element_satisfies_attr(el, attr) {
                        return false;
                    }
                    // Per WAI-ARIA 1.2/1.3, aria-valuetext provides a human-readable value
                    // alternative for slider's aria-valuenow.
                    if role == "slider"
                        && *attr == "aria-valuenow"
                        && el
                            .attr("aria-valuetext")
                            .is_some_and(|v| !v.trim().is_empty())
                    {
                        return false;
                    }
                    true
                })
                .collect();
            if !missing.is_empty() {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!(
                        "Role '{role}' is missing required attribute(s): {}",
                        missing.join(", ")
                    ),
                ));
            }
        }
    }
}

impl AriaHiddenFocus {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-hidden-focus",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.name-role-value",
            "wcag2a",
            "wcag412",
            "TTv5",
            "TT6.a",
            "EN-301-549",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-10.8.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "aria-hidden elements must not be focusable or contain focusable elements",
        description: "Remove aria-hidden=\"true\" or prevent keyboard focus (e.g. tabindex=\"-1\" or inert) on descendants.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/name-role-value.html",
    };
}

impl Rule for AriaHiddenFocus {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_rendered(el.id) {
                continue;
            }
            let is_hidden_root = el
                .attr("aria-hidden")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"));
            if !is_hidden_root || is_inert(doc, el.id) {
                continue;
            }
            if has_focusable_in_subtree(doc, el.id) {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

fn has_focusable_in_subtree(doc: &Document, id: NodeId) -> bool {
    let mut visited = HashSet::new();
    let mut stack = vec![id];
    while let Some(nid) = stack.pop() {
        if !visited.insert(nid) {
            continue;
        }
        let Some(node) = doc.node(nid) else {
            continue;
        };
        if !node.is_element() || !doc.is_rendered(nid) || node.has_attr("inert") {
            continue;
        }
        if is_sequentially_focusable(node) {
            return true;
        }
        stack.extend(&node.children);
    }
    false
}

impl DuplicateIdAria {
    pub const META: RuleMeta = RuleMeta {
        id: "duplicate-id-aria",
        impact: Impact::Critical,
        wcag: "4.1.1 (A)",
        tags: &[
            "cat.parsing",
            "wcag2a",
            "wcag412",
            "EN-301-549",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-8.2.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "IDs used in ARIA and labels must be unique",
        description: "Ensure every id referenced by aria-* attributes or <label for> appears only once in the document.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/parsing.html",
    };
}

fn id_document_scope(doc: &Document, id: NodeId) -> Option<NodeId> {
    doc.ancestors(id).into_iter().find(|&anc_id| {
        doc.node(anc_id)
            .is_some_and(|anc| matches!(anc.tag.as_str(), "iframe" | "frame"))
    })
}

impl Rule for DuplicateIdAria {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let mut referenced_ids: BTreeSet<(Option<NodeId>, &str)> = BTreeSet::new();
        for el in doc.elements() {
            let scope = id_document_scope(doc, el.id);
            if el.tag == "label"
                && let Some(target) = el.attr("for").map(str::trim).filter(|s| !s.is_empty())
            {
                referenced_ids.insert((scope, target));
            }
            for &attr in ARIA_IDREF_ATTRS {
                if let Some(val) = el.attr(attr) {
                    for token in val.split_ascii_whitespace() {
                        referenced_ids.insert((scope, token));
                    }
                }
            }
        }
        if referenced_ids.is_empty() {
            return;
        }

        let mut id_nodes: BTreeMap<(Option<NodeId>, &str), Vec<NodeId>> = BTreeMap::new();
        for el in doc.elements() {
            if let Some(id_val) = el.id_attr() {
                let key = (id_document_scope(doc, el.id), id_val);
                if referenced_ids.contains(&key) {
                    id_nodes.entry(key).or_default().push(el.id);
                }
            }
        }

        for ((_, id_val), nodes) in id_nodes {
            if nodes.len() > 1 {
                for &nid in &nodes[1..] {
                    out.push(Violation::with_detail(
                        &self.meta(),
                        doc,
                        nid,
                        format!("Duplicate id '{id_val}' is referenced by ARIA or <label for>."),
                    ));
                }
            }
        }
    }
}

impl AriaAllowedAttr {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-allowed-attr",
        impact: Impact::Critical,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag412",
            "EN-301-549",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-7.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Elements must only use supported ARIA attributes",
        description: "Ensure an element's role supports its ARIA attributes",
        docs_url: "https://www.w3.org/TR/wai-aria-1.2/#state_prop_def",
    };
}

impl Rule for AriaAllowedAttr {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_rendered(el.id) {
                continue;
            }
            let role = effective_role(doc, el);
            let prohibited_role = prohibited_role_for_element(doc, el);
            let disallows_global_aria = matches!(el.tag.as_str(), "br" | "wbr");
            let is_native_password_input = el.tag == "input"
                && explicit_role(el).is_none()
                && el
                    .attr("type")
                    .is_some_and(|t| t.trim().eq_ignore_ascii_case("password"));
            let disallowed: Vec<&str> = el
                .attrs
                .iter()
                .filter_map(|(k, v)| {
                    let k = k.as_str();
                    if !k.starts_with("aria-") || !is_valid_aria_attr(k) {
                        return None;
                    }
                    if disallows_global_aria {
                        return (k != "aria-hidden").then_some(k);
                    }
                    // Prohibited attributes are reported by `aria-prohibited-attr`.
                    if prohibited_role.is_some_and(|r| prohibited_attrs_for_role(r).contains(&k)) {
                        return None;
                    }
                    if is_native_password_input && allowed_attrs_for_role("textbox").contains(&k) {
                        return None;
                    }
                    // Screen readers consistently ignore default-false aria-required="false"
                    // and aria-multiline="false" on contenteditable elements.
                    let trimmed = v.trim();
                    if (k == "aria-required" && trimmed == "false")
                        || (k == "aria-multiline"
                            && trimmed == "false"
                            && el.has_attr("contenteditable"))
                    {
                        return None;
                    }
                    if !is_attr_allowed_for_role(role, k) {
                        return Some(k);
                    }
                    // Assistive technologies misread uppercase token values on case-sensitive ARIA states.
                    if is_case_sensitive_aria_attr(k)
                        && !trimmed.is_empty()
                        && trimmed != trimmed.to_ascii_lowercase()
                    {
                        return Some(k);
                    }
                    None
                })
                .collect();
            if !disallowed.is_empty() {
                let mut v = Violation::with_detail(
                    &Self::META,
                    doc,
                    el.id,
                    format!(
                        "ARIA attribute(s) not allowed for role '{}': {}",
                        role.unwrap_or("none"),
                        disallowed.join(", ")
                    ),
                );
                // Custom elements without an explicit role may attach ElementInternals.role at runtime.
                if role.is_none() && el.tag.contains('-') && !is_sequentially_focusable(el) {
                    v = v.into_incomplete();
                }
                out.push(v);
            }
        }
    }
}

impl AriaAllowedRole {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-allowed-role",
        impact: Impact::Minor,
        wcag: "Best Practice",
        tags: &["cat.aria", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "ARIA role should be appropriate for the element",
        description: "Ensure role attribute has an appropriate value for the element",
        docs_url: "https://www.w3.org/TR/html-aria/",
    };
}

impl Rule for AriaAllowedRole {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.has_attr("hidden") {
                continue;
            }
            let Some(role) = explicit_role(el) else {
                continue;
            };
            if !is_role_allowed_on_element(doc, el, role) {
                out.push(Violation::with_detail(
                    &Self::META,
                    doc,
                    el.id,
                    format!("ARIA role '{role}' is not allowed on <{}>.", el.tag),
                ));
            }
        }
    }
}

fn has_figcaption_descendant(doc: &Document, id: NodeId) -> bool {
    let mut visited = HashSet::new();
    visited.insert(id);
    let mut stack: Vec<NodeId> = doc.node(id).map_or_default(|n| n.children.clone());
    while let Some(cid) = stack.pop() {
        if !visited.insert(cid) {
            continue;
        }
        if let Some(child) = doc.node(cid)
            && child.is_element()
        {
            if child.tag == "figcaption" {
                return true;
            }
            stack.extend(&child.children);
        }
    }
    false
}

fn is_first_summary_in_details(doc: &Document, el: &Node) -> bool {
    let Some(parent_id) = el.parent else {
        return false;
    };
    let Some(parent) = doc.node(parent_id) else {
        return false;
    };
    if parent.tag != "details" {
        return false;
    }
    parent.children.iter().find_map(|&cid| {
        let child = doc.node(cid)?;
        (child.is_element() && child.tag == "summary").then_some(cid)
    }) == Some(el.id)
}

#[allow(clippy::too_many_lines)]
fn is_role_allowed_on_element(doc: &Document, el: &Node, role: &str) -> bool {
    // DPUB-ARIA roles that are only valid on specific HTML host elements.
    match role {
        "doc-backlink" | "doc-biblioref" | "doc-glossref" | "doc-noteref" => {
            return matches!(el.tag.as_str(), "a" | "area") && el.has_attr("href");
        }
        "doc-cover" => return el.tag == "img",
        "doc-biblioentry" | "doc-endnote" => return el.tag == "li",
        _ => {}
    }

    match el.tag.as_str() {
        "base" | "body" | "col" | "colgroup" | "dd" | "dt" | "figcaption" | "head" | "html"
        | "label" | "legend" | "link" | "meta" | "noscript" | "param" | "script" | "slot"
        | "source" | "style" | "template" | "title" | "track" => false,
        "br" | "wbr" => matches!(role, "none" | "presentation"),
        "option" => role == "option",
        "details" | "optgroup" => role == "group",
        "summary" => !is_first_summary_in_details(doc, el) || role == "button",
        "main" => role == "main",
        "nav" => matches!(
            role,
            "navigation"
                | "doc-index"
                | "doc-pagelist"
                | "doc-toc"
                | "menu"
                | "menubar"
                | "none"
                | "presentation"
                | "tablist"
        ),
        "aside" => matches!(
            role,
            "complementary"
                | "doc-dedication"
                | "doc-example"
                | "doc-footnote"
                | "doc-glossary"
                | "doc-pullquote"
                | "doc-tip"
                | "feed"
                | "none"
                | "note"
                | "presentation"
                | "region"
                | "search"
        ),
        "article" => matches!(
            role,
            "article"
                | "application"
                | "document"
                | "feed"
                | "main"
                | "none"
                | "presentation"
                | "region"
        ),
        "section" => matches!(
            role,
            "alert"
                | "alertdialog"
                | "application"
                | "banner"
                | "complementary"
                | "contentinfo"
                | "dialog"
                | "doc-abstract"
                | "doc-acknowledgments"
                | "doc-afterword"
                | "doc-appendix"
                | "doc-bibliography"
                | "doc-chapter"
                | "doc-colophon"
                | "doc-conclusion"
                | "doc-credit"
                | "doc-credits"
                | "doc-dedication"
                | "doc-endnotes"
                | "doc-epigraph"
                | "doc-epilogue"
                | "doc-errata"
                | "doc-example"
                | "doc-foreword"
                | "doc-glossary"
                | "doc-index"
                | "doc-introduction"
                | "doc-notice"
                | "doc-pagelist"
                | "doc-part"
                | "doc-preface"
                | "doc-prologue"
                | "doc-pullquote"
                | "doc-qna"
                | "doc-toc"
                | "document"
                | "feed"
                | "group"
                | "log"
                | "main"
                | "marquee"
                | "navigation"
                | "none"
                | "note"
                | "presentation"
                | "region"
                | "search"
                | "status"
                | "tabpanel"
        ),
        "figure" => {
            if has_figcaption_descendant(doc, el.id) {
                matches!(role, "figure" | "doc-example" | "none" | "presentation")
            } else {
                true
            }
        }
        "fieldset" => matches!(role, "group" | "none" | "presentation" | "radiogroup"),
        "dl" => matches!(role, "group" | "list" | "none" | "presentation"),
        "iframe" | "embed" | "object" => {
            matches!(
                role,
                "application" | "document" | "image" | "img" | "none" | "presentation"
            )
        }
        "audio" | "video" => role == "application",
        "img" => {
            if el.attr("alt") == Some("") {
                matches!(role, "none" | "presentation")
            } else if !el.has_attr("alt") && accessible_name(doc, el.id).is_empty() {
                matches!(role, "img" | "image" | "none" | "presentation")
            } else {
                matches!(
                    role,
                    "img"
                        | "image"
                        | "button"
                        | "checkbox"
                        | "doc-cover"
                        | "link"
                        | "math"
                        | "menuitem"
                        | "menuitemcheckbox"
                        | "menuitemradio"
                        | "meter"
                        | "none"
                        | "option"
                        | "presentation"
                        | "progressbar"
                        | "radio"
                        | "scrollbar"
                        | "separator"
                        | "slider"
                        | "switch"
                        | "tab"
                        | "treeitem"
                )
            }
        }
        "header" | "footer" => matches!(
            role,
            "banner" | "contentinfo" | "group" | "none" | "presentation"
        ),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            matches!(
                role,
                "heading" | "doc-subtitle" | "none" | "presentation" | "tab"
            )
        }
        "button" => matches!(
            role,
            "button"
                | "checkbox"
                | "combobox"
                | "gridcell"
                | "link"
                | "menuitem"
                | "menuitemcheckbox"
                | "menuitemradio"
                | "option"
                | "radio"
                | "separator"
                | "slider"
                | "switch"
                | "tab"
                | "treeitem"
        ),
        "a" if el.has_attr("href") => matches!(
            role,
            "link"
                | "button"
                | "checkbox"
                | "doc-backlink"
                | "doc-biblioref"
                | "doc-glossref"
                | "doc-noteref"
                | "menuitem"
                | "menuitemcheckbox"
                | "menuitemradio"
                | "option"
                | "radio"
                | "switch"
                | "tab"
                | "treeitem"
        ),
        "area" => {
            if el.has_attr("href") {
                role == "link"
            } else {
                matches!(role, "button" | "link" | "none" | "presentation")
            }
        }
        "select" => matches!(role, "combobox" | "listbox" | "menu"),
        "textarea" => role == "textbox",
        "progress" => role == "progressbar",
        "meter" => role == "meter",
        "hr" => matches!(role, "separator" | "none" | "presentation"),
        "ul" | "ol" | "menu" => matches!(
            role,
            "list"
                | "directory"
                | "group"
                | "listbox"
                | "menu"
                | "menubar"
                | "none"
                | "presentation"
                | "radiogroup"
                | "tablist"
                | "toolbar"
                | "tree"
        ),
        "li" => matches!(
            role,
            "listitem"
                | "doc-biblioentry"
                | "doc-endnote"
                | "menuitem"
                | "menuitemcheckbox"
                | "menuitemradio"
                | "none"
                | "option"
                | "presentation"
                | "radio"
                | "separator"
                | "tab"
                | "treeitem"
        ),
        "dialog" => matches!(role, "dialog" | "alertdialog"),
        "form" => matches!(role, "form" | "none" | "presentation" | "search"),
        "input" => is_role_allowed_on_input(el, role),
        _ => true,
    }
}

fn is_role_allowed_on_input(el: &Node, role: &str) -> bool {
    let ty = el
        .attr("type")
        .map_or("text", str::trim)
        .to_ascii_lowercase();
    match ty.as_str() {
        "hidden" => false,
        "checkbox" => match role {
            "checkbox" | "menuitemcheckbox" | "option" | "switch" => true,
            "button" => el.has_attr("aria-pressed"),
            _ => false,
        },
        "radio" => matches!(role, "radio" | "menuitemradio"),
        "button" | "submit" | "reset" | "image" => matches!(
            role,
            "button"
                | "checkbox"
                | "combobox"
                | "link"
                | "menuitem"
                | "menuitemcheckbox"
                | "menuitemradio"
                | "option"
                | "radio"
                | "switch"
                | "tab"
        ),
        "range" => role == "slider",
        "number" => role == "spinbutton",
        "search" => matches!(role, "searchbox" | "combobox"),
        "text" | "email" | "tel" | "url" => {
            matches!(role, "textbox" | "combobox" | "searchbox" | "spinbutton")
        }
        _ => true,
    }
}

impl AriaBrailleEquivalent {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-braille-equivalent",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &["cat.aria", "wcag2a", "wcag412", "EN-301-549", "EN-9.4.1.2"],
        enabled_by_default: true,
        needs_review: false,
        help: "aria-braille attributes must have a non-braille equivalent",
        description: "Ensure aria-braillelabel and aria-brailleroledescription have a non-braille equivalent",
        docs_url: "https://www.w3.org/TR/wai-aria-1.3/#aria-braillelabel",
    };
}

impl Rule for AriaBrailleEquivalent {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if el
                .attr("aria-braillelabel")
                .is_some_and(|v| !v.trim().is_empty())
                && accessible_name(doc, el.id).is_empty()
            {
                out.push(Violation::with_detail(
                    &Self::META,
                    doc,
                    el.id,
                    "aria-braillelabel requires a non-empty accessible name.",
                ));
            }
            if el
                .attr("aria-brailleroledescription")
                .is_some_and(|v| !v.trim().is_empty())
                && el
                    .attr("aria-roledescription")
                    .is_none_or(|v| v.trim().is_empty())
            {
                out.push(Violation::with_detail(
                    &Self::META,
                    doc,
                    el.id,
                    "aria-brailleroledescription requires a non-empty aria-roledescription.",
                ));
            }
        }
    }
}

impl AriaConditionalAttr {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-conditional-attr",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag412",
            "EN-301-549",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-7.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "ARIA attributes must be used as specified for the element's role",
        description: "Ensure ARIA attributes are used as described in the specification of the element's role",
        docs_url: "https://www.w3.org/TR/html-aria/",
    };
}

impl Rule for AriaConditionalAttr {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let mut disallowed: Vec<&str> = Vec::new();

            // Native <input type="checkbox"> / <input type="radio"> must not have an
            // aria-checked value that contradicts the native `checked` state.
            if el.tag == "input"
                && let Some(aria_checked) = el
                    .attr("aria-checked")
                    .map(|v| v.trim().to_ascii_lowercase())
                    .filter(|v| !v.is_empty())
            {
                let ty = el
                    .attr("type")
                    .map_or("text", str::trim)
                    .to_ascii_lowercase();
                let is_checked = el.has_attr("checked");
                if (ty == "checkbox" && effective_role(doc, el) == Some("checkbox"))
                    || (ty == "radio" && effective_role(doc, el) == Some("radio"))
                {
                    let contradicts = (aria_checked == "true" && !is_checked)
                        || (aria_checked == "false" && is_checked)
                        || aria_checked == "mixed";
                    if contradicts {
                        disallowed.push("aria-checked");
                    }
                }
            }

            // Any element with role="row" outside a treegrid must not use treegrid-only row attributes.
            if effective_role(doc, el) == Some("row") && !has_ancestor_role(doc, el.id, "treegrid")
            {
                for attr in [
                    "aria-expanded",
                    "aria-level",
                    "aria-posinset",
                    "aria-setsize",
                ] {
                    if el.has_attr(attr) {
                        disallowed.push(attr);
                    }
                }
            }

            // Native colspan / rowspan supersede aria-colspan / aria-rowspan.
            if el.has_attr("colspan") && el.has_attr("aria-colspan") {
                disallowed.push("aria-colspan");
            }
            if el.has_attr("rowspan") && el.has_attr("aria-rowspan") {
                disallowed.push("aria-rowspan");
            }

            if !disallowed.is_empty() {
                out.push(Violation::with_detail(
                    &Self::META,
                    doc,
                    el.id,
                    format!(
                        "Conditionally prohibited ARIA attribute(s): {}",
                        disallowed.join(", ")
                    ),
                ));
            }
        }
    }
}

fn has_ancestor_role(doc: &Document, id: NodeId, target_role: &str) -> bool {
    doc.ancestors(id).into_iter().any(|anc_id| {
        doc.node(anc_id)
            .is_some_and(|anc| effective_role(doc, anc) == Some(target_role))
    })
}

impl AriaDeprecatedRole {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-deprecated-role",
        impact: Impact::Minor,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag412",
            "EN-301-549",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-7.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Deprecated ARIA roles must not be used",
        description: "Ensure elements do not use deprecated roles",
        docs_url: "https://www.w3.org/TR/wai-aria-1.2/#deprecated_roles",
    };
}

impl Rule for AriaDeprecatedRole {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_rendered(el.id) {
                continue;
            }
            if let Some(role) = explicit_role(el)
                && is_deprecated_role(role)
            {
                out.push(Violation::with_detail(
                    &Self::META,
                    doc,
                    el.id,
                    format!("Deprecated ARIA role '{role}' must not be used."),
                ));
            }
        }
    }
}

impl AriaHiddenBody {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-hidden-body",
        impact: Impact::Critical,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag131",
            "wcag412",
            "EN-301-549",
            "EN-9.1.3.1",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-10.8.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "aria-hidden=\"true\" must not be present on the document body",
        description: "Ensure aria-hidden=\"true\" is not present on the document body.",
        docs_url: "https://www.w3.org/TR/wai-aria-1.2/#aria-hidden",
    };
}

impl Rule for AriaHiddenBody {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag == "body"
                && el
                    .attr("aria-hidden")
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
            {
                out.push(Violation::from_node(&Self::META, doc, el.id));
            }
        }
    }
}

fn prohibited_role_for_element<'a>(doc: &Document, el: &'a Node) -> Option<&'a str> {
    explicit_role(el).or_else(|| implicit_role(doc, el))
}

/// HTML elements with no corresponding ARIA role that prohibit naming (`aria-label`,
/// `aria-labelledby`, `aria-braillelabel`) per HTML-AAM when no explicit role is set.
fn html_element_prohibits_naming(tag: &str) -> bool {
    matches!(
        tag,
        "abbr"
            | "b"
            | "bdi"
            | "bdo"
            | "br"
            | "cite"
            | "col"
            | "colgroup"
            | "data"
            | "dd"
            | "dfn"
            | "dt"
            | "figcaption"
            | "i"
            | "kbd"
            | "label"
            | "legend"
            | "q"
            | "rp"
            | "rt"
            | "ruby"
            | "samp"
            | "small"
            | "u"
            | "var"
            | "wbr"
    )
}

fn has_presentational_children_ancestor(doc: &Document, id: NodeId) -> bool {
    doc.ancestors(id).into_iter().any(|anc_id| {
        doc.node(anc_id).is_some_and(|anc| {
            matches!(
                effective_role(doc, anc),
                Some(
                    "button"
                        | "checkbox"
                        | "img"
                        | "image"
                        | "math"
                        | "menuitemcheckbox"
                        | "menuitemradio"
                        | "option"
                        | "progressbar"
                        | "radio"
                        | "scrollbar"
                        | "separator"
                        | "slider"
                        | "switch"
                        | "tab"
                )
            )
        })
    })
}

impl AriaProhibitedAttr {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-prohibited-attr",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag412",
            "EN-301-549",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-7.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Elements must only use permitted ARIA attributes",
        description: "Ensure ARIA attributes are not prohibited for an element's role",
        docs_url: "https://www.w3.org/TR/wai-aria-1.2/#prohibitedattributes",
    };
}

impl Rule for AriaProhibitedAttr {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let role = prohibited_role_for_element(doc, el);
            let prohibited: &[&str] = if let Some(r) = role {
                prohibited_attrs_for_role(r)
            } else if explicit_role(el).is_none() && html_element_prohibits_naming(&el.tag) {
                &["aria-braillelabel", "aria-label", "aria-labelledby"]
            } else {
                &[]
            };
            if prohibited.is_empty() {
                continue;
            }
            // A generic element without explicit role inside a children-presentational widget
            // (e.g. `<div role="button"><span aria-label="..."></span></div>`) contributes to the
            // parent widget's accessible name rather than exposing a prohibited label of its own.
            let is_widget_name_span = explicit_role(el).is_none()
                && role == Some("generic")
                && has_presentational_children_ancestor(doc, el.id);
            let found: Vec<&str> = prohibited
                .iter()
                .copied()
                .filter(|&attr| {
                    if is_widget_name_span && matches!(attr, "aria-label" | "aria-labelledby") {
                        return false;
                    }
                    el.attr(attr).is_some_and(|v| !v.trim().is_empty())
                })
                .collect();
            if !found.is_empty() {
                let mut v = Violation::with_detail(
                    &Self::META,
                    doc,
                    el.id,
                    format!(
                        "Prohibited ARIA attribute(s) for role '{}': {}",
                        role.unwrap_or(el.tag.as_str()),
                        found.join(", ")
                    ),
                );
                // Per ARIA in HTML (`aria-prohibited-attr`), naming attributes on generic/native
                // non-widget elements without an explicit role are routed to `incomplete`
                // (`needsReview`) because AT support varies.
                let is_generic_naming_only = explicit_role(el).is_none()
                    && found.iter().all(|&attr| {
                        matches!(attr, "aria-label" | "aria-labelledby" | "aria-braillelabel")
                    });
                if is_generic_naming_only {
                    v = v.into_incomplete();
                }
                out.push(v);
            }
        }
    }
}

impl AriaRequiredChildren {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-required-children",
        impact: Impact::Critical,
        wcag: "1.3.1 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag131",
            "EN-301-549",
            "EN-9.1.3.1",
            "RGAAv4",
            "RGAA-9.3.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Certain ARIA roles must contain particular children",
        description: "Ensure elements with an ARIA role that require child roles contain them",
        docs_url: "https://www.w3.org/TR/wai-aria-1.2/#mustContain",
    };
}

impl Rule for AriaRequiredChildren {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let Some(role) = explicit_role(el) else {
                continue;
            };
            let required = required_children_for_role(role);
            if required.is_empty() {
                continue;
            }
            if el
                .attr("aria-busy")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
                && !has_any_owned_element(doc, el)
            {
                continue;
            }
            let mut visited = Vec::with_capacity(8);
            visited.push(el.id);
            let (found_required, has_disallowed) =
                validate_owned_children(doc, el, role, required, 0, &mut visited);
            if !found_required || has_disallowed {
                out.push(Violation::with_detail(
                    &Self::META,
                    doc,
                    el.id,
                    format!(
                        "Role '{role}' requires owned children with role: {}",
                        required.join(", ")
                    ),
                ));
            }
        }
    }
}

fn has_any_owned_element(doc: &Document, el: &Node) -> bool {
    if el
        .children
        .iter()
        .any(|&cid| doc.node(cid).is_some_and(Node::is_element))
    {
        return true;
    }
    el.attr("aria-owns").is_some_and(|owns| {
        owns.split_ascii_whitespace()
            .any(|id| doc.find_by_id(id).is_some())
    })
}

fn is_transparent_container(doc: &Document, node: &Node) -> bool {
    if is_sequentially_focusable(node) {
        return false;
    }
    let role = effective_role(doc, node);
    if !matches!(role, None | Some("generic" | "none" | "presentation")) {
        return false;
    }
    !node
        .attrs
        .iter()
        .any(|(k, v)| k != "aria-hidden" && is_global_aria_attr(k) && !v.trim().is_empty())
}

fn validate_owned_children(
    doc: &Document,
    container: &Node,
    parent_role: &str,
    required: &[&str],
    depth: usize,
    visited: &mut Vec<NodeId>,
) -> (bool, bool) {
    if depth >= MAX_TRAVERSAL_DEPTH {
        return (false, false);
    }
    let mut candidate_ids: Vec<NodeId> = container.children.clone();
    if let Some(owns) = container.attr("aria-owns") {
        for ref_id in owns.split_ascii_whitespace() {
            if let Some(target_id) = doc.find_by_id(ref_id) {
                candidate_ids.push(target_id);
            }
        }
    }

    let mut found_required = false;
    let mut has_disallowed = false;

    for cid in candidate_ids {
        if visited.contains(&cid) {
            continue;
        }
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if !child.is_element() || !doc.is_in_accessibility_tree(cid) {
            continue;
        }
        visited.push(cid);
        if is_transparent_container(doc, child) {
            let (sub_found, sub_bad) =
                validate_owned_children(doc, child, parent_role, required, depth + 1, visited);
            found_required |= sub_found;
            has_disallowed |= sub_bad;
            continue;
        }
        let child_role = effective_role(doc, child);
        if matches!(parent_role, "table" | "grid" | "treegrid") && child_role == Some("caption") {
            continue;
        }
        if matches!(parent_role, "menu" | "menubar")
            && matches!(child_role, Some("separator" | "menu"))
        {
            continue;
        }
        if child_role.is_some_and(|r| required.contains(&r)) {
            if child_role == Some("group") {
                let (sub_found, sub_bad) =
                    validate_owned_children(doc, child, parent_role, required, depth + 1, visited);
                if sub_bad || !sub_found {
                    has_disallowed = true;
                } else {
                    found_required = true;
                }
            } else {
                found_required = true;
            }
        } else {
            has_disallowed = true;
        }
    }
    (found_required, has_disallowed)
}

impl AriaRequiredParent {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-required-parent",
        impact: Impact::Critical,
        wcag: "1.3.1 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag131",
            "EN-301-549",
            "EN-9.1.3.1",
            "RGAAv4",
            "RGAA-9.3.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Certain ARIA roles must be contained by particular parents",
        description: "Ensure elements with an ARIA role that require parent roles are contained by them",
        docs_url: "https://www.w3.org/TR/wai-aria-1.2/#scope",
    };
}

impl Rule for AriaRequiredParent {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let mut owned_by: BTreeMap<&str, Vec<NodeId>> = BTreeMap::new();
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if let Some(owns) = el.attr("aria-owns") {
                for target_id in owns.split_ascii_whitespace() {
                    owned_by.entry(target_id).or_default().push(el.id);
                }
            }
        }

        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let Some(role) = explicit_role(el) else {
                continue;
            };
            let required_parents = required_parent_for_role(role);
            if required_parents.is_empty() {
                continue;
            }
            if has_required_parent(doc, el, role, required_parents, &owned_by) {
                continue;
            }
            out.push(Violation::with_detail(
                &Self::META,
                doc,
                el.id,
                format!(
                    "Role '{role}' must be contained within a parent with role: {}",
                    required_parents.join(", ")
                ),
            ));
        }
    }
}

fn has_required_parent(
    doc: &Document,
    el: &Node,
    el_role: &str,
    required_parents: &[&str],
    owned_by: &BTreeMap<&str, Vec<NodeId>>,
) -> bool {
    let is_owned_by_required = |node: &Node| -> bool {
        node.id_attr().is_some_and(|nid| {
            owned_by.get(nid).is_some_and(|owners| {
                owners.iter().any(|&owner_id| {
                    doc.node(owner_id).is_some_and(|owner| {
                        effective_role(doc, owner)
                            .is_some_and(|r| r != "group" && required_parents.contains(&r))
                    })
                })
            })
        })
    };

    let mut seen_group = false;
    for anc_id in doc.ancestors(el.id) {
        let Some(anc) = doc.node(anc_id) else {
            continue;
        };
        if !anc.is_element() {
            continue;
        }
        let anc_role = effective_role(doc, anc);
        if anc_role == Some("group") && required_parents.contains(&"group") {
            seen_group = true;
            if is_owned_by_required(anc) {
                return true;
            }
            continue;
        }
        // Per WAI-ARIA 1.2, nested treeitems inside a group are owned by an enclosing treeitem in a tree.
        if el_role == "treeitem" && seen_group && anc_role == Some("treeitem") {
            if is_owned_by_required(anc) {
                return true;
            }
            continue;
        }
        if anc_role.is_some_and(|r| required_parents.contains(&r)) {
            return true;
        }
        if is_transparent_container(doc, anc) {
            if is_owned_by_required(anc) {
                return true;
            }
            continue;
        }
        break;
    }
    is_owned_by_required(el)
}

impl AriaRoledescription {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-roledescription",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag412",
            "EN-301-549",
            "EN-9.4.1.2",
            "deprecated",
        ],
        enabled_by_default: false,
        needs_review: false,
        help: "aria-roledescription must be on elements with a semantic role",
        description: "Ensure aria-roledescription is only used on elements with an implicit or explicit role",
        docs_url: "https://www.w3.org/TR/wai-aria-1.2/#aria-roledescription",
    };
}

impl Rule for AriaRoledescription {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if el
                .attr("aria-roledescription")
                .is_none_or(|v| v.trim().is_empty())
            {
                continue;
            }
            let has_semantic_role = effective_role(doc, el).is_some_and(|r| {
                !(matches!(r, "generic" | "none" | "presentation")
                    || (explicit_role(el).is_none() && r == "paragraph"))
            });
            if !has_semantic_role {
                out.push(Violation::from_node(&Self::META, doc, el.id));
            }
        }
    }
}

impl AriaText {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-text",
        impact: Impact::Serious,
        wcag: "Best Practice",
        tags: &["cat.aria", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "\"role=text\" should have no focusable descendants",
        description: "Ensure role=\"text\" is used on elements with no focusable descendants",
        docs_url: "https://www.w3.org/TR/wai-aria-1.2/",
    };
}

fn has_aria_text_focusable_in_subtree(doc: &Document, id: NodeId) -> bool {
    let mut visited = HashSet::new();
    let mut stack = vec![id];
    while let Some(nid) = stack.pop() {
        if !visited.insert(nid) {
            continue;
        }
        let Some(node) = doc.node(nid) else {
            continue;
        };
        if !node.is_element() || !doc.is_rendered(nid) || node.has_attr("inert") {
            continue;
        }
        if is_sequentially_focusable(node) || is_natively_focusable(node) {
            return true;
        }
        stack.extend(&node.children);
    }
    false
}

impl Rule for AriaText {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let is_text_role = el
                .attr("role")
                .is_some_and(|r| r.split_ascii_whitespace().any(|tok| tok == "text"));
            if !is_text_role {
                continue;
            }
            if el
                .children
                .iter()
                .any(|&cid| has_aria_text_focusable_in_subtree(doc, cid))
            {
                out.push(Violation::from_node(&Self::META, doc, el.id));
            }
        }
    }
}

impl AriaValidAttrValue {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-valid-attr-value",
        impact: Impact::Critical,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag412",
            "EN-301-549",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-7.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "ARIA attributes must conform to valid values",
        description: "Ensure all ARIA attributes have valid values",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/name-role-value.html",
    };
}

impl Rule for AriaValidAttrValue {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let all_ids: HashSet<&str> = doc.elements().filter_map(Node::id_attr).collect();
        for el in doc.elements() {
            if !doc.is_rendered(el.id) {
                continue;
            }
            let is_marked_invalid = el
                .attr("aria-invalid")
                .map(str::trim)
                .is_some_and(|v| !v.is_empty() && v != "false");
            let invalid: Vec<String> = el
                .attrs
                .iter()
                .filter(|(k, v)| {
                    if !k.starts_with("aria-") || !is_valid_aria_attr(k) {
                        return false;
                    }
                    if v.trim().is_empty() {
                        return false;
                    }
                    if k.as_str() == "aria-errormessage" {
                        if !is_marked_invalid {
                            return false;
                        }
                        // Per WAI-ARIA 1.2, aria-errormessage takes a single IDREF (not an IDREFS list).
                        let trimmed = v.trim();
                        if trimmed.bytes().any(|b| b.is_ascii_whitespace()) {
                            return true;
                        }
                        let describedby = el.attr("aria-describedby").unwrap_or("");
                        let has_visible_error = doc.find_by_id(trimmed).is_some_and(|tid| {
                            if !doc.is_in_accessibility_tree(tid) {
                                return false;
                            }
                            let is_live = doc.node(tid).is_some_and(|target| {
                                effective_role(doc, target) == Some("alert")
                                    || target.attr("aria-live").is_some_and(|lv| {
                                        let l = lv.trim();
                                        !l.is_empty() && !l.eq_ignore_ascii_case("off")
                                    })
                            });
                            let also_described =
                                describedby.split_ascii_whitespace().any(|d| d == trimmed);
                            is_live
                                || also_described
                                || has_visible_text(doc, tid)
                                || !accessible_name(doc, tid).is_empty()
                        });
                        return !has_visible_error;
                    }
                    !validate_aria_attr_value(k, v, &all_ids)
                })
                .map(|(k, v)| format!("{k}=\"{v}\""))
                .collect();
            if !invalid.is_empty() {
                out.push(Violation::with_detail(
                    &Self::META,
                    doc,
                    el.id,
                    format!("Invalid ARIA attribute value(s): {}", invalid.join(", ")),
                ));
            }
        }
    }
}

fn has_visible_text(doc: &Document, id: NodeId) -> bool {
    let mut visited = HashSet::new();
    let mut stack = vec![id];
    while let Some(nid) = stack.pop() {
        if !visited.insert(nid) {
            continue;
        }
        let Some(node) = doc.node(nid) else {
            continue;
        };
        if node.is_text() {
            if !node.text.trim().is_empty() {
                return true;
            }
            continue;
        }
        if !node.is_element() || !doc.is_in_accessibility_tree(nid) {
            continue;
        }
        stack.extend(&node.children);
    }
    false
}

impl PresentationRoleConflict {
    pub const META: RuleMeta = RuleMeta {
        id: "presentation-role-conflict",
        impact: Impact::Minor,
        wcag: "Best Practice",
        tags: &["cat.aria", "best-practice", "ACT"],
        enabled_by_default: true,
        needs_review: false,
        help: "Elements marked as presentational should be consistently ignored",
        description: "Ensure elements marked as presentational do not have global ARIA or tabindex so that all screen readers ignore them",
        docs_url: "https://www.w3.org/TR/wai-aria-1.2/#conflict_resolution_presentation_none",
    };
}

impl Rule for PresentationRoleConflict {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if has_presentational_role_conflict(el) {
                out.push(Violation::from_node(&Self::META, doc, el.id));
            }
        }
    }
}
