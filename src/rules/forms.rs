//! Form labeling and interaction rules: `label`, `select-name`, `autocomplete-valid`,
//! `form-field-multiple-labels`, `label-content-name-mismatch`, `label-title-only`,
//! and `accesskeys`.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::accname::{accessible_name, collect_raw_text};
use crate::aria::{effective_role, is_effectively_presentational, is_interactive_control};
use crate::ir::{Document, MAX_TRAVERSAL_DEPTH, Node, NodeId, normalize_whitespace};
use crate::rule::{AuditOptions, ConformanceProfile, Impact, Rule, RuleMeta, Violation};

pub struct FormLabel;
pub struct SelectName;
pub struct AutocompleteValid;
pub struct FormFieldMultipleLabels;
pub struct LabelContentNameMismatch;
pub struct LabelTitleOnly;
pub struct Accesskeys;

impl FormLabel {
    pub const META: RuleMeta = RuleMeta {
        id: "label",
        impact: Impact::Critical,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.forms",
            "wcag2a",
            "wcag412",
            "section508",
            "section508.22.n",
            "TTv5",
            "TT5.c",
            "EN-301-549",
            "EN-9.4.1.2",
            "ACT",
            "RGAAv4",
            "RGAA-11.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Form elements must have labels",
        description: "Associate every <input> and <textarea> with a <label>, aria-label, aria-labelledby, or title.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/name-role-value.html",
    };
}

impl Rule for FormLabel {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        self.run_with_options(doc, &AuditOptions::default(), out);
    }

    fn run_with_options(&self, doc: &Document, options: &AuditOptions, out: &mut Vec<Violation>) {
        let allow_placeholder = options.profile == ConformanceProfile::W3cAct;
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if is_effectively_presentational(el) {
                continue;
            }
            let needs_label = match el.tag.as_str() {
                "textarea" => true,
                "input" => {
                    let ty = el
                        .attr("type")
                        .map_or("text", str::trim)
                        .to_ascii_lowercase();
                    !matches!(
                        ty.as_str(),
                        "hidden" | "button" | "submit" | "reset" | "image"
                    )
                }
                _ => false,
            };
            if !needs_label {
                continue;
            }
            if !accessible_name(doc, el.id).is_empty() {
                continue;
            }
            if allow_placeholder
                && el
                    .attr("placeholder")
                    .is_some_and(|p| !normalize_whitespace(p).is_empty())
            {
                continue;
            }
            out.push(Violation::from_node(&self.meta(), doc, el.id));
        }
    }
}

impl SelectName {
    pub const META: RuleMeta = RuleMeta {
        id: "select-name",
        impact: Impact::Critical,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.forms",
            "wcag2a",
            "wcag412",
            "section508",
            "section508.22.n",
            "TTv5",
            "TT5.c",
            "EN-301-549",
            "EN-9.4.1.2",
            "ACT",
            "RGAAv4",
            "RGAA-11.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "<select> elements must have an accessible name",
        description: "Associate every <select> element with a <label>, aria-label, aria-labelledby, or title.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/name-role-value.html",
    };
}

