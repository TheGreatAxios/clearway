//! Modular Clearway extension rules for modern web accessibility checks.
//!
//! Covers WCAG 2.2 authentication, WCAG 3.0 APCA contrast, modern HTML popover
//! validation, focus-indicator stripping, pointer-events traps, text-clipping
//! overflow, visual/DOM reading order reversal, and `prefers-reduced-motion`.
//! All rules in this module carry the `"clearway"` tag.

use std::collections::HashSet;

use crate::accname::collect_raw_text;
use crate::aria::is_sequentially_focusable;
use crate::contrast::{
    apca_lc, composite_over, effective_background, effective_foreground, is_large_text,
};
use crate::ir::{Document, NodeId};
use crate::rule::{Impact, Rule, RuleMeta, Violation};
use crate::rules::computed::{
    effective_gradient_stops, has_effective_background_image, is_inactive_ui_control_or_label,
    is_node_or_ancestor_disabled, is_non_text_icon_glyph, is_visually_hidden,
};

pub struct AccessibleAuthentication;
pub struct FocusOutlineNone;
pub struct PopoverTargetValid;
pub struct PointerEventsFocusable;
pub struct TextClippingOverflow;
pub struct ReadingOrderReverse;
pub struct ApcaContrast;
pub struct PrefersReducedMotion;

// ---------------------------------------------------------------------------
// 1. accessible-authentication
// ---------------------------------------------------------------------------

impl AccessibleAuthentication {
    pub const META: RuleMeta = RuleMeta {
        id: "accessible-authentication",
        impact: Impact::Serious,
        wcag: "3.3.8 (AA)",
        tags: &["cat.forms", "wcag22aa", "wcag338", "clearway"],
        enabled_by_default: true,
        needs_review: false,
        help: "Password inputs must not block paste",
        description: "Ensure <input type=\"password\"> elements do not block pasting via onpaste handlers so password managers and users with cognitive disabilities can sign in.",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/accessible-authentication-minimum.html",
    };
}

fn handler_blocks_paste(handler: &str) -> bool {
    let lower = handler.to_ascii_lowercase();
    if lower.contains("preventdefault") {
        return true;
    }
    let mut rest = lower.as_str();
    while let Some(pos) = rest.find("return") {
        let after = &rest[pos + "return".len()..];
        let trimmed = after.trim_start();
        if let Some(after_false) = trimmed.strip_prefix("false") {
            let next_ok = after_false
                .chars()
                .next()
                .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_');
            if next_ok {
                return true;
            }
        }
        rest = after;
    }
    false
}

impl Rule for AccessibleAuthentication {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) || el.tag != "input" {
                continue;
            }
            let is_password = el
                .attr("type")
                .is_some_and(|t| t.trim().eq_ignore_ascii_case("password"));
            if !is_password {
                continue;
            }
            if let Some(onpaste) = el.attr("onpaste")
                && handler_blocks_paste(onpaste)
            {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    "Password input blocks paste via onpaste handler, failing WCAG 2.2 Accessible Authentication.",
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 2. focus-outline-none
// ---------------------------------------------------------------------------

impl FocusOutlineNone {
    pub const META: RuleMeta = RuleMeta {
        id: "focus-outline-none",
        impact: Impact::Serious,
        wcag: "2.4.7 (AA)",
        tags: &["cat.keyboard", "wcag2aa", "wcag247", "clearway"],
        enabled_by_default: true,
        needs_review: false,
        help: "Focus styles must not remove outlines without a visible replacement",
        description: "Ensure :focus CSS rules that set outline: none or outline: 0 provide an alternative focus indicator such as box-shadow, border, or background.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/focus-visible.html",
    };
}

/// Strips `/* ... */` CSS block comments while preserving remaining text.
fn strip_css_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find("*/") {
            rest = &after_open[end + 2..];
        } else {
            return out;
        }
    }
    out.push_str(rest);
    out
}

/// Returns `true` if `selector_list` contains a `:focus` pseudo-class that is
/// not `:focus-visible` or `:focus-within`.
fn selector_has_plain_focus(selector_list: &str) -> bool {
    let lower = selector_list.to_ascii_lowercase();
    let mut rest = lower.as_str();
    while let Some(pos) = rest.find(":focus") {
        let after = &rest[pos + ":focus".len()..];
        let next_is_ident = after
            .as_bytes()
            .first()
            .is_some_and(|&b| b == b'-' || b == b'_' || b.is_ascii_alphanumeric());
        if !next_is_ident {
            return true;
        }
        rest = after;
    }
    false
}

