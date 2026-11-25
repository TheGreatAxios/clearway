//! Computed-layout, style, and runtime rules: `color-contrast`,
//! `color-contrast-enhanced`, `css-orientation-lock`, `focus-order-semantics`,
//! `frame-focusable-content`, `frame-tested`, `hidden-content`,
//! `link-in-text-block`, `no-autoplay-audio`, `p-as-heading`,
//! `scrollable-region-focusable`, and `target-size`.
//!
//! These rules are browser-agnostic: they operate on optional [`ComputedStyle`],
//! [`Rect`], frame, and media snapshots attached to IR nodes. When those fields
//! are absent (e.g. raw static HTML), computed-style rules cleanly treat those
//! nodes as inapplicable.

use std::collections::{HashMap, HashSet};

use crate::accname::collect_raw_text;
use crate::aria::{
    effective_role, is_interactive_control, is_natively_focusable, is_sequentially_focusable,
};
use crate::contrast::{
    aa_threshold, aaa_threshold, composite_over, contrast_ratio, effective_background,
    effective_foreground,
};
use crate::ir::{ComputedStyle, Document, Node, NodeId, Rect, Rgba, normalize_whitespace};
use crate::rule::{Impact, Rule, RuleMeta, Violation};

/// WCAG 2.2 SC 2.5.8 AA minimum interactive target dimension in CSS pixels.
pub const MIN_TARGET_SIZE_PX: f32 = 24.0;

/// WCAG G183 minimum contrast ratio between a non-underlined link and surrounding text.
const LINK_IN_TEXT_MIN_CONTRAST: f64 = 3.0;

/// WCAG 2.1 SC 1.4.2 maximum allowed autoplay audio duration in seconds without controls.
const MAX_AUTOPLAY_SECS: f32 = 3.0;

pub struct ColorContrast;
pub struct ColorContrastEnhanced;
pub struct CssOrientationLock;
pub struct FocusOrderSemantics;
pub struct FrameFocusableContent;
pub struct FrameTested;
pub struct HiddenContent;
pub struct LinkInTextBlock;
pub struct NoAutoplayAudio;
pub struct PAsHeading;
pub struct ScrollableRegionFocusable;
pub struct TargetSize;