impl Rule for SelectName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "select" || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if is_effectively_presentational(el) {
                continue;
            }
            if accessible_name(doc, el.id).is_empty() {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

fn is_labelable_form_control(el: &Node) -> bool {
    match el.tag.as_str() {
        "select" | "textarea" => true,
        "input" => {
            let ty = el
                .attr("type")
                .map_or("text", str::trim)
                .to_ascii_lowercase();
            !matches!(
                ty.as_str(),
                "hidden" | "button" | "submit" | "reset" | "image"
            )
        }
        _ => false,
    }
}

fn associated_label_ids(doc: &Document, control: &Node) -> BTreeSet<NodeId> {
    let mut ids = BTreeSet::new();
    if let Some(control_id) = control.id_attr() {
        for &label_id in doc.labels_for(control_id) {
            if doc.is_in_accessibility_tree(label_id) {
                ids.insert(label_id);
            }
        }
    }
    for anc_id in doc.ancestors(control.id) {
        if let Some(anc) = doc.node(anc_id)
            && anc.tag == "label"
            && doc.is_in_accessibility_tree(anc_id)
        {
            let points_elsewhere = anc
                .attr("for")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .is_some_and(|f| control.id_attr() != Some(f));
            if !points_elsewhere {
                ids.insert(anc_id);
            }
            break;
        }
    }
    ids
}

fn is_valid_autocomplete_value(raw: &str) -> bool {
    let tokens: Vec<String> = raw
        .split_ascii_whitespace()
        .map(str::to_ascii_lowercase)
        .collect();
    if tokens.is_empty() {
        return false;
    }
    if tokens.len() == 1
        && matches!(
            tokens[0].as_str(),
            "on" | "off"
                | "none"
                | "false"
                | "true"
                | "disabled"
                | "enabled"
                | "undefined"
                | "null"
                | "xon"
                | "xoff"
        )
    {
        return true;
    }
    let mut idx = 0usize;
    if idx < tokens.len() && tokens[idx].starts_with("section-") && tokens[idx].len() > 8 {
        idx += 1;
    }
    if idx < tokens.len() && matches!(tokens[idx].as_str(), "shipping" | "billing") {
        idx += 1;
    }
    if idx >= tokens.len() {
        return false;
    }
    let is_contact_type = matches!(
        tokens[idx].as_str(),
        "home" | "work" | "mobile" | "fax" | "pager"
    );
    if is_contact_type {
        idx += 1;
        if idx >= tokens.len() || !is_contact_autofill_field(&tokens[idx]) {
            return false;
        }
        idx += 1;
    } else if is_normal_autofill_field(&tokens[idx]) || is_contact_autofill_field(&tokens[idx]) {
        idx += 1;
    } else {
        return false;
    }
    if idx < tokens.len() && tokens[idx] == "webauthn" {
        idx += 1;
    }
    idx == tokens.len()
}

fn is_normal_autofill_field(token: &str) -> bool {
    matches!(
        token,
        "name"
            | "honorific-prefix"
            | "given-name"
            | "additional-name"
            | "family-name"
            | "honorific-suffix"
            | "nickname"
            | "organization-title"
            | "username"
            | "new-password"
            | "current-password"
            | "one-time-code"
            | "organization"
            | "street-address"
            | "address-line1"
            | "address-line2"
            | "address-line3"
            | "address-level4"
            | "address-level3"
            | "address-level2"
            | "address-level1"
            | "country"
            | "country-name"
            | "postal-code"
            | "cc-name"
            | "cc-given-name"
            | "cc-additional-name"
            | "cc-family-name"
            | "cc-number"
            | "cc-exp"
            | "cc-exp-month"
            | "cc-exp-year"
            | "cc-csc"
            | "cc-type"
            | "transaction-currency"
            | "transaction-amount"
            | "language"
            | "bday"
            | "bday-day"
            | "bday-month"
            | "bday-year"
            | "sex"
            | "url"
            | "photo"
    )
}

fn is_contact_autofill_field(token: &str) -> bool {
    matches!(
        token,
        "tel"
            | "tel-country-code"
            | "tel-national"
            | "tel-area-code"
            | "tel-local"
            | "tel-local-prefix"
            | "tel-local-suffix"
            | "tel-extension"
            | "email"
            | "impp"
    )
}

impl AutocompleteValid {
    pub const META: RuleMeta = RuleMeta {
        id: "autocomplete-valid",
        impact: Impact::Serious,
        wcag: "1.3.5 (AA)",
        tags: &[
            "cat.forms",
            "wcag21aa",
            "wcag135",
            "EN-301-549",
            "EN-9.1.3.5",
            "ACT",
            "RGAAv4",
            "RGAA-11.13.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "autocomplete attribute must be used correctly",
        description: "Ensure autocomplete attributes on form fields contain valid WHATWG HTML autofill tokens.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/identify-input-purpose.html",
    };
}

impl Rule for AutocompleteValid {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !is_labelable_form_control(el) || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if el.has_attr("disabled")
                || el
                    .attr("aria-disabled")
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
            {
                continue;
            }
            let Some(raw) = el
                .attr("autocomplete")
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            if !is_valid_autocomplete_value(raw) {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!("Invalid autocomplete attribute value '{raw}'."),
                ));
            }
        }
    }
}

