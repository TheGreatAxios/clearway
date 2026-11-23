//! Accessible name rules: `image-alt`, `button-name`, `input-button-name`,
//! `link-name`, `input-image-alt`, `area-alt`, `frame-title`, `frame-title-unique`,
//! `svg-img-alt`, `role-img-alt`, `object-alt`, `summary-name`,
//! `image-redundant-alt`, `server-side-image-map`, and `identical-links-same-purpose`.

use std::collections::{BTreeMap, HashSet};

use crate::accname::accessible_name;
use crate::aria::{effective_explicit_role, explicit_role, is_effectively_presentational};
use crate::ir::{Document, MAX_TRAVERSAL_DEPTH, NodeId, normalize_whitespace};
use crate::rule::{Impact, Rule, RuleMeta, Violation};

pub struct ImageAlt;
pub struct ButtonName;
pub struct InputButtonName;
pub struct LinkName;
pub struct InputImageAlt;
pub struct AreaAlt;
pub struct FrameTitle;
pub struct FrameTitleUnique;
pub struct SvgImgAlt;
pub struct RoleImgAlt;
pub struct ObjectAlt;
pub struct SummaryName;
pub struct ImageRedundantAlt;
pub struct ServerSideImageMap;
pub struct IdenticalLinksSamePurpose;