impl ColorContrast {
    pub const META: RuleMeta = RuleMeta {
        id: "color-contrast",
        impact: Impact::Serious,
        wcag: "1.4.3 (AA)",
        tags: &[
            "cat.color",
            "wcag2aa",
            "wcag143",
            "TTv5",
            "TT13.c",
            "EN-301-549",
            "EN-9.1.4.3",
            "ACT",
            "RGAAv4",
            "RGAA-3.2.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Elements must meet minimum WCAG 2.1 AA color contrast ratio thresholds",
        description: "Text must have at least 4.5:1 contrast against its background (3.0:1 for large text >= 24px or >= 18.66px bold).",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum.html",
    };
}

fn is_aria_disabled_true(node: &Node) -> bool {
    node.attr("aria-disabled")
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

pub(crate) fn is_node_or_ancestor_disabled(doc: &Document, id: NodeId) -> bool {
    let mut current = Some(id);
    let mut depth = 0usize;
    while let Some(nid) = current {
        if depth > doc.nodes.len() {
            break;
        }
        depth += 1;
        let Some(node) = doc.node(nid) else {
            break;
        };
        if node.has_attr("disabled") || is_aria_disabled_true(node) {
            return true;
        }
        current = node.parent;
    }
    false
}

fn find_first_labelable_descendant(doc: &Document, id: NodeId) -> Option<NodeId> {
    let mut visited = HashSet::new();
    visited.insert(id);
    let mut stack: Vec<NodeId> = doc
        .node(id)
        .map_or_default(|n| n.children.iter().rev().copied().collect());
    while let Some(cid) = stack.pop() {
        if !visited.insert(cid) {
            continue;
        }
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if !child.is_element() {
            continue;
        }
        let is_labelable = matches!(
            child.tag.as_str(),
            "button" | "meter" | "output" | "progress" | "select" | "textarea"
        ) || (child.tag == "input"
            && !child
                .attr("type")
                .is_some_and(|t| t.trim().eq_ignore_ascii_case("hidden")));
        if is_labelable {
            return Some(cid);
        }
        for &gcid in child.children.iter().rev() {
            stack.push(gcid);
        }
    }
    None
}

fn is_label_for_disabled_control(doc: &Document, label_node: &Node) -> bool {
    if label_node.tag != "label" {
        return false;
    }
    if let Some(for_id) = label_node
        .attr("for")
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return doc
            .find_by_id(for_id)
            .is_some_and(|target| is_node_or_ancestor_disabled(doc, target));
    }
    find_first_labelable_descendant(doc, label_node.id)
        .is_some_and(|target| is_node_or_ancestor_disabled(doc, target))
}

fn is_only_labelling_disabled_controls(doc: &Document, id: NodeId) -> bool {
    let mut current = Some(id);
    let mut depth = 0usize;
    while let Some(cand_id) = current {
        if depth > doc.nodes.len() {
            break;
        }
        depth += 1;
        let Some(node) = doc.node(cand_id) else {
            break;
        };
        if let Some(el_id) = node.id_attr() {
            let referrers = doc.aria_labelledby_referrers(el_id);
            if !referrers.is_empty()
                && referrers
                    .iter()
                    .all(|&ref_id| is_node_or_ancestor_disabled(doc, ref_id))
            {
                return true;
            }
        }
        current = node.parent;
    }
    false
}

/// WCAG 1.4.3 / 1.4.6 inactive user interface component exemption.
pub(crate) fn is_inactive_ui_control_or_label(doc: &Document, id: NodeId) -> bool {
    if is_node_or_ancestor_disabled(doc, id) {
        return true;
    }
    let mut current = Some(id);
    let mut depth = 0usize;
    while let Some(nid) = current {
        if depth > doc.nodes.len() {
            break;
        }
        depth += 1;
        let Some(node) = doc.node(nid) else {
            break;
        };
        if is_label_for_disabled_control(doc, node) {
            return true;
        }
        current = node.parent;
    }
    is_only_labelling_disabled_controls(doc, id)
}

fn is_color_emoji_char(c: char) -> bool {
    matches!(c, '\u{1F300}'..='\u{1FAFF}' | '\u{1F000}'..='\u{1F02F}')
}

/// WCAG 1.4.3 / 1.4.6 non-text UI icon glyph exemption:
/// 1. Full-color Unicode emoji glyphs (e.g. `💡`, U+1F300..=U+1FAFF and U+1F000..=U+1F02F)
///    are rendered with multi-color bitmap/COLR palettes unaffected by CSS `color`
///    (while monochrome BMP symbols like `★`/`☆` U+2605/U+2606 are still checked).
/// 2. When an element has an explicit `aria-label` or `aria-labelledby` whose computed
///    accessible name differs from its single-character visible text (e.g. `"X"`, `"+"`,
///    `"×"`, `"?"`, `"-"` used as an icon glyph), WCAG treats the glyph as non-text UI
///    iconography (WCAG 1.4.11 3:1) rather than human-language text (4.5:1 / 7:1).
pub(crate) fn is_non_text_icon_glyph(doc: &Document, node: &Node) -> bool {
    let visible = normalize_whitespace(&collect_raw_text(doc, node.id));
    if !visible.is_empty()
        && visible
            .chars()
            .all(|c| is_color_emoji_char(c) || matches!(c, '\u{FE0E}' | '\u{FE0F}' | '\u{200D}'))
        && visible.chars().any(is_color_emoji_char)
    {
        return true;
    }
    let mut chars = visible.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if chars.next().is_some() || first.is_ascii_digit() {
        return false;
    }
    let has_explicit_name = node
        .attr("aria-label")
        .is_some_and(|v| !v.trim().is_empty())
        || node
            .attr("aria-labelledby")
            .is_some_and(|v| !v.trim().is_empty());
    if !has_explicit_name {
        return false;
    }
    let acc_name = crate::accname::accessible_name(doc, node.id);
    let acc_trimmed = acc_name.trim();
    let mut single_buf = [0u8; 4];
    let single_str = first.encode_utf8(&mut single_buf);
    !acc_trimmed.is_empty() && !acc_trimmed.eq_ignore_ascii_case(single_str)
}

/// Returns the nearest CSS gradient color stops behind `id` (with element
/// opacity applied) if not covered by a nearer opaque solid `background-color`
/// or non-gradient `background-image`.
pub(crate) fn effective_gradient_stops(doc: &Document, id: NodeId) -> Option<Vec<Rgba>> {
    for nid in std::iter::once(id).chain(doc.ancestors(id)) {
        let Some(computed) = doc.node(nid).and_then(|n| n.computed.as_ref()) else {
            continue;
        };
        if computed.has_background_image {
            return None;
        }
        let opacity = computed.opacity.unwrap_or(1.0).clamp(0.0, 1.0);
        if !computed.background_gradient_stops.is_empty() {
            let stops = computed
                .background_gradient_stops
                .iter()
                .map(|&s| Rgba::new(s.r, s.g, s.b, (s.a * opacity).clamp(0.0, 1.0)))
                .collect();
            return Some(stops);
        }
        if computed
            .background_color
            .is_some_and(|bg| bg.a * opacity >= 0.99)
        {
            return None;
        }
    }
    None
}

/// Returns `true` if `id` or an ancestor has a CSS `background-image` that is
/// not completely covered by a nearer opaque solid `background-color` or opaque gradient.
pub(crate) fn has_effective_background_image(doc: &Document, id: NodeId) -> bool {
    for nid in std::iter::once(id).chain(doc.ancestors(id)) {
        let Some(computed) = doc.node(nid).and_then(|n| n.computed.as_ref()) else {
            continue;
        };
        if computed.has_background_image {
            return true;
        }
        let opacity = computed.opacity.unwrap_or(1.0).clamp(0.0, 1.0);
        if !computed.background_gradient_stops.is_empty()
            && computed
                .background_gradient_stops
                .iter()
                .all(|s| s.a * opacity >= 0.99)
        {
            return false;
        }
        if computed
            .background_color
            .is_some_and(|bg| bg.a * opacity >= 0.99)
        {
            return false;
        }
    }
    false
}

/// Returns `true` if `id` or any ancestor is visually hidden from sighted users
/// via `opacity: 0`, CSS `clip`/`clip-path` collapsing, classic `.sr-only`
/// (`<= 1.5x1.5px` with `overflow: hidden|clip`), or being positioned entirely
/// outside a non-scrollable `overflow: hidden|clip` ancestor's bounding box.
pub(crate) fn is_visually_hidden(doc: &Document, id: NodeId) -> bool {
    let is_hidden_overflow = |v: Option<&str>| {
        v.is_some_and(|s| {
            let t = s.trim();
            t.eq_ignore_ascii_case("hidden") || t.eq_ignore_ascii_case("clip")
        })
    };
    let el_rect = doc
        .node(id)
        .and_then(|n| n.rect)
        .filter(|r| r.width > 0.0 && r.height > 0.0);
    for nid in std::iter::once(id).chain(doc.ancestors(id)) {
        let Some(node) = doc.node(nid) else {
            continue;
        };
        let Some(computed) = node.computed.as_ref() else {
            continue;
        };
        if computed.opacity.is_some_and(|op| op <= 0.0) {
            return true;
        }
        if computed.is_clipped {
            return true;
        }
        if let Some(rect) = node.rect {
            let clip_x = is_hidden_overflow(computed.overflow_x.as_deref());
            let clip_y = is_hidden_overflow(computed.overflow_y.as_deref());
            if (clip_x || clip_y)
                && ((rect.width <= 1.5 && rect.height <= 1.5)
                    || rect.width <= 0.0
                    || rect.height <= 0.0)
            {
                return true;
            }
            if nid != id
                && let Some(er) = el_rect
                && rect.width > 0.0
                && rect.height > 0.0
            {
                let clipped_x = clip_x
                    && (er.x + er.width <= rect.x + 0.5 || er.x >= rect.x + rect.width - 0.5);
                let clipped_y = clip_y
                    && (er.y + er.height <= rect.y + 0.5 || er.y >= rect.y + rect.height - 0.5);
                if clipped_x || clipped_y {
                    return true;
                }
            }
        }
    }
    false
}

#[allow(clippy::too_many_lines)]
fn evaluate_contrast(
    meta: &RuleMeta,
    doc: &Document,
    level_label: &str,
    threshold_fn: fn(Option<f32>, Option<u16>) -> f64,
    out: &mut Vec<Violation>,
) {
    for el in doc.elements() {
        if el.tag == "option"
            || !doc.is_in_accessibility_tree(el.id)
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
        if computed.color.is_none_or(|c| c.a <= 0.0) {
            continue;
        }
        let bg = effective_background(doc, el.id);
        let threshold = threshold_fn(computed.font_size_px, computed.font_weight);

        if let Some(stops) = effective_gradient_stops(doc, el.id) {
            let mut all_pass = true;
            let mut all_fail = true;
            let mut min_ratio = f64::INFINITY;
            let mut max_ratio = 0.0_f64;
            let mut evaluated = 0usize;

            for stop in stops {
                let backdrop = composite_over(stop, bg);
                let Some(foreground) = effective_foreground(doc, el.id, backdrop) else {
                    continue;
                };
                let ratio = contrast_ratio(foreground, backdrop);
                evaluated += 1;
                min_ratio = min_ratio.min(ratio);
                max_ratio = max_ratio.max(ratio);
                if ratio + 0.005 < threshold {
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
                            meta,
                            doc,
                            el.id,
                            format!(
                                "Contrast across all CSS gradient stops ({min_ratio:.2}:1-{max_ratio:.2}:1) is below {level_label} minimum {threshold:.1}:1; requires visual review."
                            ),
                        )
                        .into_incomplete()
                    } else {
                        Violation::with_detail(
                            meta,
                            doc,
                            el.id,
                            format!(
                                "Contrast across all CSS gradient stops ({min_ratio:.2}:1-{max_ratio:.2}:1) is below {level_label} minimum {threshold:.1}:1."
                            ),
                        )
                    };
                    out.push(v);
                } else {
                    out.push(
                        Violation::with_detail(
                            meta,
                            doc,
                            el.id,
                            format!(
                                "Contrast across CSS gradient stops ranges from {min_ratio:.2}:1 to {max_ratio:.2}:1 ({level_label} minimum {threshold:.1}:1); requires visual review."
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
        let ratio = contrast_ratio(fg, bg);
        if ratio + 0.005 < threshold {
            let bg_image = has_effective_background_image(doc, el.id);
            let equal_ratio = ratio <= 1.005;
            let v = if computed.has_text_shadow || bg_image || equal_ratio {
                let reason = match (computed.has_text_shadow, bg_image, equal_ratio) {
                    (true, true, _) => "text-shadow and background-image present",
                    (true, false, _) => "text-shadow present",
                    (false, true, _) => "background-image present",
                    (false, false, true) => "foreground and background colors are equal (1:1)",
                    (false, false, false) => unreachable!(),
                };
                Violation::with_detail(
                    meta,
                    doc,
                    el.id,
                    format!(
                        "Contrast ratio {ratio:.2}:1 is below {level_label} minimum {threshold:.1}:1 (fg rgb({},{},{}), bg rgb({},{},{})); {reason}, requires visual review.",
                        fg.r, fg.g, fg.b, bg.r, bg.g, bg.b
                    ),
                )
                .into_incomplete()
            } else {
                Violation::with_detail(
                    meta,
                    doc,
                    el.id,
                    format!(
                        "Contrast ratio {ratio:.2}:1 is below {level_label} minimum {threshold:.1}:1 (fg rgb({},{},{}), bg rgb({},{},{})).",
                        fg.r, fg.g, fg.b, bg.r, bg.g, bg.b
                    ),
                )
            };
            out.push(v);
        }
    }
}

impl Rule for ColorContrast {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        evaluate_contrast(&self.meta(), doc, "WCAG AA", aa_threshold, out);
    }
}

impl ColorContrastEnhanced {
    pub const META: RuleMeta = RuleMeta {
        id: "color-contrast-enhanced",
        impact: Impact::Serious,
        wcag: "1.4.6 (AAA)",
        tags: &["cat.color", "wcag2aaa", "wcag146", "ACT"],
        enabled_by_default: false,
        needs_review: false,
        help: "Elements must meet enhanced color contrast ratio thresholds",
        description: "Text must have at least 7.0:1 contrast against its background (4.5:1 for large text >= 24px or >= 18.66px bold).",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/contrast-enhanced.html",
    };
}

impl Rule for ColorContrastEnhanced {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        evaluate_contrast(&self.meta(), doc, "WCAG AAA", aaa_threshold, out);
    }
}

impl CssOrientationLock {
    pub const META: RuleMeta = RuleMeta {
        id: "css-orientation-lock",
        impact: Impact::Serious,
        wcag: "1.3.4 (AA)",
        tags: &[
            "cat.structure",
            "wcag134",
            "wcag21aa",
            "EN-301-549",
            "EN-9.1.3.4",
            "RGAAv4",
            "RGAA-13.9.1",
            "experimental",
        ],
        enabled_by_default: false,
        needs_review: false,
        help: "CSS Media queries must not lock display orientation",
        description: "Ensure content is not locked to a specific display orientation via CSS.",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/orientation.html",
    };
}

impl Rule for CssOrientationLock {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        if !doc.css_orientation_locked {
            return;
        }
        let target_id = doc
            .elements()
            .find(|n| n.tag == "html")
            .map_or(doc.root, |n| n.id);
        out.push(Violation::from_node(&self.meta(), doc, target_id));
    }
}

fn is_semantic_focus_role(role: &str) -> bool {
    matches!(
        role,
        "alertdialog"
            | "application"
            | "banner"
            | "button"
            | "checkbox"
            | "combobox"
            | "complementary"
            | "contentinfo"
            | "dialog"
            | "form"
            | "grid"
            | "gridcell"
            | "link"
            | "listbox"
            | "main"
            | "menu"
            | "menubar"
            | "menuitem"
            | "menuitemcheckbox"
            | "menuitemradio"
            | "navigation"
            | "option"
            | "progressbar"
            | "radio"
            | "radiogroup"
            | "region"
            | "scrollbar"
            | "search"
            | "searchbox"
            | "separator"
            | "slider"
            | "spinbutton"
            | "switch"
            | "tab"
            | "tablist"
            | "tabpanel"
            | "textbox"
            | "toolbar"
            | "tooltip"
            | "tree"
            | "treegrid"
            | "treeitem"
    )
}

impl FocusOrderSemantics {
    pub const META: RuleMeta = RuleMeta {
        id: "focus-order-semantics",
        impact: Impact::Minor,
        wcag: "2.4.3 (Best Practice)",
        tags: &[
            "cat.keyboard",
            "best-practice",
            "RGAAv4",
            "RGAA-12.8.1",
            "experimental",
        ],
        enabled_by_default: false,
        needs_review: false,
        help: "Elements in the focus order should have an appropriate role",
        description: "Ensure elements made focusable with tabindex have an interactive or semantic ARIA role.",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/focus-order.html",
    };
}

impl Rule for FocusOrderSemantics {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) || is_natively_focusable(el) {
                continue;
            }
            let added_to_tab_order = el
                .attr("tabindex")
                .and_then(|v| v.trim().parse::<i32>().ok())
                .is_some_and(|v| v >= 0);
            if !added_to_tab_order {
                continue;
            }
            let has_appropriate_role = effective_role(doc, el).is_some_and(is_semantic_focus_role);
            if !has_appropriate_role {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!(
                        "Focusable <{}> with tabindex lacks an interactive or semantic ARIA role.",
                        el.tag
                    ),
                ));
            }
        }
    }
}