impl FormFieldMultipleLabels {
    pub const META: RuleMeta = RuleMeta {
        id: "form-field-multiple-labels",
        impact: Impact::Moderate,
        wcag: "3.3.2 (A)",
        tags: &[
            "cat.forms",
            "wcag2a",
            "wcag332",
            "TTv5",
            "TT5.c",
            "EN-301-549",
            "EN-9.3.3.2",
            "RGAAv4",
            "RGAA-11.2.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Form field must not have multiple label elements",
        description: "Associate each form control with at most one <label> element.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/labels-or-instructions.html",
    };
}

impl Rule for FormFieldMultipleLabels {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !is_labelable_form_control(el) || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let labels = associated_label_ids(doc, el);
            if labels.len() > 1 {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!(
                        "Form control has {} associated <label> elements.",
                        labels.len()
                    ),
                ));
            }
        }
    }
}

impl LabelContentNameMismatch {
    pub const META: RuleMeta = RuleMeta {
        id: "label-content-name-mismatch",
        impact: Impact::Serious,
        wcag: "2.5.3 (A)",
        tags: &[
            "cat.semantics",
            "wcag21a",
            "wcag253",
            "EN-301-549",
            "EN-9.2.5.3",
            "RGAAv4",
            "RGAA-6.1.5",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Elements must have their visible text as part of their accessible name",
        description: "When an interactive element has visible text and aria-label or aria-labelledby, the accessible name must contain the visible text.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/label-in-name.html",
    };
}

impl Rule for LabelContentNameMismatch {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let style_hints = LabelStyleHints::from_doc(doc);
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if !el.has_attr("aria-label") && !el.has_attr("aria-labelledby") {
                continue;
            }
            let role = effective_role(doc, el);
            let supports_content_label = is_interactive_control(doc, el)
                || matches!(
                    role,
                    Some(
                        "button"
                            | "checkbox"
                            | "link"
                            | "menuitem"
                            | "menuitemcheckbox"
                            | "menuitemradio"
                            | "option"
                            | "radio"
                            | "switch"
                            | "tab"
                            | "treeitem"
                    )
                );
            if !supports_content_label {
                continue;
            }
            if style_hints.is_icon_font(el) {
                continue;
            }
            let raw_visible = normalize_whitespace(&collect_visible_text(doc, el.id, &style_hints));
            if raw_visible.is_empty() {
                continue;
            }
            // WCAG 2.5.3: single non-digit icon glyphs (e.g. "X", "+", "×") are non-text UI symbols.
            if raw_visible.chars().count() == 1
                && raw_visible
                    .chars()
                    .next()
                    .is_some_and(|c| !c.is_ascii_digit())
            {
                continue;
            }
            let visible = normalize_for_label_match(&raw_visible);
            if visible.is_empty() {
                continue;
            }
            let acc = normalize_for_label_match(&accessible_name(doc, el.id));
            if !acc.is_empty() && !contains_word_sequence(&acc, &visible) {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!(
                        "Accessible name '{acc}' does not contain visible label text '{visible}'."
                    ),
                ));
            }
        }
    }
}

#[derive(Default)]
struct LabelStyleHints {
    inline_tags: BTreeSet<String>,
    inline_classes: BTreeSet<String>,
    hidden_classes: BTreeSet<String>,
    icon_font_tags: BTreeSet<String>,
    icon_font_classes: BTreeSet<String>,
}

