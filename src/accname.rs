//! W3C Accessible Name and Description Computation (`AccName` 1.2) core.
//!
//! Precedence order implemented:
//! 1. `aria-labelledby` (IDREF/IDREFS lookup, including referenced hidden nodes)
//! 2. `aria-label`
//! 3. Native host-language labeling (`<label for>`, ancestor `<label>`, `alt`,
//!    `value` on button inputs, `<svg><title>`, `<legend>`, `<caption>`)
//! 4. Visible text subtree (skipping `hidden`, `display: none`, `aria-hidden="true"`)
//! 5. `title` attribute fallback

use crate::ir::{Document, Node, NodeId, normalize_whitespace};

/// Computes the W3C `AccName` 1.2 accessible name for `id`.
pub fn accessible_name(doc: &Document, id: NodeId) -> String {
    let mut visited = Vec::with_capacity(8);
    normalize_whitespace(&compute_name(doc, id, false, false, &mut visited))
}

fn compute_name(
    doc: &Document,
    id: NodeId,
    in_labelledby: bool,
    allow_hidden: bool,
    visited: &mut Vec<NodeId>,
) -> String {
    if visited.contains(&id) {
        return String::new();
    }
    let Some(node) = doc.node(id) else {
        return String::new();
    };
    if node.is_text() {
        return node.text.clone();
    }
    if !node.is_element() {
        return String::new();
    }
    if !allow_hidden && !doc.is_in_accessibility_tree(id) {
        return String::new();
    }

    visited.push(id);
    let result = compute_element_name(doc, node, in_labelledby, allow_hidden, visited);
    visited.pop();
    result
}

fn compute_element_name(
    doc: &Document,
    node: &Node,
    in_labelledby: bool,
    allow_hidden: bool,
    visited: &mut Vec<NodeId>,
) -> String {
    // Step 2B: aria-labelledby (do not recurse into another aria-labelledby).
    if !in_labelledby && let Some(labelledby) = node.attr("aria-labelledby") {
        let mut parts = Vec::new();
        for ref_id in labelledby.split_ascii_whitespace() {
            if let Some(target_id) = doc.find_by_id(ref_id) {
                let text = compute_name(doc, target_id, true, true, visited);
                let norm = normalize_whitespace(&text);
                if !norm.is_empty() {
                    parts.push(norm);
                }
            }
        }
        if !parts.is_empty() {
            return parts.join(" ");
        }
    }

    // Step 2C: aria-label.
    if let Some(label) = node.attr("aria-label") {
        let norm = normalize_whitespace(label);
        if !norm.is_empty() {
            return norm;
        }
    }

    // Step 2E: Embedded control when computing name via aria-labelledby.
    if in_labelledby {
        if node.tag == "select" {
            return selected_option_text(doc, node);
        }
        if crate::aria::effective_role(doc, node) == Some("progressbar") {
            return String::new();
        }
    }

    // Step 2D: Native host-language labeling.
    if let Some(native) = compute_native_label(doc, node, visited) {
        let norm = normalize_whitespace(&native);
        if !norm.is_empty() {
            return norm;
        }
    }

    // Step 2F: Name from content (visible subtree).
    if supports_name_from_content(node, in_labelledby) {
        let subtree = collect_subtree_text(doc, node, allow_hidden, in_labelledby, None, visited);
        let norm = normalize_whitespace(&subtree);
        if !norm.is_empty() {
            return norm;
        }
    }

    // HTML labelable <button> fallback: associated <label for> or wrapping <label>.
    if node.tag == "button"
        && let Some(label_text) = associated_labels_text(doc, node, visited)
    {
        let norm = normalize_whitespace(&label_text);
        if !norm.is_empty() {
            return norm;
        }
    }

    // Step 2I: Tooltip (`title` attribute) fallback.
    if let Some(title) = node.attr("title") {
        let norm = normalize_whitespace(title);
        if !norm.is_empty() {
            return norm;
        }
    }

    String::new()
}