impl FrameFocusableContent {
    pub const META: RuleMeta = RuleMeta {
        id: "frame-focusable-content",
        impact: Impact::Serious,
        wcag: "2.1.1 (A)",
        tags: &[
            "cat.keyboard",
            "wcag2a",
            "wcag211",
            "TTv5",
            "TT4.a",
            "EN-301-549",
            "EN-9.2.1.1",
            "RGAAv4",
            "RGAA-7.3.2",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Frames with focusable content must not have tabindex=-1",
        description: "Ensure <frame> and <iframe> elements with focusable content do not have tabindex=\"-1\".",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/keyboard.html",
    };
}

impl Rule for FrameFocusableContent {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let open_dialogs: Vec<NodeId> = doc
            .elements()
            .filter(|n| n.tag == "dialog" && n.has_attr("open") && doc.is_rendered(n.id))
            .map(|n| n.id)
            .collect();

        for el in doc.elements() {
            if !matches!(el.tag.as_str(), "iframe" | "frame")
                || !doc.is_in_accessibility_tree(el.id)
            {
                continue;
            }
            let is_tiny_frame = el.rect.is_some_and(|r| r.width <= 1.0 && r.height <= 1.0)
                || el.computed.as_ref().is_some_and(|c| {
                    c.client_width.is_some_and(|w| w <= 1.0)
                        && c.client_height.is_some_and(|h| h <= 1.0)
                });
            if is_tiny_frame {
                continue;
            }
            let ancestors = doc.ancestors(el.id);
            let is_inert = el.has_attr("inert")
                || ancestors
                    .iter()
                    .any(|&anc_id| doc.node(anc_id).is_some_and(|anc| anc.has_attr("inert")));
            if is_inert {
                continue;
            }
            let behind_open_dialog = open_dialogs
                .iter()
                .any(|&dlg_id| dlg_id != el.id && !ancestors.contains(&dlg_id));
            if behind_open_dialog {
                continue;
            }
            let has_negative_tabindex = el
                .attr("tabindex")
                .and_then(|v| v.trim().parse::<i32>().ok())
                .is_some_and(|t| t < 0);
            if has_negative_tabindex && el.frame_has_focusable == Some(true) {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl FrameTested {
    pub const META: RuleMeta = RuleMeta {
        id: "frame-tested",
        impact: Impact::Critical,
        wcag: "Best Practice",
        tags: &["cat.structure", "best-practice", "review-item"],
        enabled_by_default: true,
        needs_review: true,
        help: "Frames should be tested for accessibility",
        description: "Ensure <iframe> and <frame> elements have been tested for accessibility.",
        docs_url: "https://www.w3.org/WAI/standards-guidelines/act/rules/",
    };
}

impl Rule for FrameTested {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !matches!(el.tag.as_str(), "iframe" | "frame")
                || !doc.is_in_accessibility_tree(el.id)
            {
                continue;
            }
            if el.frame_tested == Some(false) {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

fn is_computed_hidden(computed: &ComputedStyle) -> bool {
    computed
        .display
        .as_deref()
        .is_some_and(|d| d.trim().eq_ignore_ascii_case("none"))
        || computed.visibility.as_deref().is_some_and(|v| {
            let trimmed = v.trim();
            trimmed.eq_ignore_ascii_case("hidden") || trimmed.eq_ignore_ascii_case("collapse")
        })
}

impl HiddenContent {
    pub const META: RuleMeta = RuleMeta {
        id: "hidden-content",
        impact: Impact::Minor,
        wcag: "Best Practice",
        tags: &[
            "cat.structure",
            "best-practice",
            "experimental",
            "review-item",
        ],
        enabled_by_default: false,
        needs_review: false,
        help: "Hidden content on the page should be analyzed",
        description: "Inform users about hidden elements with text content that could not be analyzed while hidden.",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/info-and-relationships.html",
    };
}

impl Rule for HiddenContent {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if matches!(
                el.tag.as_str(),
                "head" | "script" | "style" | "template" | "noscript" | "meta" | "link" | "title"
            ) {
                continue;
            }
            let Some(computed) = &el.computed else {
                continue;
            };
            if !is_computed_hidden(computed) {
                continue;
            }
            let ancestor_hidden = doc.ancestors(el.id).into_iter().any(|anc_id| {
                doc.node(anc_id)
                    .and_then(|n| n.computed.as_ref())
                    .is_some_and(is_computed_hidden)
            });
            if ancestor_hidden {
                continue;
            }
            if !collect_raw_text(doc, el.id).trim().is_empty() {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl LinkInTextBlock {
    pub const META: RuleMeta = RuleMeta {
        id: "link-in-text-block",
        impact: Impact::Serious,
        wcag: "1.4.1 (A)",
        tags: &[
            "cat.color",
            "wcag2a",
            "wcag141",
            "TTv5",
            "TT13.a",
            "EN-301-549",
            "EN-9.1.4.1",
            "RGAAv4",
            "RGAA-10.6.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Links must be distinguishable without relying on color",
        description: "Ensure links within text blocks are underlined or have at least 3.0:1 contrast against surrounding text.",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html",
    };
}

fn block_has_non_link_text(doc: &Document, node_id: NodeId, excluded_id: NodeId) -> bool {
    let mut visited = HashSet::new();
    visited.insert(node_id);
    let mut stack: Vec<NodeId> = doc.node(node_id).map_or_default(|n| n.children.clone());
    while let Some(cid) = stack.pop() {
        if cid == excluded_id || !visited.insert(cid) {
            continue;
        }
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if child.is_text() && child.text.chars().any(char::is_alphanumeric) {
            return true;
        }
        if child.is_element() && doc.is_in_accessibility_tree(cid) {
            let is_link = (child.tag == "a" && child.has_attr("href"))
                || effective_role(doc, child) == Some("link");
            if is_link {
                continue;
            }
            stack.extend(&child.children);
        }
    }
    false
}

fn is_block_display(display: Option<&str>) -> bool {
    display.is_some_and(|d| {
        let trimmed = d.trim();
        !trimmed.is_empty()
            && !trimmed.starts_with("inline")
            && !trimmed.eq_ignore_ascii_case("contents")
    })
}

fn is_block_container_element(node: &Node) -> bool {
    if let Some(display) = node.computed.as_ref().and_then(|c| c.display.as_deref()) {
        return is_block_display(Some(display));
    }
    matches!(
        node.tag.as_str(),
        "p" | "li"
            | "td"
            | "th"
            | "blockquote"
            | "div"
            | "section"
            | "article"
            | "header"
            | "footer"
            | "nav"
            | "aside"
            | "main"
            | "figcaption"
            | "figure"
            | "dd"
            | "dt"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
    )
}

fn is_italic_style(style: Option<&str>) -> bool {
    style.is_some_and(|s| {
        let trimmed = s.trim();
        trimmed.eq_ignore_ascii_case("italic") || trimmed.eq_ignore_ascii_case("oblique")
    })
}

fn is_separator_punct(c: char) -> bool {
    matches!(
        c,
        '|' | '•' | '·' | '/' | '-' | '—' | '–' | '©' | ',' | '.' | ':' | ';'
    )
}

fn is_segment_divider_char(c: char) -> bool {
    matches!(c, '|' | '•' | '·')
}

#[derive(Default)]
struct SegmentWalkState {
    in_target_segment: bool,
    found_target: bool,
    segment_non_link_text: String,
    segment_link_chars: usize,
}

fn walk_prose_segment(
    doc: &Document,
    nid: NodeId,
    target_link_id: NodeId,
    is_root: bool,
    visited: &mut HashSet<NodeId>,
    state: &mut SegmentWalkState,
) {
    if !visited.insert(nid) {
        return;
    }
    let Some(node) = doc.node(nid) else {
        return;
    };
    if node.is_text() {
        for part in node.text.split_inclusive(is_segment_divider_char) {
            let has_divider = part.chars().last().is_some_and(is_segment_divider_char);
            let clean = if has_divider {
                part.trim_end_matches(is_segment_divider_char)
            } else {
                part
            };
            state.segment_non_link_text.push_str(clean);
            if has_divider {
                if state.in_target_segment {
                    state.found_target = true;
                    return;
                }
                state.segment_non_link_text.clear();
                state.segment_link_chars = 0;
            }
        }
        return;
    }
    if !node.is_element() {
        return;
    }
    if !is_root {
        if nid == target_link_id {
            state.in_target_segment = true;
            state.segment_link_chars += normalize_whitespace(&collect_raw_text(doc, nid))
                .chars()
                .count();
            return;
        }
        if !doc.is_in_accessibility_tree(nid) {
            return;
        }
        if node.tag == "br" || is_block_container_element(node) {
            if state.in_target_segment {
                state.found_target = true;
                return;
            }
            state.segment_non_link_text.clear();
            state.segment_link_chars = 0;
            return;
        }
        let is_other_link_or_widget = (node.tag == "a" && node.has_attr("href"))
            || effective_role(doc, node) == Some("link")
            || is_interactive_control(doc, node);
        if is_other_link_or_widget {
            state.segment_link_chars += normalize_whitespace(&collect_raw_text(doc, nid))
                .chars()
                .count();
            return;
        }
    }
    for &cid in &node.children {
        walk_prose_segment(doc, cid, target_link_id, false, visited, state);
        if state.found_target {
            return;
        }
    }
}

fn link_is_in_prose_text_block(doc: &Document, block_id: NodeId, link_id: NodeId) -> bool {
    let mut visited = HashSet::new();
    let mut state = SegmentWalkState::default();
    walk_prose_segment(doc, block_id, link_id, true, &mut visited, &mut state);
    if !state.in_target_segment {
        return false;
    }
    let non_link_len = normalize_whitespace(&state.segment_non_link_text)
        .chars()
        .count();
    if non_link_len <= state.segment_link_chars {
        return false;
    }
    let stripped: String = state
        .segment_non_link_text
        .chars()
        .map(|c| if is_separator_punct(c) { ' ' } else { c })
        .collect();
    let mut words = 0usize;
    let mut alnum_chars = 0usize;
    for word in stripped.split_whitespace() {
        let count = word.chars().filter(|c| c.is_alphanumeric()).count();
        if count > 0 {
            words += 1;
            alnum_chars += count;
        }
    }
    words >= 2 || alnum_chars >= 6
}

fn computed_has_link_style_cue(
    link_computed: &ComputedStyle,
    parent_computed: &ComputedStyle,
) -> bool {
    if link_computed
        .text_decoration_line
        .as_deref()
        .is_some_and(|d| {
            let lower = d.to_ascii_lowercase();
            lower.contains("underline")
                || lower.contains("overline")
                || lower.contains("line-through")
        })
        || link_computed.has_border_bottom
        || link_computed.has_box_shadow
    {
        return true;
    }
    let pw = parent_computed.font_weight.unwrap_or(400);
    if let Some(lw) = link_computed.font_weight
        && ((lw >= 600 && lw > pw) || (pw >= 700 && lw < 600))
    {
        return true;
    }
    if is_italic_style(link_computed.font_style.as_deref())
        != is_italic_style(parent_computed.font_style.as_deref())
    {
        return true;
    }
    if link_computed.background_color.is_some_and(|bg| bg.a > 0.0)
        && link_computed.background_color != parent_computed.background_color
    {
        return true;
    }
    false
}

fn link_has_visual_style_cue(
    doc: &Document,
    el: &Node,
    link_computed: &ComputedStyle,
    parent_computed: &ComputedStyle,
) -> bool {
    if computed_has_link_style_cue(link_computed, parent_computed) {
        return true;
    }
    if !doc.has_direct_text(el.id) {
        let child_elements: Vec<&Node> = el
            .children
            .iter()
            .filter_map(|&cid| doc.node(cid))
            .filter(|n| n.is_element() && doc.is_in_accessibility_tree(n.id))
            .collect();
        if !child_elements.is_empty()
            && child_elements.iter().all(|child| {
                child
                    .computed
                    .as_ref()
                    .is_some_and(|cc| computed_has_link_style_cue(cc, parent_computed))
            })
        {
            return true;
        }
    }
    false
}

impl Rule for LinkInTextBlock {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "a"
                || !el.has_attr("href")
                || !doc.is_in_accessibility_tree(el.id)
                || is_visually_hidden(doc, el.id)
            {
                continue;
            }
            let Some(link_computed) = &el.computed else {
                continue;
            };
            if link_computed.color.is_none() || is_block_display(link_computed.display.as_deref()) {
                continue;
            }
            let Some(text_block_id) = doc
                .ancestors(el.id)
                .into_iter()
                .find(|&anc_id| doc.node(anc_id).is_some_and(is_block_container_element))
            else {
                continue;
            };
            if !doc.node(text_block_id).is_some_and(|anc| {
                matches!(
                    anc.tag.as_str(),
                    "p" | "li" | "td" | "th" | "blockquote" | "figcaption" | "dd" | "div"
                )
            }) {
                continue;
            }
            if !link_is_in_prose_text_block(doc, text_block_id, el.id) {
                continue;
            }
            let Some(parent_computed) = doc.node(text_block_id).and_then(|n| n.computed.as_ref())
            else {
                continue;
            };
            if parent_computed.color.is_none() {
                continue;
            }
            if link_has_visual_style_cue(doc, el, link_computed, parent_computed) {
                continue;
            }
            let bg = effective_background(doc, el.id);
            let Some(link_fg) = effective_foreground(doc, el.id, bg) else {
                continue;
            };
            let Some(parent_fg) = effective_foreground(doc, text_block_id, bg) else {
                continue;
            };
            if link_fg == parent_fg {
                continue;
            }
            let ratio = contrast_ratio(link_fg, parent_fg);
            if ratio + 0.005 < LINK_IN_TEXT_MIN_CONTRAST {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!(
                        "Non-underlined link has {ratio:.2}:1 contrast against surrounding text (minimum is 3.0:1)."
                    ),
                ));
            }
        }
    }
}

impl NoAutoplayAudio {
    pub const META: RuleMeta = RuleMeta {
        id: "no-autoplay-audio",
        impact: Impact::Moderate,
        wcag: "1.4.2 (A)",
        tags: &[
            "cat.time-and-media",
            "wcag2a",
            "wcag142",
            "TTv5",
            "TT2.a",
            "EN-301-549",
            "EN-9.1.4.2",
            "ACT",
            "RGAAv4",
            "RGAA-4.10.1",
        ],
        enabled_by_default: true,
        needs_review: true,
        help: "<video> or <audio> elements must not play automatically",
        description: "Ensure <video> or <audio> elements do not autoplay audio for more than 3 seconds without a mechanism to pause or mute.",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/audio-control.html",
    };
}

fn parse_npt_timestamp(raw: &str) -> Option<f32> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parts: Vec<&str> = trimmed.split(':').collect();
    let secs = match parts.as_slice() {
        [s] => s.parse::<f32>().ok()?,
        [m, s] => {
            let mins = m.parse::<f32>().ok()?;
            let secs = s.parse::<f32>().ok()?;
            if !mins.is_finite() || mins < 0.0 || !secs.is_finite() || secs < 0.0 {
                return None;
            }
            mins * 60.0 + secs
        }
        [h, m, s] => {
            let hours = h.parse::<f32>().ok()?;
            let mins = m.parse::<f32>().ok()?;
            let secs = s.parse::<f32>().ok()?;
            if !hours.is_finite()
                || hours < 0.0
                || !mins.is_finite()
                || mins < 0.0
                || !secs.is_finite()
                || secs < 0.0
            {
                return None;
            }
            hours * 3600.0 + mins * 60.0 + secs
        }
        _ => return None,
    };
    if secs.is_finite() && secs >= 0.0 {
        Some(secs)
    } else {
        None
    }
}

fn parse_media_fragment_range(url: &str) -> Option<(f32, Option<f32>)> {
    let (_, fragment) = url.split_once('#')?;
    for param in fragment.split('&') {
        let Some(raw_val) = param.strip_prefix("t=") else {
            continue;
        };
        let val = raw_val.strip_prefix("npt:").unwrap_or(raw_val);
        if let Some((start_s, end_s)) = val.split_once(',') {
            let start = if start_s.trim().is_empty() {
                0.0
            } else {
                parse_npt_timestamp(start_s)?
            };
            let end = if end_s.trim().is_empty() {
                None
            } else {
                Some(parse_npt_timestamp(end_s)?)
            };
            if let Some(end_val) = end
                && end_val < start
            {
                continue;
            }
            return Some((start, end));
        }
        let start = parse_npt_timestamp(val)?;
        return Some((start, None));
    }
    None
}

fn effective_media_duration(doc: &Document, el: &Node) -> Option<f32> {
    let fragment_range = el
        .attr("src")
        .and_then(parse_media_fragment_range)
        .or_else(|| {
            el.children.iter().find_map(|&cid| {
                let child = doc.node(cid)?;
                if child.tag != "source" {
                    return None;
                }
                child.attr("src").and_then(parse_media_fragment_range)
            })
        });

    if let Some((start, end_opt)) = fragment_range {
        return match (end_opt, el.media_duration_secs) {
            (Some(end), Some(total)) => Some((end.min(total) - start).max(0.0)),
            (Some(end), None) => Some((end - start).max(0.0)),
            (None, Some(total)) => Some((total - start).max(0.0)),
            (None, None) => None,
        };
    }

    el.media_duration_secs
}

impl Rule for NoAutoplayAudio {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !matches!(el.tag.as_str(), "audio" | "video") || !doc.is_in_accessibility_tree(el.id)
            {
                continue;
            }
            if !el.has_attr("autoplay") || el.has_attr("muted") || el.has_attr("controls") {
                continue;
            }
            let exceeds_limit =
                effective_media_duration(doc, el).is_none_or(|secs| secs > MAX_AUTOPLAY_SECS);
            if exceeds_limit {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl PAsHeading {
    pub const META: RuleMeta = RuleMeta {
        id: "p-as-heading",
        impact: Impact::Serious,
        wcag: "1.3.1 (A)",
        tags: &[
            "cat.semantics",
            "wcag2a",
            "wcag131",
            "EN-301-549",
            "EN-9.1.3.1",
            "RGAAv4",
            "RGAA-9.1.3",
            "experimental",
        ],
        enabled_by_default: false,
        needs_review: false,
        help: "Styled <p> elements must not be used as headings",
        description: "Use semantic heading elements (<h1>-<h6>) instead of styling short <p> elements larger or bolder than body paragraphs.",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/info-and-relationships.html",
    };
}

impl Rule for PAsHeading {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "p" || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let Some(computed) = &el.computed else {
                continue;
            };
            let text = normalize_whitespace(&collect_raw_text(doc, el.id));
            if text.is_empty() || text.len() >= 80 || text.ends_with('.') {
                continue;
            }
            let Some(next_p) = next_sibling_paragraph(doc, el) else {
                continue;
            };
            let Some(sib_computed) = &next_p.computed else {
                continue;
            };

            let el_size = computed.font_size_px.unwrap_or(16.0);
            let sib_size = sib_computed.font_size_px.unwrap_or(16.0);
            let el_weight = computed.font_weight.unwrap_or(400);
            let sib_weight = sib_computed.font_weight.unwrap_or(400);

            let is_larger = el_size >= sib_size * 1.25;
            let is_bolder = el_weight >= 700 && sib_weight < 700;
            if is_larger || is_bolder {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

fn next_sibling_paragraph<'a>(doc: &'a Document, el: &Node) -> Option<&'a Node> {
    let parent = doc.node(el.parent?)?;
    let pos = parent.children.iter().position(|&cid| cid == el.id)?;
    parent.children[pos + 1..].iter().find_map(|&cid| {
        let sib = doc.node(cid)?;
        if sib.tag == "p" && doc.is_in_accessibility_tree(cid) {
            Some(sib)
        } else {
            None
        }
    })
}

fn is_scroll_overflow(val: Option<&str>) -> bool {
    val.is_some_and(|v| {
        let trimmed = v.trim();
        trimmed.eq_ignore_ascii_case("auto") || trimmed.eq_ignore_ascii_case("scroll")
    })
}

fn has_focusable_descendant(doc: &Document, id: NodeId) -> bool {
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
        if !child.is_element() || !doc.is_in_accessibility_tree(cid) {
            continue;
        }
        if is_sequentially_focusable(child) {
            return true;
        }
        stack.extend(&child.children);
    }
    false
}

impl ScrollableRegionFocusable {
    pub const META: RuleMeta = RuleMeta {
        id: "scrollable-region-focusable",
        impact: Impact::Serious,
        wcag: "2.1.1 (A)",
        tags: &[
            "cat.keyboard",
            "wcag2a",
            "wcag211",
            "wcag213",
            "TTv5",
            "TT4.a",
            "EN-301-549",
            "EN-9.2.1.1",
            "EN-9.2.1.3",
            "RGAAv4",
            "RGAA-7.3.2",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Scrollable region must have keyboard access",
        description: "Ensure scrollable containers with overflowing content are keyboard-focusable or contain a focusable descendant.",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/keyboard.html",
    };
}

fn has_meaningful_scrollable_content(doc: &Document, id: NodeId) -> bool {
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
        if child.is_element() && doc.is_rendered(cid) {
            if matches!(
                child.tag.as_str(),
                "img"
                    | "svg"
                    | "canvas"
                    | "video"
                    | "audio"
                    | "picture"
                    | "object"
                    | "embed"
                    | "iframe"
                    | "table"
                    | "hr"
            ) {
                return true;
            }
            stack.extend(&child.children);
        }
    }
    false
}

impl Rule for ScrollableRegionFocusable {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if matches!(el.tag.as_str(), "html" | "body")
                || !doc.is_in_accessibility_tree(el.id)
                || is_visually_hidden(doc, el.id)
                || el.rect.is_some_and(|r| r.width <= 0.0 || r.height <= 0.0)
            {
                continue;
            }
            let Some(computed) = &el.computed else {
                continue;
            };
            let overflows_y = is_scroll_overflow(computed.overflow_y.as_deref())
                && computed
                    .scroll_height
                    .zip(computed.client_height)
                    .is_some_and(|(s, c)| s > c + 1.0);
            let overflows_x = is_scroll_overflow(computed.overflow_x.as_deref())
                && computed
                    .scroll_width
                    .zip(computed.client_width)
                    .is_some_and(|(s, c)| s > c + 1.0);
            if !overflows_y && !overflows_x {
                continue;
            }
            if !has_meaningful_scrollable_content(doc, el.id) {
                continue;
            }
            if !is_sequentially_focusable(el) && !has_focusable_descendant(doc, el.id) {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

/// WCAG 2.5.8 inline exception: interactive target inline within a sentence or block of text.
fn is_inline_in_text_block(doc: &Document, el: &Node) -> bool {
    let is_inline = match el.computed.as_ref().and_then(|c| c.display.as_deref()) {
        Some(d) => d.trim().eq_ignore_ascii_case("inline"),
        None => el.tag == "a",
    };
    if !is_inline {
        return false;
    }
    let Some(text_block_id) = doc.ancestors(el.id).into_iter().find(|&anc_id| {
        doc.node(anc_id).is_some_and(|anc| {
            matches!(
                anc.tag.as_str(),
                "p" | "li" | "td" | "blockquote" | "dd" | "figcaption"
            )
        })
    }) else {
        return false;
    };
    block_has_non_link_text(doc, text_block_id, el.id)
}

/// Returns `true` if `el` (or an ancestor) is visually hidden / collapsed
/// (zero-sized, `opacity: 0`, clipped via `clip`/`clip-path`, or `<= 1.5x1.5px`
/// with `overflow: hidden|clip`, e.g. `.sr-only` skip links that expand on focus).
fn is_visually_hidden_target(doc: &Document, el: &Node, rect: Rect) -> bool {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return true;
    }
    is_visually_hidden(doc, el.id)
}

fn is_undersized_rect(rect: Rect) -> bool {
    rect.width + 0.01 < MIN_TARGET_SIZE_PX || rect.height + 0.01 < MIN_TARGET_SIZE_PX
}

fn circle_intersects_rect(cx: f64, cy: f64, radius: f64, rect: Rect) -> bool {
    let min_x = f64::from(rect.x);
    let max_x = f64::from(rect.x + rect.width);
    let min_y = f64::from(rect.y);
    let max_y = f64::from(rect.y + rect.height);
    let px = cx.clamp(min_x, max_x);
    let py = cy.clamp(min_y, max_y);
    let dx = cx - px;
    let dy = cy - py;
    let effective_r = (radius - 0.01).max(0.0);
    dx * dx + dy * dy < effective_r * effective_r
}

fn circles_intersect(ax: f64, ay: f64, bx: f64, by: f64, diameter: f64) -> bool {
    let dx = ax - bx;
    let dy = ay - by;
    let effective_d = (diameter - 0.01).max(0.0);
    dx * dx + dy * dy < effective_d * effective_d
}

impl TargetSize {
    pub const META: RuleMeta = RuleMeta {
        id: "target-size",
        impact: Impact::Serious,
        wcag: "2.5.8 (AA)",
        tags: &[
            "cat.sensory-and-visual-cues",
            "wcag22aa",
            "wcag258",
            "EN-301-549v4",
            "EN-9.2.5.8",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Interactive targets must be at least 24 by 24 CSS pixels",
        description: "Ensure interactive controls have a bounding box of at least 24x24 CSS pixels or satisfy the WCAG 2.5.8 24px spacing exception.",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html",
    };
}

const MAX_CELLS_PER_TARGET: i64 = 64;
const MAX_TARGET_COORD: f64 = 1.0e7;

type TargetGrid = HashMap<(i32, i32), Vec<usize>>;

#[allow(clippy::cast_possible_truncation)]
fn build_target_spatial_grid(
    targets: &[(NodeId, Rect, bool, bool)],
    cell_size: f64,
) -> (TargetGrid, Vec<usize>) {
    let mut grid: TargetGrid = HashMap::with_capacity(targets.len());
    let mut large_targets: Vec<usize> = Vec::new();

    for (idx, &(_, rect, _, _)) in targets.iter().enumerate() {
        let min_x = f64::from(rect.x);
        let max_x = f64::from(rect.x + rect.width);
        let min_y = f64::from(rect.y);
        let max_y = f64::from(rect.y + rect.height);
        if !min_x.is_finite()
            || !max_x.is_finite()
            || !min_y.is_finite()
            || !max_y.is_finite()
            || min_x.abs() > MAX_TARGET_COORD
            || max_x.abs() > MAX_TARGET_COORD
            || min_y.abs() > MAX_TARGET_COORD
            || max_y.abs() > MAX_TARGET_COORD
        {
            large_targets.push(idx);
            continue;
        }
        let cmin_x = (min_x / cell_size).floor() as i32;
        let cmax_x = (max_x / cell_size).floor() as i32;
        let cmin_y = (min_y / cell_size).floor() as i32;
        let cmax_y = (max_y / cell_size).floor() as i32;
        let span_x = i64::from(cmax_x) - i64::from(cmin_x) + 1;
        let span_y = i64::from(cmax_y) - i64::from(cmin_y) + 1;
        if span_x <= 0 || span_y <= 0 || span_x.saturating_mul(span_y) > MAX_CELLS_PER_TARGET {
            large_targets.push(idx);
            continue;
        }
        for gx in cmin_x..=cmax_x {
            for gy in cmin_y..=cmax_y {
                grid.entry((gx, gy)).or_default().push(idx);
            }
        }
    }

    (grid, large_targets)
}

impl Rule for TargetSize {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    #[allow(clippy::cast_possible_truncation)]
    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        // Collect all visible interactive targets once for the WCAG 2.5.8 spacing check.
        let targets: Vec<(NodeId, Rect, bool, bool)> = doc
            .elements()
            .filter_map(|el| {
                if !doc.is_in_accessibility_tree(el.id) || !is_interactive_control(doc, el) {
                    return None;
                }
                let rect = el.rect?;
                if is_visually_hidden_target(doc, el, rect) {
                    return None;
                }
                let undersized = is_undersized_rect(rect);
                let inline_exempt = is_inline_in_text_block(doc, el);
                Some((el.id, rect, undersized, inline_exempt))
            })
            .collect();

        let diameter = f64::from(MIN_TARGET_SIZE_PX);
        let radius = diameter / 2.0;
        let (grid, large_targets) = build_target_spatial_grid(&targets, diameter);

        let conflicts_with = |id: NodeId, cx: f64, cy: f64, other_idx: usize| -> bool {
            let Some(&(other_id, other_rect, other_undersized, _)) = targets.get(other_idx) else {
                return false;
            };
            if other_id == id {
                return false;
            }
            if circle_intersects_rect(cx, cy, radius, other_rect) {
                return true;
            }
            if other_undersized {
                let ox = f64::from(other_rect.x) + f64::from(other_rect.width) / 2.0;
                let oy = f64::from(other_rect.y) + f64::from(other_rect.height) / 2.0;
                if circles_intersect(cx, cy, ox, oy, diameter) {
                    return true;
                }
            }
            false
        };

        for &(id, rect, undersized, inline_exempt) in &targets {
            if !undersized || inline_exempt {
                continue;
            }
            let cx = f64::from(rect.x) + f64::from(rect.width) / 2.0;
            let cy = f64::from(rect.y) + f64::from(rect.height) / 2.0;

            // WCAG 2.5.8 Spacing Exception: passes if a 24px-diameter circle centered on
            // the target does not intersect any other visible target's bounding box or the
            // 24px-diameter circle of any other undersized target.
            let mut cramped = large_targets
                .iter()
                .any(|&other_idx| conflicts_with(id, cx, cy, other_idx));

            if !cramped {
                if !cx.is_finite()
                    || !cy.is_finite()
                    || cx.abs() > MAX_TARGET_COORD
                    || cy.abs() > MAX_TARGET_COORD
                {
                    cramped =
                        (0..targets.len()).any(|other_idx| conflicts_with(id, cx, cy, other_idx));
                } else {
                    let ax = (cx / diameter).floor() as i32;
                    let ay = (cy / diameter).floor() as i32;
                    'cells: for dx in -1..=1 {
                        for dy in -1..=1 {
                            if let Some(bucket) = grid.get(&(ax + dx, ay + dy)) {
                                for &other_idx in bucket {
                                    if conflicts_with(id, cx, cy, other_idx) {
                                        cramped = true;
                                        break 'cells;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if cramped {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    id,
                    format!(
                        "Interactive target is {:.1}x{:.1}px (minimum is 24x24px and does not meet 24px spacing exception).",
                        rect.width, rect.height
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::parse_html;

    fn run_link_in_text_block(doc: &Document) -> Vec<Violation> {
        let mut out = Vec::new();
        LinkInTextBlock.run(doc, &mut out);
        out
    }

    #[test]
    fn link_in_text_block_skips_identical_foreground_color() {
        let mut doc = parse_html(
            r#"<!doctype html><html lang="en"><head><title>T</title></head><body>
               <p id="p">Read our updated <a id="same" href="/privacy">Privacy Notices</a> for complete policy details.</p>
               </body></html>"#,
        );
        let white = Rgba::opaque(255, 255, 255);
        let dark_bg = Rgba::opaque(20, 20, 20);
        for node in &mut doc.nodes {
            match node.id_attr() {
                Some("p") => {
                    node.computed = Some(ComputedStyle {
                        color: Some(white),
                        background_color: Some(dark_bg),
                        display: Some("block".to_owned()),
                        ..ComputedStyle::default()
                    });
                }
                Some("same") => {
                    node.computed = Some(ComputedStyle {
                        color: Some(white),
                        display: Some("inline".to_owned()),
                        text_decoration_line: Some("none".to_owned()),
                        ..ComputedStyle::default()
                    });
                }
                _ => {}
            }
        }
        assert_eq!(run_link_in_text_block(&doc).len(), 0);
    }

    #[test]
    fn link_in_text_block_flags_distinct_hue_near_one_to_one_luminance() {
        let mut doc = parse_html(
            r#"<!doctype html><html lang="en"><head><title>T</title></head><body>
               <p id="p">Specialized care is available through the <a id="hue" href="/unit">Regional Care Unit</a> downtown.</p>
               </body></html>"#,
        );
        let parent_fg = Rgba::opaque(0x3b, 0x37, 0x33);
        let link_fg = Rgba::opaque(0x03, 0x3f, 0x33);
        let bg = Rgba::opaque(244, 241, 237);
        for node in &mut doc.nodes {
            match node.id_attr() {
                Some("p") => {
                    node.computed = Some(ComputedStyle {
                        color: Some(parent_fg),
                        background_color: Some(bg),
                        display: Some("block".to_owned()),
                        ..ComputedStyle::default()
                    });
                }
                Some("hue") => {
                    node.computed = Some(ComputedStyle {
                        color: Some(link_fg),
                        display: Some("inline".to_owned()),
                        text_decoration_line: Some("none".to_owned()),
                        ..ComputedStyle::default()
                    });
                }
                _ => {}
            }
        }
        let fails = run_link_in_text_block(&doc);
        assert_eq!(fails.len(), 1);
        assert_eq!(fails[0].selector, "#hue");
    }

    #[test]
    fn link_in_text_block_skips_footer_credit_where_link_text_exceeds_non_link_text() {
        let mut doc = parse_html(
            r#"<!doctype html><html lang="en"><head><title>T</title></head><body>
               <div id="credit"><span>Website Designed by <a id="a1" href="/a">Creative Design Group</a> | <a id="a2" href="/b">Smart Marketing</a></span></div>
               </body></html>"#,
        );
        let parent_fg = Rgba::opaque(51, 51, 51);
        let link_fg = Rgba::opaque(0, 51, 204);
        let bg = Rgba::opaque(255, 255, 255);
        for node in &mut doc.nodes {
            match node.id_attr() {
                Some("credit") => {
                    node.computed = Some(ComputedStyle {
                        color: Some(parent_fg),
                        background_color: Some(bg),
                        display: Some("block".to_owned()),
                        ..ComputedStyle::default()
                    });
                }
                Some("a1" | "a2") => {
                    node.computed = Some(ComputedStyle {
                        color: Some(link_fg),
                        display: Some("inline".to_owned()),
                        text_decoration_line: Some("none".to_owned()),
                        ..ComputedStyle::default()
                    });
                }
                _ => {}
            }
        }
        assert_eq!(run_link_in_text_block(&doc).len(), 0);
    }
}