impl LabelStyleHints {
    fn from_doc(doc: &Document) -> Self {
        let mut hints = Self::default();
        for el in doc.elements() {
            if el.tag != "style" {
                continue;
            }
            let css = strip_css_comments(&collect_raw_text(doc, el.id));
            for rule in css.split('}') {
                let Some((sel_list, decls)) = rule.split_once('{') else {
                    continue;
                };
                let is_inline = css_decl_is_inline(decls);
                let is_hidden = css_decls_visually_hidden(decls);
                let is_icon = css_decl_is_icon_font(decls);
                if !is_inline && !is_hidden && !is_icon {
                    continue;
                }
                for sel in sel_list.split(',') {
                    let s = sel.trim();
                    if let Some(cls) = s.strip_prefix('.')
                        && !cls.is_empty()
                        && cls
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                    {
                        let cls_lower = cls.to_ascii_lowercase();
                        if is_inline {
                            hints.inline_classes.insert(cls_lower.clone());
                        }
                        if is_hidden {
                            hints.hidden_classes.insert(cls_lower.clone());
                        }
                        if is_icon {
                            hints.icon_font_classes.insert(cls_lower);
                        }
                    } else if (is_inline || is_icon)
                        && !s.is_empty()
                        && s.bytes().all(|b| b.is_ascii_alphanumeric())
                    {
                        let tag_lower = s.to_ascii_lowercase();
                        if is_inline {
                            hints.inline_tags.insert(tag_lower.clone());
                        }
                        if is_icon {
                            hints.icon_font_tags.insert(tag_lower);
                        }
                    }
                }
            }
        }
        hints
    }

    fn is_icon_font(&self, node: &Node) -> bool {
        if self.icon_font_tags.contains(&node.tag) {
            return true;
        }
        if let Some(classes) = node.attr("class")
            && classes
                .split_ascii_whitespace()
                .any(|c| self.icon_font_classes.contains(&c.to_ascii_lowercase()))
        {
            return true;
        }
        node.attr("style").is_some_and(css_decl_is_icon_font)
    }

    fn is_visually_hidden(&self, node: &Node) -> bool {
        if node.computed.as_ref().and_then(|c| c.opacity) == Some(0.0) {
            return true;
        }
        if let Some(classes) = node.attr("class") {
            for cls in classes.split_ascii_whitespace() {
                let lower = cls.to_ascii_lowercase();
                if matches!(
                    lower.as_str(),
                    "sr-only"
                        | "visually-hidden"
                        | "visuallyhidden"
                        | "screen-reader-text"
                        | "a11y-hidden"
                ) || self.hidden_classes.contains(&lower)
                {
                    return true;
                }
            }
        }
        node.attr("style").is_some_and(css_decls_visually_hidden)
    }

    fn is_block_element(&self, node: &Node) -> bool {
        if let Some(style) = node.attr("style") {
            if css_decl_is_inline(style) {
                return false;
            }
            if css_decl_is_block(style) {
                return true;
            }
        }
        if let Some(disp) = node.computed.as_ref().and_then(|c| c.display.as_deref()) {
            let d = disp.trim().to_ascii_lowercase();
            if d == "inline" {
                return false;
            }
            if matches!(
                d.as_str(),
                "block" | "flex" | "grid" | "table" | "list-item"
            ) {
                return true;
            }
        }
        if self.inline_tags.contains(&node.tag) {
            return false;
        }
        if let Some(classes) = node.attr("class")
            && classes
                .split_ascii_whitespace()
                .any(|c| self.inline_classes.contains(&c.to_ascii_lowercase()))
        {
            return false;
        }
        is_block_tag(&node.tag)
    }
}

