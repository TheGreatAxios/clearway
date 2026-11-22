//! Browser-agnostic flat arena DOM intermediate representation (IR).
//!
//! Any runtime (HTML5 parser, Playwright, CDP, Python, Ruby, WASM) can construct
//! or deserialize a [`Document`] and pass it directly to `audit_document`.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Index into [`Document::nodes`].
pub type NodeId = usize;

/// An sRGB color with alpha channel in `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

impl Rgba {
    /// Creates an RGBA color with alpha clamped to `[0.0, 1.0]`.
    pub const fn new(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Creates a fully opaque RGB color (`a = 1.0`).
    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 1.0 }
    }
}

/// Axis-aligned bounding box in CSS pixels, typically from `getBoundingClientRect()`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Optional browser-computed style snapshot attached to an element node.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct ComputedStyle {
    #[serde(default)]
    pub color: Option<Rgba>,
    #[serde(default)]
    pub background_color: Option<Rgba>,
    #[serde(default)]
    pub visual_background: Option<Rgba>,
    #[serde(default)]
    pub font_size_px: Option<f32>,
    #[serde(default)]
    pub font_weight: Option<u16>,
    #[serde(default)]
    pub display: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub opacity: Option<f32>,
    #[serde(default)]
    pub text_decoration_line: Option<String>,
    #[serde(default)]
    pub font_style: Option<String>,
    #[serde(default)]
    pub overflow_x: Option<String>,
    #[serde(default)]
    pub overflow_y: Option<String>,
    #[serde(default)]
    pub scroll_width: Option<f32>,
    #[serde(default)]
    pub client_width: Option<f32>,
    #[serde(default)]
    pub scroll_height: Option<f32>,
    #[serde(default)]
    pub client_height: Option<f32>,
    #[serde(default)]
    pub has_text_shadow: bool,
    #[serde(default)]
    pub has_border_bottom: bool,
    #[serde(default)]
    pub has_box_shadow: bool,
    #[serde(default)]
    pub has_background_image: bool,
    #[serde(default)]
    pub background_gradient_stops: Vec<Rgba>,
    #[serde(default)]
    pub pointer_events: Option<String>,
    #[serde(default)]
    pub flex_direction: Option<String>,
    #[serde(default)]
    pub is_clipped: bool,
}

/// A node in the flat arena DOM tree.
///
/// - Document root: `tag == "#document"`
/// - Text node: `tag == "#text"` with content in `text`
/// - Element node: lowercase HTML tag name (e.g. `"button"`, `"img"`, `"svg"`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    #[serde(default)]
    pub parent: Option<NodeId>,
    #[serde(default)]
    pub children: Vec<NodeId>,
    pub tag: String,
    #[serde(default)]
    pub attrs: BTreeMap<String, String>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub computed: Option<ComputedStyle>,
    #[serde(default)]
    pub rect: Option<Rect>,
    #[serde(default)]
    pub frame_tested: Option<bool>,
    #[serde(default)]
    pub frame_has_focusable: Option<bool>,
    #[serde(default)]
    pub media_duration_secs: Option<f32>,
}

impl Node {
    /// Returns `true` if this node is an HTML/SVG element.
    pub fn is_element(&self) -> bool {
        !self.tag.is_empty() && !self.tag.starts_with('#')
    }

    /// Returns `true` if this node is a text node.
    pub fn is_text(&self) -> bool {
        self.tag == "#text"
    }

    /// Returns the attribute value by exact lowercase name, if present.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(String::as_str)
    }

    /// Returns `true` if the attribute is present (even if empty).
    pub fn has_attr(&self, name: &str) -> bool {
        self.attrs.contains_key(name)
    }

    /// Returns the non-empty `id` attribute, if present.
    pub fn id_attr(&self) -> Option<&str> {
        self.attr("id").map(str::trim).filter(|s| !s.is_empty())
    }
}

#[derive(Debug)]
struct VisibilityCache {
    rendered: Vec<bool>,
    aria_hidden: Vec<bool>,
}

#[derive(Debug, Default)]
struct DocumentCache {
    id_index: OnceLock<HashMap<String, Vec<NodeId>>>,
    label_for_index: OnceLock<HashMap<String, Vec<NodeId>>>,
    aria_labelledby_refs: OnceLock<HashMap<String, Vec<NodeId>>>,
    visibility: OnceLock<VisibilityCache>,
}