impl ImageAlt {
    pub const META: RuleMeta = RuleMeta {
        id: "image-alt",
        impact: Impact::Critical,
        wcag: "1.1.1 (A)",
        tags: &[
            "cat.text-alternatives",
            "wcag2a",
            "wcag111",
            "section508",
            "section508.22.a",
            "TTv5",
            "TT7.a",
            "TT7.b",
            "EN-301-549",
            "EN-9.1.1.1",
            "ACT",
            "RGAAv4",
            "RGAA-1.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Images must have alternate text",
        description: "Provide an alt attribute (or empty alt=\"\" for decorative images) on every <img> element.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/non-text-content.html",
    };
}

impl Rule for ImageAlt {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "img" || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if is_effectively_presentational(el) {
                continue;
            }
            // Strictly empty `alt=""` explicitly marks decorative images as valid.
            // Whitespace-only `alt=" "` is NOT decorative and must have a non-empty accessible name.
            if el.attr("alt") == Some("") {
                continue;
            }
            if accessible_name(doc, el.id).is_empty() {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl ButtonName {
    pub const META: RuleMeta = RuleMeta {
        id: "button-name",
        impact: Impact::Critical,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.name-role-value",
            "wcag2a",
            "wcag412",
            "section508",
            "section508.22.a",
            "TTv5",
            "TT6.a",
            "EN-301-549",
            "EN-9.4.1.2",
            "ACT",
            "RGAAv4",
            "RGAA-11.9.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Buttons must have discernible text",
        description: "Ensure every <button> or [role=\"button\"] has visible text, aria-label, or aria-labelledby.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/name-role-value.html",
    };
}

impl Rule for ButtonName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if is_effectively_presentational(el) {
                continue;
            }
            let role = effective_explicit_role(el);
            let is_overridden_widget = matches!(
                role,
                Some(
                    "checkbox"
                        | "combobox"
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
                )
            ) || (role == Some("gridcell") && el.has_attr("disabled"));
            let is_btn = el.tag == "button" && !is_overridden_widget;
            if !is_btn {
                continue;
            }
            if accessible_name(doc, el.id).is_empty() {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl LinkName {
    pub const META: RuleMeta = RuleMeta {
        id: "link-name",
        impact: Impact::Serious,
        wcag: "2.4.4 (A)",
        tags: &[
            "cat.name-role-value",
            "wcag2a",
            "wcag244",
            "wcag412",
            "section508",
            "section508.22.a",
            "TTv5",
            "TT6.a",
            "EN-301-549",
            "EN-9.2.4.4",
            "EN-9.4.1.2",
            "ACT",
            "RGAAv4",
            "RGAA-6.2.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Links must have discernible text",
        description: "Ensure every <a href> or [role=\"link\"] has visible text, an image with alt text, aria-label, or aria-labelledby.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/link-purpose-in-context.html",
    };
}

impl Rule for LinkName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if is_effectively_presentational(el) {
                continue;
            }
            let role = effective_explicit_role(el);
            let is_link = (el.tag == "a" && el.has_attr("href") && role.is_none())
                || matches!(
                    role,
                    Some(
                        "link" | "doc-backlink" | "doc-biblioref" | "doc-glossref" | "doc-noteref"
                    )
                );
            if !is_link {
                continue;
            }
            if accessible_name(doc, el.id).is_empty() {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl InputImageAlt {
    pub const META: RuleMeta = RuleMeta {
        id: "input-image-alt",
        impact: Impact::Critical,
        wcag: "1.1.1 (A)",
        tags: &[
            "cat.text-alternatives",
            "wcag2a",
            "wcag111",
            "wcag412",
            "section508",
            "section508.22.a",
            "TTv5",
            "TT7.a",
            "EN-301-549",
            "EN-9.1.1.1",
            "EN-9.4.1.2",
            "ACT",
            "RGAAv4",
            "RGAA-1.1.3",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Image buttons (<input type=\"image\">) must have alternate text",
        description: "Provide a non-empty alt, aria-label, aria-labelledby, or title on <input type=\"image\">.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/non-text-content.html",
    };
}

impl Rule for InputImageAlt {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "input" || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if !el
                .attr("type")
                .is_some_and(|t| t.trim().eq_ignore_ascii_case("image"))
            {
                continue;
            }
            if accessible_name(doc, el.id).is_empty() {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl AreaAlt {
    pub const META: RuleMeta = RuleMeta {
        id: "area-alt",
        impact: Impact::Critical,
        wcag: "1.1.1 (A)",
        tags: &[
            "cat.text-alternatives",
            "wcag2a",
            "wcag244",
            "wcag412",
            "section508",
            "section508.22.a",
            "TTv5",
            "TT6.a",
            "EN-301-549",
            "EN-9.2.4.4",
            "EN-9.4.1.2",
            "ACT",
            "RGAAv4",
            "RGAA-1.1.2",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Active <area> elements must have alternate text",
        description: "Provide a non-empty alt, aria-label, or aria-labelledby on every <area href>.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/non-text-content.html",
    };
}

impl Rule for AreaAlt {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "area" || !el.has_attr("href") || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if accessible_name(doc, el.id).is_empty() {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl FrameTitle {
    pub const META: RuleMeta = RuleMeta {
        id: "frame-title",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.text-alternatives",
            "wcag2a",
            "wcag412",
            "section508",
            "section508.22.i",
            "TTv5",
            "TT12.d",
            "EN-301-549",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-2.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Frames must have an accessible name",
        description: "Provide a non-empty title, aria-label, or aria-labelledby on every <iframe> and <frame>.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/name-role-value.html",
    };
}

impl Rule for FrameTitle {
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
            if is_effectively_presentational(el) && !el.has_attr("title") {
                continue;
            }
            if el.tag == "iframe"
                && el
                    .attr("tabindex")
                    .and_then(|t| t.trim().parse::<i32>().ok())
                    .is_some_and(|ti| ti < 0)
            {
                continue;
            }
            if accessible_name(doc, el.id).is_empty() {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl SvgImgAlt {
    pub const META: RuleMeta = RuleMeta {
        id: "svg-img-alt",
        impact: Impact::Serious,
        wcag: "1.1.1 (A)",
        tags: &[
            "cat.text-alternatives",
            "wcag2a",
            "wcag111",
            "section508",
            "section508.22.a",
            "TTv5",
            "TT7.a",
            "EN-301-549",
            "EN-9.1.1.1",
            "ACT",
            "RGAAv4",
            "RGAA-1.1.5",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "<svg> elements with an img role must have an accessible name",
        description: "Provide a <title> child, aria-label, aria-labelledby, or title attribute on <svg role=\"img\">.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/non-text-content.html",
    };
}

const SVG_IMG_TAGS: &[&str] = &[
    "svg", "circle", "rect", "path", "polygon", "ellipse", "g", "line", "polyline",
];

impl Rule for SvgImgAlt {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !SVG_IMG_TAGS.contains(&el.tag.as_str()) || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if !matches!(
                explicit_role(el),
                Some("img" | "image" | "graphics-document" | "graphics-symbol")
            ) {
                continue;
            }
            if accessible_name(doc, el.id).is_empty() {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl InputButtonName {
    pub const META: RuleMeta = RuleMeta {
        id: "input-button-name",
        impact: Impact::Critical,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.name-role-value",
            "wcag2a",
            "wcag412",
            "section508",
            "section508.22.a",
            "TTv5",
            "TT5.c",
            "EN-301-549",
            "EN-9.4.1.2",
            "ACT",
            "RGAAv4",
            "RGAA-11.9.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Input buttons must have discernible text",
        description: "Ensure every <input type=\"button|submit|reset\"> has a non-empty accessible name.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/name-role-value.html",
    };
}

impl Rule for InputButtonName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "input" || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if is_effectively_presentational(el) {
                continue;
            }
            let ty = el.attr("type").map(str::trim).map(str::to_ascii_lowercase);
            if !matches!(ty.as_deref(), Some("button" | "submit" | "reset")) {
                continue;
            }
            if accessible_name(doc, el.id).is_empty() {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl RoleImgAlt {
    pub const META: RuleMeta = RuleMeta {
        id: "role-img-alt",
        impact: Impact::Serious,
        wcag: "1.1.1 (A)",
        tags: &[
            "cat.text-alternatives",
            "wcag2a",
            "wcag111",
            "section508",
            "section508.22.a",
            "TTv5",
            "TT7.a",
            "EN-301-549",
            "EN-9.1.1.1",
            "ACT",
            "RGAAv4",
            "RGAA-1.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "[role=\"img\"] elements must have alternative text",
        description: "Ensure elements with role=\"img\" have a non-empty aria-label, aria-labelledby, or title.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/non-text-content.html",
    };
}

impl Rule for RoleImgAlt {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if matches!(el.tag.as_str(), "img" | "area" | "input" | "object")
                || SVG_IMG_TAGS.contains(&el.tag.as_str())
            {
                continue;
            }
            let has_img_role = explicit_role(el) == Some("img")
                || el
                    .attr("role")
                    .is_some_and(|r| r.split_ascii_whitespace().any(|t| t == "image"));
            if !has_img_role {
                continue;
            }
            if accessible_name(doc, el.id).is_empty() {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl ObjectAlt {
    pub const META: RuleMeta = RuleMeta {
        id: "object-alt",
        impact: Impact::Serious,
        wcag: "1.1.1 (A)",
        tags: &[
            "cat.text-alternatives",
            "wcag2a",
            "wcag111",
            "section508",
            "section508.22.a",
            "EN-301-549",
            "EN-9.1.1.1",
            "RGAAv4",
            "RGAA-1.1.6",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "<object> elements must have alternative text",
        description: "Provide aria-label, aria-labelledby, title, or non-empty fallback content inside <object>.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/non-text-content.html",
    };
}

impl Rule for ObjectAlt {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "object" || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if is_effectively_presentational(el) {
                continue;
            }
            if !accessible_name(doc, el.id).is_empty() {
                continue;
            }
            out.push(Violation::from_node(&self.meta(), doc, el.id));
        }
    }
}

impl SummaryName {
    pub const META: RuleMeta = RuleMeta {
        id: "summary-name",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.name-role-value",
            "wcag2a",
            "wcag412",
            "section508",
            "section508.22.a",
            "TTv5",
            "TT6.a",
            "EN-301-549",
            "EN-9.4.1.2",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Summary elements must have discernible text",
        description: "Ensure every <summary> element has visible text, aria-label, aria-labelledby, or title.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/name-role-value.html",
    };
}

impl Rule for SummaryName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "summary" || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            // Per HTML spec, only the first direct child <summary> of a <details>
            // element acts as the disclosure summary control.
            let is_first_details_summary =
                el.parent.and_then(|pid| doc.node(pid)).is_some_and(|p| {
                    p.tag == "details"
                        && p.children
                            .iter()
                            .copied()
                            .find(|&cid| doc.node(cid).is_some_and(|c| c.tag == "summary"))
                            == Some(el.id)
                });
            if !is_first_details_summary {
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

impl FrameTitleUnique {
    pub const META: RuleMeta = RuleMeta {
        id: "frame-title-unique",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.text-alternatives",
            "wcag2a",
            "wcag412",
            "TTv5",
            "TT12.d",
            "EN-301-549",
            "EN-9.4.1.2",
            "RGAAv4",
            "RGAA-2.2.1",
        ],
        enabled_by_default: true,
        needs_review: true,
        help: "Frames must have a unique title attribute",
        description: "Ensure <iframe> and <frame> elements do not share duplicate accessible titles on the same page.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/name-role-value.html",
    };
}

impl Rule for FrameTitleUnique {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let mut by_title: BTreeMap<String, Vec<NodeId>> = BTreeMap::new();
        for el in doc.elements() {
            if !matches!(el.tag.as_str(), "iframe" | "frame")
                || !doc.is_in_accessibility_tree(el.id)
            {
                continue;
            }
            if is_effectively_presentational(el) {
                continue;
            }
            let name = normalize_whitespace(&accessible_name(doc, el.id)).to_ascii_lowercase();
            if !name.is_empty() {
                by_title.entry(name).or_default().push(el.id);
            }
        }
        for (title, nodes) in by_title {
            if nodes.len() > 1 {
                for &nid in &nodes[1..] {
                    out.push(Violation::with_detail(
                        &self.meta(),
                        doc,
                        nid,
                        format!("Duplicate frame title '{title}' shared by multiple frames."),
                    ));
                }
            }
        }
    }
}

impl ImageRedundantAlt {
    pub const META: RuleMeta = RuleMeta {
        id: "image-redundant-alt",
        impact: Impact::Minor,
        wcag: "1.1.1 (Best Practice)",
        tags: &["cat.text-alternatives", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Alternative text of images should not be repeated as text",
        description: "Avoid setting <img> alt text identical to the surrounding button or link text.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/non-text-content.html",
    };
}

impl Rule for ImageRedundantAlt {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "img" || !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if is_effectively_presentational(el) {
                continue;
            }
            let Some(alt_raw) = el.attr("alt") else {
                continue;
            };
            let alt = normalize_whitespace(alt_raw).to_ascii_lowercase();
            if alt.is_empty() {
                continue;
            }
            let container_id = doc.ancestors(el.id).into_iter().find(|&aid| {
                doc.node(aid).is_some_and(|anc| {
                    let role = explicit_role(anc);
                    matches!(
                        anc.tag.as_str(),
                        "button"
                            | "p"
                            | "li"
                            | "td"
                            | "th"
                            | "h1"
                            | "h2"
                            | "h3"
                            | "h4"
                            | "h5"
                            | "h6"
                            | "figcaption"
                            | "label"
                    ) || (anc.tag == "a" && anc.has_attr("href"))
                        || matches!(
                            role,
                            Some(
                                "button"
                                    | "link"
                                    | "listitem"
                                    | "cell"
                                    | "gridcell"
                                    | "columnheader"
                                    | "rowheader"
                                    | "heading"
                                    | "paragraph"
                            )
                        )
                })
            });
            let Some(cid) = container_id else {
                continue;
            };
            let surrounding =
                normalize_whitespace(&collect_text_excluding_imgs(doc, cid)).to_ascii_lowercase();
            if !surrounding.is_empty() && surrounding == alt {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!("Image alt text '{alt_raw}' duplicates the surrounding control text."),
                ));
            }
        }
    }
}

fn collect_text_excluding_imgs(doc: &Document, id: NodeId) -> String {
    let mut visited = HashSet::new();
    visited.insert(id);
    collect_text_excluding_imgs_inner(doc, id, 0, &mut visited)
}

fn collect_text_excluding_imgs_inner(
    doc: &Document,
    id: NodeId,
    depth: usize,
    visited: &mut HashSet<NodeId>,
) -> String {
    if depth >= MAX_TRAVERSAL_DEPTH {
        return String::new();
    }
    let Some(node) = doc.node(id) else {
        return String::new();
    };
    let mut parts = Vec::new();
    for &cid in &node.children {
        if !visited.insert(cid) {
            continue;
        }
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if child.is_text() {
            if !child.text.trim().is_empty() {
                parts.push(child.text.clone());
            }
            continue;
        }
        if !child.is_element() || !doc.is_in_accessibility_tree(cid) || child.tag == "img" {
            continue;
        }
        let sub = collect_text_excluding_imgs_inner(doc, cid, depth + 1, visited);
        if !sub.is_empty() {
            parts.push(sub);
        }
    }
    parts.join(" ")
}

impl ServerSideImageMap {
    pub const META: RuleMeta = RuleMeta {
        id: "server-side-image-map",
        impact: Impact::Minor,
        wcag: "2.1.1 (A)",
        tags: &[
            "cat.text-alternatives",
            "wcag2a",
            "wcag211",
            "section508",
            "section508.22.f",
            "TTv5",
            "TT4.a",
            "EN-301-549",
            "EN-9.2.1.1",
            "RGAAv4",
            "RGAA-1.1.4",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Server-side image maps must not be used",
        description: "Remove the ismap attribute from <img> and use a client-side image map (<map> and <area>) instead.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/keyboard.html",
    };
}

impl Rule for ServerSideImageMap {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag == "img" && doc.is_in_accessibility_tree(el.id) && el.has_attr("ismap") {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl IdenticalLinksSamePurpose {
    pub const META: RuleMeta = RuleMeta {
        id: "identical-links-same-purpose",
        impact: Impact::Minor,
        wcag: "2.4.9 (AAA)",
        tags: &["cat.semantics", "wcag2aaa", "wcag249"],
        enabled_by_default: false,
        needs_review: true,
        help: "Links with the same name must have a similar purpose",
        description: "Ensure links with identical accessible names point to the same destination.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/link-purpose-link-only.html",
    };
}

impl Rule for IdenticalLinksSamePurpose {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        let mut by_name: BTreeMap<String, Vec<(NodeId, String)>> = BTreeMap::new();
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            if is_effectively_presentational(el) {
                continue;
            }
            let role = effective_explicit_role(el);
            let is_link = (el.tag == "a" && el.has_attr("href") && role.is_none())
                || matches!(
                    role,
                    Some(
                        "link" | "doc-backlink" | "doc-biblioref" | "doc-glossref" | "doc-noteref"
                    )
                );
            if !is_link {
                continue;
            }
            let Some(href) = el.attr("href").map(str::trim).filter(|s| !s.is_empty()) else {
                continue;
            };
            let name = normalize_whitespace(&accessible_name(doc, el.id)).to_ascii_lowercase();
            if name.is_empty() {
                continue;
            }
            by_name
                .entry(name)
                .or_default()
                .push((el.id, href.to_owned()));
        }
        for (name, links) in by_name {
            let Some((_, first_href)) = links.first() else {
                continue;
            };
            for (nid, href) in &links[1..] {
                if href != first_href {
                    out.push(Violation::with_detail(
                        &self.meta(),
                        doc,
                        *nid,
                        format!(
                            "Link named '{name}' points to '{href}', differing from earlier destination '{first_href}'."
                        ),
                    ));
                }
            }
        }
    }
}