fn strip_css_comments(css: &str) -> String {
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

fn css_decl_is_icon_font(decls: &str) -> bool {
    for decl in decls.split(';') {
        let Some((prop, val)) = decl.split_once(':') else {
            continue;
        };
        if prop.trim().eq_ignore_ascii_case("font-family") {
            let v = val.to_ascii_lowercase();
            if v.contains("material icons")
                || v.contains("material symbols")
                || v.contains("font awesome")
                || v.contains("fontawesome")
                || v.contains("icomoon")
                || v.contains("glyphicons")
                || v.contains("ionicons")
                || v.contains("bootstrap-icons")
            {
                return true;
            }
        }
    }
    false
}

fn css_decl_is_inline(decls: &str) -> bool {
    let mut is_inline = false;
    for decl in decls.split(';') {
        let Some((prop, val)) = decl.split_once(':') else {
            continue;
        };
        if prop.trim().eq_ignore_ascii_case("display") {
            let v = val.split('!').next().map_or("", str::trim);
            is_inline = v.eq_ignore_ascii_case("inline");
        }
    }
    is_inline
}

fn css_decl_is_block(decls: &str) -> bool {
    let mut is_block = false;
    for decl in decls.split(';') {
        let Some((prop, val)) = decl.split_once(':') else {
            continue;
        };
        if prop.trim().eq_ignore_ascii_case("display") {
            let v = val
                .split('!')
                .next()
                .map_or("", str::trim)
                .to_ascii_lowercase();
            is_block = matches!(
                v.as_str(),
                "block" | "flex" | "grid" | "table" | "list-item"
            );
        }
    }
    is_block
}

fn css_decls_visually_hidden(decls: &str) -> bool {
    let mut pos_abs = false;
    let mut neg_offset = false;
    let mut tiny_w = false;
    let mut tiny_h = false;
    let mut overflow_hidden = false;

    for decl in decls.split(';') {
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
            "clip-path" if v.contains("inset(50%") || v.contains("inset(100%") => return true,
            "clip" if v.starts_with("rect(0") || v.starts_with("rect(1px") => return true,
            "opacity" if v == "0" || v == "0.0" => return true,
            "position" if matches!(v.as_str(), "absolute" | "fixed") => pos_abs = true,
            "left" | "top" if v.starts_with('-') => {
                let num_str = v.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '%');
                if num_str.parse::<f64>().is_ok_and(|n| n <= -100.0) {
                    neg_offset = true;
                }
            }
            "width" if matches!(v.as_str(), "1px" | "0" | "0px") => tiny_w = true,
            "height" if matches!(v.as_str(), "1px" | "0" | "0px") => tiny_h = true,
            "overflow" if v == "hidden" => overflow_hidden = true,
            _ => {}
        }
    }
    (pos_abs && neg_offset) || (tiny_w && tiny_h && overflow_hidden)
}

fn is_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "br"
            | "dd"
            | "div"
            | "dl"
            | "dt"
            | "figcaption"
            | "figure"
            | "footer"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "td"
            | "th"
            | "tr"
            | "ul"
    )
}

fn collect_visible_text(doc: &Document, id: NodeId, hints: &LabelStyleHints) -> String {
    let mut visited = HashSet::new();
    visited.insert(id);
    collect_visible_text_inner(doc, id, hints, 0, &mut visited)
}

fn collect_visible_text_inner(
    doc: &Document,
    id: NodeId,
    hints: &LabelStyleHints,
    depth: usize,
    visited: &mut HashSet<NodeId>,
) -> String {
    if depth >= MAX_TRAVERSAL_DEPTH {
        return String::new();
    }
    let Some(node) = doc.node(id) else {
        return String::new();
    };
    let mut out = String::new();
    for &cid in &node.children {
        if !visited.insert(cid) {
            continue;
        }
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if child.is_text() {
            out.push_str(&child.text);
            continue;
        }
        // Visually rendered text includes `aria-hidden="true"` subtrees (WCAG 2.5.3),
        // but skips non-rendered metadata tags (<svg><title>, <desc>, <style>, <script>)
        // and visually-hidden / sr-only / offscreen spans.
        if !child.is_element()
            || !doc.is_rendered(cid)
            || matches!(
                child.tag.as_str(),
                "title" | "desc" | "style" | "script" | "noscript" | "template"
            )
            || hints.is_visually_hidden(child)
            || hints.is_icon_font(child)
        {
            continue;
        }
        let block = hints.is_block_element(child);
        if block && !out.is_empty() && !out.ends_with(char::is_whitespace) {
            out.push(' ');
        }
        let sub = collect_visible_text_inner(doc, cid, hints, depth + 1, visited);
        out.push_str(&sub);
        if block && !out.is_empty() && !out.ends_with(char::is_whitespace) {
            out.push(' ');
        }
    }
    out
}