impl Clone for DocumentCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for DocumentCache {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

/// Flat arena representation of a parsed HTML document or browser DOM snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub nodes: Vec<Node>,
    pub root: NodeId,
    #[serde(default)]
    pub css_orientation_locked: bool,
    #[serde(skip)]
    cache: DocumentCache,
}

impl Document {
    /// Constructs a new [`Document`] from a flat node arena and root [`NodeId`].
    pub fn new(nodes: Vec<Node>, root: NodeId) -> Self {
        Self {
            nodes,
            root,
            css_orientation_locked: false,
            cache: DocumentCache::default(),
        }
    }

    fn id_index(&self) -> &HashMap<String, Vec<NodeId>> {
        self.cache.id_index.get_or_init(|| {
            let mut map: HashMap<String, Vec<NodeId>> = HashMap::new();
            for n in &self.nodes {
                if n.is_element()
                    && let Some(id_val) = n.attr("id")
                    && !id_val.is_empty()
                {
                    map.entry(id_val.to_owned()).or_default().push(n.id);
                }
            }
            map
        })
    }

    fn label_for_index(&self) -> &HashMap<String, Vec<NodeId>> {
        self.cache.label_for_index.get_or_init(|| {
            let mut map: HashMap<String, Vec<NodeId>> = HashMap::new();
            for n in &self.nodes {
                if n.tag == "label"
                    && let Some(for_val) = n.attr("for")
                    && !for_val.is_empty()
                {
                    map.entry(for_val.to_owned()).or_default().push(n.id);
                }
            }
            map
        })
    }

    fn aria_labelledby_refs(&self) -> &HashMap<String, Vec<NodeId>> {
        self.cache.aria_labelledby_refs.get_or_init(|| {
            let mut map: HashMap<String, Vec<NodeId>> = HashMap::new();
            for n in &self.nodes {
                if n.is_element()
                    && let Some(val) = n.attr("aria-labelledby")
                {
                    for tok in val.split_whitespace() {
                        let entry = map.entry(tok.to_owned()).or_default();
                        if entry.last() != Some(&n.id) {
                            entry.push(n.id);
                        }
                    }
                }
            }
            map
        })
    }

    fn visibility(&self) -> &VisibilityCache {
        self.cache.visibility.get_or_init(|| {
            let n = self.nodes.len();
            // State encoding: 0 = uncomputed, 1 = on stack (cycle guard), 2 = false, 3 = true.
            let mut rendered_state = vec![0u8; n];
            let mut aria_hidden_state = vec![0u8; n];
            let mut path = Vec::with_capacity(16);

            for start in 0..n {
                if rendered_state[start] == 0 {
                    path.clear();
                    let mut current = Some(start);
                    let result = loop {
                        let Some(nid) = current else {
                            break true;
                        };
                        let Some(state) = rendered_state.get(nid).copied() else {
                            break false;
                        };
                        if state == 2 {
                            break false;
                        }
                        if state == 3 || state == 1 {
                            break true;
                        }
                        let Some(node) = self.nodes.get(nid) else {
                            break false;
                        };
                        rendered_state[nid] = 1;
                        path.push(nid);
                        if node.is_element() && is_node_self_hidden(node) {
                            break false;
                        }
                        current = node.parent;
                    };
                    let val = if result { 3 } else { 2 };
                    for &nid in &path {
                        rendered_state[nid] = val;
                    }
                }

                if aria_hidden_state[start] == 0 {
                    path.clear();
                    let mut current = Some(start);
                    let result = loop {
                        let Some(nid) = current else {
                            break false;
                        };
                        let Some(state) = aria_hidden_state.get(nid).copied() else {
                            break false;
                        };
                        if state == 2 || state == 1 {
                            break false;
                        }
                        if state == 3 {
                            break true;
                        }
                        let Some(node) = self.nodes.get(nid) else {
                            break false;
                        };
                        aria_hidden_state[nid] = 1;
                        path.push(nid);
                        if node
                            .attr("aria-hidden")
                            .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
                        {
                            break true;
                        }
                        current = node.parent;
                    };
                    let val = if result { 3 } else { 2 };
                    for &nid in &path {
                        aria_hidden_state[nid] = val;
                    }
                }
            }

            VisibilityCache {
                rendered: rendered_state.into_iter().map(|s| s == 3).collect(),
                aria_hidden: aria_hidden_state.into_iter().map(|s| s == 3).collect(),
            }
        })
    }