/// Returns `true` if `decls` removes the focus outline (`outline: none` or
/// `outline: 0`) without providing a `box-shadow`, `border`, or `background`
/// declaration in the same block.
fn decls_remove_outline_without_replacement(decls: &str) -> bool {
    let mut removes_outline = false;
    let mut has_replacement = false;

    for decl in decls.split(';') {
        let Some((raw_prop, raw_val)) = decl.split_once(':') else {
            continue;
        };
        let prop = raw_prop.trim().to_ascii_lowercase();
        let val = raw_val
            .split('!')
            .next()
            .map_or("", str::trim)
            .to_ascii_lowercase();

        if (prop == "outline" && matches!(val.as_str(), "none" | "0" | "0px"))
            || (prop == "outline-style" && val == "none")
            || (prop == "outline-width" && matches!(val.as_str(), "0" | "0px"))
        {
            removes_outline = true;
        }

        if (prop == "box-shadow" && val != "none")
            || ((prop == "border" || prop.starts_with("border-"))
                && !matches!(val.as_str(), "none" | "0" | "0px"))
            || ((prop == "background" || prop.starts_with("background-"))
                && !matches!(val.as_str(), "none" | "transparent"))
        {
            has_replacement = true;
        }
    }

    removes_outline && !has_replacement
}

const MAX_CSS_DEPTH: usize = 64;

/// Scans a CSS block (descending into `@media`/`@supports`/`@layer` blocks) and
/// returns the first offending `:focus` selector that strips outlines.
fn find_focus_outline_none_selector(css: &str, depth: usize) -> Option<String> {
    if depth >= MAX_CSS_DEPTH {
        return None;
    }
    let bytes = css.as_bytes();
    let len = bytes.len();
    let mut idx = 0usize;

    while idx < len {
        // Find next `{` or `;` at top level of this slice.
        let prelude_start = idx;
        while idx < len && bytes[idx] != b'{' && bytes[idx] != b';' {
            idx += 1;
        }
        if idx >= len {
            break;
        }
        if bytes[idx] == b';' {
            idx += 1;
            continue;
        }
        let prelude = css[prelude_start..idx].trim();
        // Match balanced braces for `{ ... }`.
        idx += 1;
        let body_start = idx;
        let mut brace_depth = 1usize;
        while idx < len && brace_depth > 0 {
            match bytes[idx] {
                b'{' => brace_depth += 1,
                b'}' => brace_depth -= 1,
                _ => {}
            }
            idx += 1;
        }
        let body_end = if brace_depth == 0 { idx - 1 } else { idx };
        let body = &css[body_start..body_end];

        if prelude.starts_with('@') {
            if let Some(found) = find_focus_outline_none_selector(body, depth + 1) {
                return Some(found);
            }
        } else if selector_has_plain_focus(prelude)
            && decls_remove_outline_without_replacement(body)
        {
            return Some(prelude.to_owned());
        }
    }
    None
}