fn contains_word_sequence(haystack: &str, needle: &str) -> bool {
    let h_words: Vec<&str> = haystack.split_ascii_whitespace().collect();
    let n_words: Vec<&str> = needle.split_ascii_whitespace().collect();
    if n_words.is_empty() || n_words.len() > h_words.len() {
        return false;
    }
    h_words
        .windows(n_words.len())
        .any(|win| win == n_words.as_slice())
}

fn strip_parentheticals(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '(' {
            let mut depth = 1usize;
            let mut buf = String::new();
            buf.push(ch);
            for next_ch in chars.by_ref() {
                buf.push(next_ch);
                match next_ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if depth == 0 {
                out.push(' ');
            } else {
                // Unmatched '(' — keep original text.
                out.push_str(&buf);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn normalize_for_label_match(input: &str) -> String {
    let without_parens = strip_parentheticals(input);
    let cleaned: String = without_parens
        .chars()
        .map(|raw| {
            // Normalize Unicode fullwidth ASCII forms (U+FF01..=U+FF5E) and ideographic space (U+3000)
            // per NFKC compatibility folding.
            let c = match raw as u32 {
                0x3000 => ' ',
                0xFF01..=0xFF5E => char::from_u32(raw as u32 - 0xFEE0).unwrap_or(raw),
                _ => raw,
            };
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect();
    normalize_whitespace(&cleaned).to_ascii_lowercase()
}

impl LabelTitleOnly {
    pub const META: RuleMeta = RuleMeta {
        id: "label-title-only",
        impact: Impact::Serious,
        wcag: "3.3.2 (Best Practice)",
        tags: &["cat.forms", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Form elements should have a visible label",
        description: "Do not rely solely on title or aria-describedby to label form controls; provide a <label>, aria-label, or aria-labelledby.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/labels-or-instructions.html",
    };
}

impl Rule for LabelTitleOnly {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !is_labelable_form_control(el) || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if is_effectively_presentational(el) {
                continue;
            }
            let has_title_or_desc = el
                .attr("title")
                .is_some_and(|t| !normalize_whitespace(t).is_empty())
                || el
                    .attr("aria-describedby")
                    .is_some_and(|d| !d.trim().is_empty());
            if !has_title_or_desc {
                continue;
            }
            let has_aria_label = el
                .attr("aria-label")
                .is_some_and(|l| !normalize_whitespace(l).is_empty());
            let has_aria_labelledby = el.attr("aria-labelledby").is_some_and(|ids| {
                ids.split_ascii_whitespace()
                    .any(|id| doc.find_by_id(id).is_some())
            });
            let has_label_el = !associated_label_ids(doc, el).is_empty();
            if !has_aria_label && !has_aria_labelledby && !has_label_el {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl Accesskeys {
    pub const META: RuleMeta = RuleMeta {
        id: "accesskeys",
        impact: Impact::Serious,
        wcag: "2.4.1 (Best Practice)",
        tags: &["cat.keyboard", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "accesskey attribute value should be unique",
        description: "Ensure every accesskey attribute value is unique across the document.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/keyboard.html",
    };
}

impl Rule for Accesskeys {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let mut by_key: BTreeMap<String, Vec<NodeId>> = BTreeMap::new();
        for el in doc.elements() {
            if !doc.is_rendered(el.id) {
                continue;
            }
            if let Some(key) = el
                .attr("accesskey")
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                by_key
                    .entry(key.to_ascii_lowercase())
                    .or_default()
                    .push(el.id);
            }
        }
        for (key, nodes) in by_key {
            if nodes.len() > 1 {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    nodes[0],
                    format!("Duplicate accesskey='{key}' shared by multiple elements."),
                ));
            }
        }
    }
}