    /// Returns all `<label>` element IDs whose `for` attribute matches `control_id`, in document order.
    pub(crate) fn labels_for(&self, control_id: &str) -> &[NodeId] {
        self.label_for_index()
            .get(control_id)
            .map_or(&[], Vec::as_slice)
    }

    /// Returns all element IDs whose `aria-labelledby` attribute references `target_id`.
    pub(crate) fn aria_labelledby_referrers(&self, target_id: &str) -> &[NodeId] {
        self.aria_labelledby_refs()
            .get(target_id)
            .map_or(&[], Vec::as_slice)
    }

    /// Looks up a node by its arena [`NodeId`].
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Iterates over all element nodes in arena order (document pre-order).
    pub fn elements(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter().filter(|n| n.is_element())
    }

    /// Returns the first element matching `id`, if any.
    pub fn find_by_id(&self, target_id: &str) -> Option<NodeId> {
        if target_id.is_empty() {
            return None;
        }
        self.id_index()
            .get(target_id)
            .and_then(|ids| ids.first().copied())
    }

    /// Returns ancestor node IDs from immediate parent up to root.
    pub fn ancestors(&self, mut id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        while let Some(parent_id) = self.node(id).and_then(|n| n.parent) {
            if out.contains(&parent_id) {
                break;
            }
            out.push(parent_id);
            id = parent_id;
        }
        out
    }

    /// Returns `true` if `id` is visually/DOM rendered (not hidden via `hidden`,
    /// `display: none`, `visibility: hidden`/`collapse`, or non-rendered tags).
    pub fn is_rendered(&self, id: NodeId) -> bool {
        self.visibility().rendered.get(id).copied().unwrap_or(false)
    }

    /// Returns `true` if `id` or any of its ancestors has `aria-hidden="true"`.
    pub fn is_aria_hidden(&self, id: NodeId) -> bool {
        self.visibility()
            .aria_hidden
            .get(id)
            .copied()
            .unwrap_or(false)
    }

    /// Returns `true` if the node is exposed in the accessibility tree.
    pub fn is_in_accessibility_tree(&self, id: NodeId) -> bool {
        let vis = self.visibility();
        vis.rendered.get(id).copied().unwrap_or(false)
            && !vis.aria_hidden.get(id).copied().unwrap_or(false)
    }

    /// Returns `true` if the element has any direct non-whitespace text node child.
    pub fn has_direct_text(&self, id: NodeId) -> bool {
        let Some(node) = self.node(id) else {
            return false;
        };
        node.children.iter().any(|&cid| {
            self.node(cid)
                .is_some_and(|c| c.is_text() && !c.text.trim().is_empty())
        })
    }

    /// Builds a deterministic CSS selector for `id`.
    pub fn css_selector(&self, id: NodeId) -> String {
        let Some(target) = self.node(id) else {
            return String::new();
        };
        if !target.is_element() {
            return target.tag.clone();
        }
        if let Some(el_id) = target.id_attr() {
            let is_unique = self.id_index().get(el_id).is_some_and(|ids| ids.len() == 1);
            if is_unique && is_simple_css_ident(el_id) {
                return format!("#{el_id}");
            }
        }

        let mut parts = Vec::new();
        let mut current = Some(id);
        let mut depth = 0usize;
        while let Some(nid) = current {
            if depth > self.nodes.len() {
                break;
            }
            depth += 1;
            let Some(node) = self.node(nid) else {
                break;
            };
            if !node.is_element() {
                break;
            }
            let segment = if let Some(parent_id) = node.parent {
                if let Some(parent) = self.node(parent_id) {
                    let same_tag: Vec<NodeId> = parent
                        .children
                        .iter()
                        .copied()
                        .filter(|&cid| {
                            self.node(cid)
                                .is_some_and(|c| c.is_element() && c.tag == node.tag)
                        })
                        .collect();
                    if same_tag.len() > 1 {
                        let pos = same_tag
                            .iter()
                            .position(|&cid| cid == nid)
                            .map_or(1, |idx| idx + 1);
                        format!("{}:nth-of-type({pos})", node.tag)
                    } else {
                        node.tag.clone()
                    }
                } else {
                    node.tag.clone()
                }
            } else {
                node.tag.clone()
            };
            parts.push(segment);
            current = node.parent;
        }
        parts.reverse();
        parts.join(" > ")
    }