fn compute_native_label(doc: &Document, node: &Node, visited: &mut Vec<NodeId>) -> Option<String> {
    match node.tag.as_str() {
        "img" | "area" => {
            if crate::aria::is_effectively_presentational(node) {
                None
            } else {
                node.attr("alt").map(str::to_owned)
            }
        }
        "input" => compute_input_native_label(doc, node, visited),
        "select" | "textarea" | "meter" | "output" | "progress" => {
            associated_labels_text(doc, node, visited)
        }
        "svg" | "circle" | "rect" | "path" | "polygon" | "ellipse" | "g" | "line" | "polyline" => {
            svg_title_text(doc, node)
        }
        "fieldset" => first_child_tag_text(doc, node, "legend", visited),
        "table" => first_child_tag_text(doc, node, "caption", visited),
        _ => None,
    }
}

fn compute_input_native_label(
    doc: &Document,
    node: &Node,
    visited: &mut Vec<NodeId>,
) -> Option<String> {
    let ty = node
        .attr("type")
        .map_or("text", str::trim)
        .to_ascii_lowercase();
    match ty.as_str() {
        "image" => {
            if let Some(alt) = node.attr("alt") {
                let norm = normalize_whitespace(alt);
                if !norm.is_empty() {
                    return Some(norm);
                }
            }
            associated_labels_text(doc, node, visited)
        }
        "button" | "submit" | "reset" => {
            if let Some(val) = node.attr("value") {
                let norm = normalize_whitespace(val);
                if !norm.is_empty() {
                    return Some(norm);
                }
                return associated_labels_text(doc, node, visited);
            }
            match ty.as_str() {
                "submit" => Some("Submit".to_owned()),
                "reset" => Some("Reset".to_owned()),
                _ => associated_labels_text(doc, node, visited),
            }
        }
        "hidden" => None,
        _ => associated_labels_text(doc, node, visited),
    }
}