impl Rule for FocusOutlineNone {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "style" {
                continue;
            }
            let raw = collect_raw_text(doc, el.id);
            let cleaned = strip_css_comments(&raw);
            if let Some(selector) = find_focus_outline_none_selector(&cleaned, 0) {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!(
                        "CSS rule \"{selector}\" removes focus outline without providing box-shadow, border, or background."
                    ),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 3. popover-target-valid
// ---------------------------------------------------------------------------

impl PopoverTargetValid {
    pub const META: RuleMeta = RuleMeta {
        id: "popover-target-valid",
        impact: Impact::Serious,
        wcag: "Best Practice",
        tags: &["cat.semantics", "best-practice", "clearway"],
        enabled_by_default: true,
        needs_review: false,
        help: "popovertarget must reference an existing element with a popover attribute",
        description: "Ensure elements with popovertarget reference an existing DOM element that declares the popover attribute.",
        docs_url: "https://html.spec.whatwg.org/multipage/popover.html#attr-popovertarget",
    };
}

impl Rule for PopoverTargetValid {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let Some(raw_target) = el.attr("popovertarget") else {
                continue;
            };
            let target_id = raw_target.trim();
            match doc.find_by_id(target_id) {
                None => {
                    out.push(Violation::with_detail(
                        &self.meta(),
                        doc,
                        el.id,
                        format!(
                            "popovertarget references \"#{target_id}\", which does not exist in the document."
                        ),
                    ));
                }
                Some(tid) => {
                    if doc.node(tid).is_some_and(|n| !n.has_attr("popover")) {
                        out.push(Violation::with_detail(
                            &self.meta(),
                            doc,
                            el.id,
                            format!(
                                "popovertarget references \"#{target_id}\", which does not have a popover attribute."
                            ),
                        ));
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 4. pointer-events-focusable
// ---------------------------------------------------------------------------

impl PointerEventsFocusable {
    pub const META: RuleMeta = RuleMeta {
        id: "pointer-events-focusable",
        impact: Impact::Moderate,
        wcag: "Best Practice",
        tags: &["cat.keyboard", "best-practice", "clearway"],
        enabled_by_default: true,
        needs_review: false,
        help: "Focusable elements must not have pointer-events: none",
        description: "Ensure keyboard-focusable interactive elements do not set pointer-events: none, which breaks pointer and touch activation.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/concurrent-input-mechanisms.html",
    };
}

impl Rule for PointerEventsFocusable {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id)
                || !is_sequentially_focusable(el)
                || is_node_or_ancestor_disabled(doc, el.id)
            {
                continue;
            }
            let has_pointer_events_none = el
                .computed
                .as_ref()
                .and_then(|c| c.pointer_events.as_deref())
                .is_some_and(|pe| pe.trim().eq_ignore_ascii_case("none"));
            if has_pointer_events_none {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!(
                        "Focusable <{}> element has computed pointer-events: none.",
                        el.tag
                    ),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 5. text-clipping-overflow
// ---------------------------------------------------------------------------

impl TextClippingOverflow {
    pub const META: RuleMeta = RuleMeta {
        id: "text-clipping-overflow",
        impact: Impact::Moderate,
        wcag: "1.4.4 (AA)",
        tags: &["cat.text-alternatives", "wcag2aa", "wcag144", "clearway"],
        enabled_by_default: false,
        needs_review: false,
        help: "Text-bearing elements must not clip overflowing content vertically",
        description: "Ensure elements containing text do not clip vertical overflow with overflow-y: hidden when scrollHeight exceeds clientHeight.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/resize-text.html",
    };
}

impl Rule for TextClippingOverflow {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let Some(computed) = &el.computed else {
                continue;
            };
            let is_hidden_y = computed
                .overflow_y
                .as_deref()
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("hidden"));
            if !is_hidden_y {
                continue;
            }
            let (Some(scroll_h), Some(client_h)) = (computed.scroll_height, computed.client_height)
            else {
                continue;
            };
            if scroll_h > client_h + 2.0 && !collect_raw_text(doc, el.id).trim().is_empty() {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!(
                        "Element clips text vertically with overflow-y: hidden (scrollHeight {scroll_h:.0}px > clientHeight {client_h:.0}px)."
                    ),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 6. reading-order-reverse
// ---------------------------------------------------------------------------

impl ReadingOrderReverse {
    pub const META: RuleMeta = RuleMeta {
        id: "reading-order-reverse",
        impact: Impact::Moderate,
        wcag: "1.3.2 (A)",
        tags: &["cat.structure", "wcag2a", "wcag132", "clearway"],
        enabled_by_default: false,
        needs_review: false,
        help: "Flex containers with reversed direction should not reorder focusable descendants",
        description: "Ensure containers using flex-direction: row-reverse or column-reverse do not invert the visual and keyboard reading order of focusable controls.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/meaningful-sequence.html",
    };
}

fn count_focusable_descendants(doc: &Document, id: NodeId) -> usize {
    let mut visited = HashSet::new();
    visited.insert(id);
    let mut stack: Vec<NodeId> = doc.node(id).map_or_default(|n| n.children.clone());
    let mut count = 0usize;
    while let Some(cid) = stack.pop() {
        if !visited.insert(cid) {
            continue;
        }
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if !child.is_element() || !doc.is_in_accessibility_tree(cid) {
            continue;
        }
        if is_sequentially_focusable(child) {
            count += 1;
        }
        stack.extend(&child.children);
    }
    count
}

impl Rule for ReadingOrderReverse {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let Some(direction) = el
                .computed
                .as_ref()
                .and_then(|c| c.flex_direction.as_deref())
                .map(str::trim)
            else {
                continue;
            };
            if !(direction.eq_ignore_ascii_case("row-reverse")
                || direction.eq_ignore_ascii_case("column-reverse"))
            {
                continue;
            }
            let focusable = count_focusable_descendants(doc, el.id);
            if focusable >= 2 {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!(
                        "Container uses flex-direction: {direction} with {focusable} focusable descendants, reversing visual and DOM order."
                    ),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 7. apca-contrast
// ---------------------------------------------------------------------------

impl ApcaContrast {
    pub const META: RuleMeta = RuleMeta {
        id: "apca-contrast",
        impact: Impact::Serious,
        wcag: "WCAG 3.0 Draft (APCA)",
        tags: &["cat.color", "wcag3-draft", "clearway"],
        enabled_by_default: false,
        needs_review: false,
        help: "Text must meet WCAG 3.0 APCA perceptual lightness contrast thresholds",
        description: "Text must achieve at least |Lc| >= 60 for body text or |Lc| >= 45 for large text (>= 24px or >= 18.66px bold) using APCA-W3 0.0.98G-4g.",
        docs_url: "https://www.w3.org/TR/wcag-3.0/",
    };
}

impl Rule for ApcaContrast {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    #[allow(clippy::too_many_lines)]
    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let meta = self.meta();
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id)
                || !doc.has_direct_text(el.id)
                || is_visually_hidden(doc, el.id)
                || is_inactive_ui_control_or_label(doc, el.id)
                || is_non_text_icon_glyph(doc, el)
            {
                continue;
            }
            let Some(computed) = &el.computed else {
                continue;
            };
            if computed.color.is_none() {
                continue;
            }
            let bg = effective_background(doc, el.id);
            let size = computed.font_size_px.unwrap_or(16.0);
            let weight = computed.font_weight.unwrap_or(400);
            let threshold = if is_large_text(size, weight) {
                45.0
            } else {
                60.0
            };

            if let Some(stops) = effective_gradient_stops(doc, el.id) {
                let mut all_pass = true;
                let mut all_fail = true;
                let mut min_lc = f64::INFINITY;
                let mut max_lc = 0.0_f64;
                let mut evaluated = 0usize;

                for stop in stops {
                    let backdrop = composite_over(stop, bg);
                    let Some(foreground) = effective_foreground(doc, el.id, backdrop) else {
                        continue;
                    };
                    let lc_abs = apca_lc(foreground, backdrop).abs();
                    evaluated += 1;
                    min_lc = min_lc.min(lc_abs);
                    max_lc = max_lc.max(lc_abs);
                    if lc_abs < threshold {
                        all_pass = false;
                    } else {
                        all_fail = false;
                    }
                }

                if evaluated > 0 {
                    if all_pass {
                        continue;
                    }
                    if all_fail {
                        let bg_image = has_effective_background_image(doc, el.id);
                        let v = if computed.has_text_shadow || bg_image {
                            Violation::with_detail(
                                &meta,
                                doc,
                                el.id,
                                format!(
                                    "APCA contrast across all CSS gradient stops (|Lc| {min_lc:.1}-{max_lc:.1}) is below minimum {threshold:.0}; requires visual review."
                                ),
                            )
                            .into_incomplete()
                        } else {
                            Violation::with_detail(
                                &meta,
                                doc,
                                el.id,
                                format!(
                                    "APCA contrast across all CSS gradient stops (|Lc| {min_lc:.1}-{max_lc:.1}) is below minimum {threshold:.0}."
                                ),
                            )
                        };
                        out.push(v);
                    } else {
                        out.push(
                            Violation::with_detail(
                                &meta,
                                doc,
                                el.id,
                                format!(
                                    "APCA contrast across CSS gradient stops ranges from |Lc| {min_lc:.1} to {max_lc:.1} (minimum {threshold:.0}); requires visual review."
                                ),
                            )
                            .into_incomplete(),
                        );
                    }
                    continue;
                }
            }

            let Some(fg) = effective_foreground(doc, el.id, bg) else {
                continue;
            };
            let lc = apca_lc(fg, bg);
            if lc.abs() < threshold {
                let bg_image = has_effective_background_image(doc, el.id);
                let v = if computed.has_text_shadow || bg_image {
                    let reason = match (computed.has_text_shadow, bg_image) {
                        (true, true) => "text-shadow and background-image present",
                        (true, false) => "text-shadow present",
                        (false, true) => "background-image present",
                        (false, false) => unreachable!(),
                    };
                    Violation::with_detail(
                        &meta,
                        doc,
                        el.id,
                        format!(
                            "APCA contrast |Lc| = {:.1} (Lc {lc:.1}) is below minimum {threshold:.0} (fg rgb({},{},{}), bg rgb({},{},{})); {reason}, requires visual review.",
                            lc.abs(),
                            fg.r,
                            fg.g,
                            fg.b,
                            bg.r,
                            bg.g,
                            bg.b
                        ),
                    )
                    .into_incomplete()
                } else {
                    Violation::with_detail(
                        &meta,
                        doc,
                        el.id,
                        format!(
                            "APCA contrast |Lc| = {:.1} (Lc {lc:.1}) is below minimum {threshold:.0} (fg rgb({},{},{}), bg rgb({},{},{})).",
                            lc.abs(),
                            fg.r,
                            fg.g,
                            fg.b,
                            bg.r,
                            bg.g,
                            bg.b
                        ),
                    )
                };
                out.push(v);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 8. prefers-reduced-motion
// ---------------------------------------------------------------------------

impl PrefersReducedMotion {
    pub const META: RuleMeta = RuleMeta {
        id: "prefers-reduced-motion",
        impact: Impact::Minor,
        wcag: "2.3.3 (AAA)",
        tags: &[
            "cat.sensory-and-visual-cues",
            "wcag21aaa",
            "wcag233",
            "clearway",
        ],
        enabled_by_default: false,
        needs_review: false,
        help: "CSS animations should respect prefers-reduced-motion",
        description: "Ensure stylesheets declaring @keyframes or CSS animations include a @media (prefers-reduced-motion) query to reduce or disable motion.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/animation-from-interactions.html",
    };
}

fn css_has_reduced_motion_query(lower_css: &str) -> bool {
    let mut rest = lower_css;
    while let Some(pos) = rest.find("@media") {
        let after = &rest[pos + "@media".len()..];
        let cond_end = after.find('{').unwrap_or(after.len());
        if after[..cond_end].contains("prefers-reduced-motion") {
            return true;
        }
        rest = &after[cond_end..];
    }
    false
}

fn css_declares_animation(lower_css: &str) -> bool {
    if lower_css.contains("@keyframes") {
        return true;
    }
    let mut rest = lower_css;
    while let Some(pos) = rest.find("animation") {
        let before_ok = pos == 0
            || !lower_css.as_bytes()[lower_css.len() - rest.len() + pos - 1]
                .is_ascii_alphanumeric();
        let after = &rest[pos + "animation".len()..];
        let trimmed = after.trim_start();
        if before_ok && let Some(after_colon) = trimmed.strip_prefix(':') {
            let val_end = after_colon.find([';', '}']).unwrap_or(after_colon.len());
            let val = after_colon[..val_end]
                .split('!')
                .next()
                .map_or("", str::trim);
            if !val.is_empty() && val != "none" {
                return true;
            }
        }
        rest = after;
    }
    false
}

impl Rule for PrefersReducedMotion {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let styles: Vec<(NodeId, String)> = doc
            .elements()
            .filter(|el| el.tag == "style")
            .map(|el| {
                let raw = collect_raw_text(doc, el.id);
                (el.id, strip_css_comments(&raw).to_ascii_lowercase())
            })
            .collect();

        if styles
            .iter()
            .any(|(_, css)| css_has_reduced_motion_query(css))
        {
            return;
        }

        for (style_id, css) in &styles {
            if css_declares_animation(css) {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    *style_id,
                    "Stylesheet declares CSS animations without a @media (prefers-reduced-motion) rule.",
                ));
            }
        }
    }
}