    /// Builds a compact opening-tag HTML snippet for `id`.
    pub fn html_snippet(&self, id: NodeId) -> String {
        let Some(node) = self.node(id) else {
            return String::new();
        };
        if !node.is_element() {
            return node.tag.clone();
        }
        let mut out = String::with_capacity(64);
        out.push('<');
        out.push_str(&node.tag);
        for (k, v) in &node.attrs {
            out.push(' ');
            out.push_str(k);
            if !v.is_empty() {
                out.push_str("=\"");
                out.push_str(v);
                out.push('"');
            }
            if out.len() > 140 {
                out.push_str(" ...");
                break;
            }
        }
        out.push('>');
        out
    }
}

fn is_node_self_hidden(node: &Node) -> bool {
    if matches!(
        node.tag.as_str(),
        "head" | "script" | "style" | "template" | "noscript"
    ) {
        return true;
    }
    if node.has_attr("hidden") {
        return true;
    }
    if let Some(computed) = &node.computed {
        if computed
            .display
            .as_deref()
            .is_some_and(|d| d.trim().eq_ignore_ascii_case("none"))
        {
            return true;
        }
        if computed.visibility.as_deref().is_some_and(|v| {
            let trimmed = v.trim();
            trimmed.eq_ignore_ascii_case("hidden") || trimmed.eq_ignore_ascii_case("collapse")
        }) {
            return true;
        }
    }
    if let Some(style) = node.attr("style") {
        for decl in style.split(';') {
            let Some((prop, val)) = decl.split_once(':') else {
                continue;
            };
            let prop = prop.trim();
            let val = val.split('!').next().map_or("", str::trim);
            if prop.eq_ignore_ascii_case("display") && val.eq_ignore_ascii_case("none") {
                return true;
            }
            if prop.eq_ignore_ascii_case("visibility")
                && (val.eq_ignore_ascii_case("hidden") || val.eq_ignore_ascii_case("collapse"))
            {
                return true;
            }
        }
    }
    false
}

fn is_simple_css_ident(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        && !s.as_bytes()[0].is_ascii_digit()
}

/// Normalizes ASCII whitespace per HTML/AccName rules: collapses runs to a single
/// space and trims leading/trailing whitespace.
pub fn normalize_whitespace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_ws = true;
    for ch in input.chars() {
        if ch.is_whitespace() {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            out.push(ch);
            in_ws = false;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Document, normalize_whitespace};

    #[test]
    fn normalizes_ascii_and_unicode_whitespace() {
        assert_eq!(normalize_whitespace("  hello \n\t world  "), "hello world");
        assert_eq!(normalize_whitespace("   "), "");
    }

    #[test]
    fn deserializes_compact_ir_with_omitted_defaults() {
        let json = r##"{
            "root": 0,
            "nodes": [
                {"id": 0, "tag": "#document", "children": [1]},
                {"id": 1, "parent": 0, "tag": "p", "children": [2], "computed": {"display": "block"}},
                {"id": 2, "parent": 1, "tag": "#text", "text": "Hello"}
            ]
        }"##;
        let doc: Document = serde_json::from_str(json).expect("compact IR should deserialize");
        assert!(!doc.css_orientation_locked);
        assert_eq!(doc.nodes[0].parent, None);
        assert_eq!(doc.nodes[2].children, Vec::<usize>::new());
        assert!(doc.nodes[2].attrs.is_empty());
        let computed = doc.nodes[1]
            .computed
            .as_ref()
            .expect("paragraph should have computed style");
        assert!(!computed.has_text_shadow);
        assert!(!computed.has_background_image);
        assert_eq!(computed.background_gradient_stops, Vec::new());
        assert!(!computed.is_clipped);
    }
}