fn associated_labels_text(
    doc: &Document,
    control: &Node,
    visited: &mut Vec<NodeId>,
) -> Option<String> {
    let mut parts = Vec::new();

    // 1. Explicit <label for="id"> in document order.
    if let Some(control_id) = control.id_attr() {
        for &label_id in doc.labels_for(control_id) {
            if let Some(el) = doc.node(label_id)
                && doc.is_in_accessibility_tree(el.id)
            {
                let text = collect_subtree_text(doc, el, false, false, Some(control.id), visited);
                let norm = normalize_whitespace(&text);
                if !norm.is_empty() {
                    parts.push(norm);
                }
            }
        }
    }

    // 2. Implicit ancestor <label> wrapping this control.
    if parts.is_empty() {
        for anc_id in doc.ancestors(control.id) {
            if let Some(anc) = doc.node(anc_id)
                && anc.tag == "label"
            {
                let text = collect_subtree_text(doc, anc, false, false, Some(control.id), visited);
                let norm = normalize_whitespace(&text);
                if !norm.is_empty() {
                    parts.push(norm);
                }
                break;
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn svg_title_text(doc: &Document, svg: &Node) -> Option<String> {
    for &cid in &svg.children {
        if let Some(child) = doc.node(cid)
            && child.tag == "title"
        {
            let text = collect_raw_text(doc, cid);
            let norm = normalize_whitespace(&text);
            if !norm.is_empty() {
                return Some(norm);
            }
        }
    }
    None
}

fn first_child_tag_text(
    doc: &Document,
    parent: &Node,
    tag: &str,
    visited: &mut Vec<NodeId>,
) -> Option<String> {
    for &cid in &parent.children {
        if let Some(child) = doc.node(cid)
            && child.tag == tag
        {
            let text = compute_name(doc, cid, false, false, visited);
            let norm = normalize_whitespace(&text);
            if !norm.is_empty() {
                return Some(norm);
            }
        }
    }
    None
}

fn supports_name_from_content(node: &Node, in_labelledby: bool) -> bool {
    if in_labelledby {
        return true;
    }
    if let Some(role) = crate::aria::effective_explicit_role(node) {
        return matches!(
            role,
            "button"
                | "cell"
                | "checkbox"
                | "columnheader"
                | "doc-backlink"
                | "doc-biblioref"
                | "doc-glossref"
                | "doc-noteref"
                | "gridcell"
                | "heading"
                | "link"
                | "menuitem"
                | "menuitemcheckbox"
                | "menuitemradio"
                | "option"
                | "radio"
                | "row"
                | "rowheader"
                | "switch"
                | "tab"
                | "tooltip"
                | "treeitem"
        );
    }
    matches!(
        node.tag.as_str(),
        "a" | "button"
            | "caption"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "label"
            | "legend"
            | "li"
            | "option"
            | "p"
            | "span"
            | "summary"
            | "td"
            | "th"
    )
}

/// Returns the text of the selected `<option>` in a `<select>` (or the first `<option>`
/// when none carries the `selected` attribute) per `AccName` 1.2 Step 2E.
fn selected_option_text(doc: &Document, select: &Node) -> String {
    let mut first_opt: Option<NodeId> = None;
    let mut selected_opt: Option<NodeId> = None;
    let mut stack: Vec<NodeId> = select.children.iter().rev().copied().collect();
    while let Some(cid) = stack.pop() {
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if !child.is_element() {
            continue;
        }
        if child.tag == "option" {
            if first_opt.is_none() {
                first_opt = Some(cid);
            }
            if child.has_attr("selected") {
                selected_opt = Some(cid);
                break;
            }
        } else if child.tag == "optgroup" {
            for &gcid in child.children.iter().rev() {
                stack.push(gcid);
            }
        }
    }
    selected_opt
        .or(first_opt)
        .map_or_default(|oid| normalize_whitespace(&collect_raw_text(doc, oid)))
}

fn collect_subtree_text(
    doc: &Document,
    parent: &Node,
    allow_hidden: bool,
    in_labelledby: bool,
    skip_id: Option<NodeId>,
    visited: &mut Vec<NodeId>,
) -> String {
    let in_embedded_label = in_labelledby || skip_id.is_some();
    let mut parts = Vec::new();
    for &cid in &parent.children {
        if skip_id == Some(cid) || visited.contains(&cid) {
            continue;
        }
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if child.is_text() {
            if !child.text.is_empty() {
                parts.push(child.text.clone());
            }
            continue;
        }
        if !child.is_element() {
            continue;
        }
        if !allow_hidden && !doc.is_in_accessibility_tree(cid) {
            continue;
        }
        // AccName 1.2 Step 2E: embedded <select> contributes its chosen <option>'s text.
        if in_embedded_label && child.tag == "select" {
            let opt_text = selected_option_text(doc, child);
            if !opt_text.is_empty() {
                parts.push(opt_text);
            }
            continue;
        }
        // AccName 1.2 Step 2E: read-only progressbar is not a user-adjustable embedded control
        // and does not support name-from-content.
        if in_embedded_label && crate::aria::effective_role(doc, child) == Some("progressbar") {
            continue;
        }
        // Skip other embedded form controls when collecting a <label>'s own text.
        if skip_id.is_some() && matches!(child.tag.as_str(), "input" | "textarea" | "button") {
            continue;
        }
        // Effectively presentational <img> (e.g. role="presentation|none" without conflict)
        // does not contribute its alt text to a parent element's accessible name.
        if child.tag == "img" && crate::aria::is_effectively_presentational(child) {
            continue;
        }
        // Embedded elements with their own accessible name (e.g. <img alt>, <svg><title>)
        // or generic inline wrappers contribute their computed name or subtree.
        let piece = if matches!(child.tag.as_str(), "img" | "svg" | "input")
            || child.has_attr("aria-label")
            || child.has_attr("aria-labelledby")
        {
            compute_name(doc, cid, false, allow_hidden, visited)
        } else {
            collect_subtree_text(doc, child, allow_hidden, in_labelledby, skip_id, visited)
        };
        if !piece.is_empty() {
            parts.push(piece);
        }
    }
    parts.join(" ")
}

/// Collects raw descendant text regardless of visibility (used for `<title>`).
pub fn collect_raw_text(doc: &Document, id: NodeId) -> String {
    let Some(node) = doc.node(id) else {
        return String::new();
    };
    if node.is_text() {
        return node.text.clone();
    }
    let mut out = String::new();
    for &cid in &node.children {
        out.push_str(&collect_raw_text(doc, cid));
    }
    out
}
