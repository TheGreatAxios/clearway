//! Spec-compliant HTML5 parser into the flat arena [`Document`] IR.

use std::collections::BTreeMap;

use ego_tree::NodeRef;
use scraper::{Html, Node as ScraperNode};

use crate::ir::{Document, Node, NodeId};

/// Parses an HTML5 document string using `html5ever` (via `scraper`) into a
/// browser-agnostic flat arena [`Document`].
pub fn parse_html(html: &str) -> Document {
    let parsed = Html::parse_document(html);
    let mut nodes = Vec::with_capacity(64);
    let root_id = build_node(parsed.tree.root(), None, &mut nodes);
    Document::new(nodes, root_id)
}

fn build_node(
    tree_node: NodeRef<'_, ScraperNode>,
    parent: Option<NodeId>,
    nodes: &mut Vec<Node>,
) -> NodeId {
    let id = nodes.len();
    let (tag, attrs, text) = match tree_node.value() {
        ScraperNode::Document => ("#document".to_owned(), BTreeMap::new(), String::new()),
        ScraperNode::Element(el) => {
            let mut map = BTreeMap::new();
            for (k, v) in el.attrs() {
                map.insert(k.to_ascii_lowercase(), v.to_owned());
            }
            (el.name().to_ascii_lowercase(), map, String::new())
        }
        ScraperNode::Text(t) => ("#text".to_owned(), BTreeMap::new(), t.text.to_string()),
        _ => ("#ignored".to_owned(), BTreeMap::new(), String::new()),
    };

    nodes.push(Node {
        id,
        parent,
        children: Vec::new(),
        tag,
        attrs,
        text,
        computed: None,
        rect: None,
        frame_tested: None,
        frame_has_focusable: None,
        media_duration_secs: None,
    });

    let mut child_ids = Vec::new();
    for child in tree_node.children() {
        match child.value() {
            ScraperNode::Element(_) | ScraperNode::Text(_) | ScraperNode::Document => {
                let cid = build_node(child, Some(id), nodes);
                child_ids.push(cid);
            }
            _ => {}
        }
    }
    if let Some(slot) = nodes.get_mut(id) {
        slot.children = child_ids;
    }

    id
}
