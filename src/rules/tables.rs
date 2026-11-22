//! Table accessibility rules: `empty-table-header`, `scope-attr-valid`,
//! `table-duplicate-name`, `table-fake-caption`, `td-has-header`,
//! `td-headers-attr`, and `th-has-data-cells`.

use std::collections::{BTreeMap, HashSet};

use crate::accname::accessible_name;
use crate::aria::{effective_explicit_role, effective_role, explicit_role};
use crate::ir::{Document, Node, NodeId, normalize_whitespace};
use crate::rule::{Impact, Rule, RuleMeta, Violation};

pub struct EmptyTableHeader;
pub struct ScopeAttrValid;
pub struct TableDuplicateName;
pub struct TableFakeCaption;
pub struct TdHasHeader;
pub struct TdHeadersAttr;
pub struct ThHasDataCells;

const MAX_SPAN: usize = 1000;

fn is_layout_table(table: &Node) -> bool {
    effective_explicit_role(table).is_some_and(|r| !matches!(r, "table" | "grid" | "treegrid"))
}

fn is_semantic_table(doc: &Document, node: &Node) -> bool {
    matches!(
        effective_role(doc, node),
        Some("table" | "grid" | "treegrid")
    )
}

fn enclosing_table(doc: &Document, id: NodeId) -> Option<&Node> {
    for anc_id in doc.ancestors(id) {
        if let Some(anc) = doc.node(anc_id)
            && anc.tag == "table"
        {
            return Some(anc);
        }
    }
    None
}

fn is_table_header_cell(node: &Node) -> bool {
    if let Some(role) = effective_explicit_role(node) {
        return matches!(role, "columnheader" | "rowheader");
    }
    node.tag == "th"
}

fn is_table_data_cell(node: &Node) -> bool {
    if let Some(role) = effective_explicit_role(node) {
        return matches!(role, "cell" | "gridcell");
    }
    node.tag == "td"
}

fn parse_span(node: &Node, attr: &str, aria_attr: &str) -> usize {
    node.attr(attr)
        .or_else(|| node.attr(aria_attr))
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .map_or(1, |n| n.min(MAX_SPAN))
}

/// Collects row elements belonging directly to `table_id` (both native `<tr>`
/// inside `<thead>`/`<tbody>`/`<tfoot>` and ARIA `[role="row"]` inside
/// `[role="rowgroup"]` or transparent wrappers) without descending into nested tables.
fn table_rows(doc: &Document, table_id: NodeId) -> Vec<NodeId> {
    let mut rows = Vec::new();
    collect_table_rows(doc, table_id, &mut rows);
    rows
}

fn collect_table_rows(doc: &Document, container_id: NodeId, rows: &mut Vec<NodeId>) {
    let Some(container) = doc.node(container_id) else {
        return;
    };
    for &cid in &container.children {
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if !child.is_element() {
            continue;
        }
        let role = effective_role(doc, child);
        if role == Some("row") || (child.tag == "tr" && effective_explicit_role(child).is_none()) {
            rows.push(cid);
        } else if matches!(role, Some("rowgroup" | "generic" | "none" | "presentation"))
            || matches!(child.tag.as_str(), "thead" | "tbody" | "tfoot")
        {
            collect_table_rows(doc, cid, rows);
        }
    }
}

fn row_cells(doc: &Document, row_id: NodeId) -> Vec<NodeId> {
    let mut cells = Vec::new();
    collect_row_cells(doc, row_id, &mut cells);
    cells
}

fn collect_row_cells(doc: &Document, container_id: NodeId, cells: &mut Vec<NodeId>) {
    let Some(container) = doc.node(container_id) else {
        return;
    };
    for &cid in &container.children {
        let Some(child) = doc.node(cid) else {
            continue;
        };
        if !child.is_element() {
            continue;
        }
        if is_table_header_cell(child) || is_table_data_cell(child) {
            cells.push(cid);
        } else if matches!(
            effective_role(doc, child),
            None | Some("generic" | "none" | "presentation")
        ) && !matches!(child.tag.as_str(), "table" | "tr")
        {
            collect_row_cells(doc, cid, cells);
        }
    }
}

struct PlacedCell {
    id: NodeId,
    row: usize,
    col: usize,
    rowspan: usize,
    colspan: usize,
    is_header: bool,
    is_data: bool,
}

struct TableGrid {
    num_rows: usize,
    num_cols: usize,
    cells: Vec<PlacedCell>,
}

fn build_table_grid(doc: &Document, table_id: NodeId) -> TableGrid {
    let rows = table_rows(doc, table_id);
    let mut occupied: HashSet<(usize, usize)> = HashSet::new();
    let mut cells = Vec::new();
    let mut num_rows = rows.len();
    let mut num_cols = 0usize;

    for (r_idx, row_id) in rows.into_iter().enumerate() {
        let mut c_idx = 0usize;
        for cell_id in row_cells(doc, row_id) {
            while occupied.contains(&(r_idx, c_idx)) {
                c_idx += 1;
            }
            let Some(cell) = doc.node(cell_id) else {
                continue;
            };
            let rowspan = parse_span(cell, "rowspan", "aria-rowspan");
            let colspan = parse_span(cell, "colspan", "aria-colspan");
            for dr in 0..rowspan {
                for dc in 0..colspan {
                    occupied.insert((r_idx + dr, c_idx + dc));
                }
            }
            num_rows = num_rows.max(r_idx + rowspan);
            num_cols = num_cols.max(c_idx + colspan);
            cells.push(PlacedCell {
                id: cell_id,
                row: r_idx,
                col: c_idx,
                rowspan,
                colspan,
                is_header: is_table_header_cell(cell),
                is_data: is_table_data_cell(cell),
            });
            c_idx += colspan;
        }
    }

    TableGrid {
        num_rows,
        num_cols,
        cells,
    }
}

fn ranges_overlap(start_a: usize, len_a: usize, start_b: usize, len_b: usize) -> bool {
    start_a < start_b + len_b && start_b < start_a + len_a
}

impl EmptyTableHeader {
    pub const META: RuleMeta = RuleMeta {
        id: "empty-table-header",
        impact: Impact::Minor,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.name-role-value", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "Table header text should not be empty",
        description: "Ensure table headers have discernible accessible text.",
        docs_url: "https://www.w3.org/WAI/tutorials/tables/",
    };
}

impl Rule for EmptyTableHeader {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) || !is_table_header_cell(el) {
                continue;
            }
            if enclosing_table(doc, el.id).is_some_and(is_layout_table) {
                continue;
            }
            if accessible_name(doc, el.id).is_empty() {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl ScopeAttrValid {
    pub const META: RuleMeta = RuleMeta {
        id: "scope-attr-valid",
        impact: Impact::Moderate,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.tables", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "scope attribute should be used correctly",
        description: "Use the scope attribute only on <th> elements with value 'row', 'col', 'rowgroup', or 'colgroup'.",
        docs_url: "https://www.w3.org/WAI/tutorials/tables/",
    };
}

impl Rule for ScopeAttrValid {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) {
                continue;
            }
            let Some(scope) = el.attr("scope") else {
                continue;
            };
            if enclosing_table(doc, el.id).is_some_and(is_layout_table) {
                continue;
            }
            if el.tag != "th" {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!(
                        "The scope attribute is only valid on <th>, not <{}>.",
                        el.tag
                    ),
                ));
                continue;
            }
            let val = scope.trim().to_ascii_lowercase();
            if !matches!(val.as_str(), "row" | "col" | "rowgroup" | "colgroup") {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    format!(
                        "Invalid scope='{scope}'; expected 'row', 'col', 'rowgroup', or 'colgroup'."
                    ),
                ));
            }
        }
    }
}

impl TableDuplicateName {
    pub const META: RuleMeta = RuleMeta {
        id: "table-duplicate-name",
        impact: Impact::Minor,
        wcag: "1.3.1 (Best Practice)",
        tags: &["cat.tables", "best-practice", "RGAAv4", "RGAA-5.2.1"],
        enabled_by_default: true,
        needs_review: false,
        help: "Tables should not have the same summary and caption",
        description: "Ensure the <caption> element does not contain the same text as the table summary attribute.",
        docs_url: "https://www.w3.org/WAI/tutorials/tables/caption-summary/",
    };
}

impl Rule for TableDuplicateName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "table" || !doc.is_in_accessibility_tree(el.id) || is_layout_table(el) {
                continue;
            }
            let Some(summary) = el
                .attr("summary")
                .map(normalize_whitespace)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            let caption_text = el.children.iter().find_map(|&cid| {
                let child = doc.node(cid)?;
                if child.tag == "caption" && doc.is_in_accessibility_tree(cid) {
                    let text = accessible_name(doc, cid);
                    (!text.is_empty()).then_some(text)
                } else {
                    None
                }
            });
            if let Some(caption) = caption_text
                && caption.eq_ignore_ascii_case(&summary)
            {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl TableFakeCaption {
    pub const META: RuleMeta = RuleMeta {
        id: "table-fake-caption",
        impact: Impact::Serious,
        wcag: "1.3.1 (A)",
        tags: &[
            "cat.tables",
            "experimental",
            "wcag2a",
            "wcag131",
            "section508",
            "section508.22.g",
            "EN-301-549",
            "EN-9.1.3.1",
            "RGAAv4",
            "RGAA-5.4.1",
        ],
        enabled_by_default: false,
        needs_review: false,
        help: "Data or header cells must not be used to give caption to a data table.",
        description: "Use a <caption> element instead of a single cell spanning all columns in the first row.",
        docs_url: "https://www.w3.org/WAI/tutorials/tables/caption-summary/",
    };
}

impl Rule for TableFakeCaption {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "table" || !doc.is_in_accessibility_tree(el.id) || is_layout_table(el) {
                continue;
            }
            let has_caption = el
                .children
                .iter()
                .any(|&cid| doc.node(cid).is_some_and(|c| c.tag == "caption"));
            if has_caption {
                continue;
            }
            let rows = table_rows(doc, el.id);
            if rows.len() < 2 {
                continue;
            }
            let first_cells = row_cells(doc, rows[0]);
            if first_cells.len() != 1 {
                continue;
            }
            let Some(first_cell) = doc.node(first_cells[0]) else {
                continue;
            };
            let first_colspan = parse_span(first_cell, "colspan", "aria-colspan");
            if first_colspan < 2 {
                continue;
            }
            let max_other_cols = rows[1..]
                .iter()
                .map(|&rid| {
                    row_cells(doc, rid)
                        .into_iter()
                        .filter_map(|cid| doc.node(cid))
                        .map(|c| parse_span(c, "colspan", "aria-colspan"))
                        .sum::<usize>()
                })
                .max()
                .unwrap_or(0);
            if max_other_cols >= 2 && first_colspan >= max_other_cols {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl TdHasHeader {
    pub const META: RuleMeta = RuleMeta {
        id: "td-has-header",
        impact: Impact::Critical,
        wcag: "1.3.1 (A)",
        tags: &[
            "cat.tables",
            "experimental",
            "wcag2a",
            "wcag131",
            "section508",
            "section508.22.g",
            "TTv5",
            "TT14.b",
            "EN-301-549",
            "EN-9.1.3.1",
            "RGAAv4",
            "RGAA-5.7.4",
        ],
        enabled_by_default: false,
        needs_review: false,
        help: "Non-empty <td> elements in larger <table> must have an associated table header",
        description: "Ensure each non-empty data cell in a table of at least 3 rows and 3 columns has an associated header.",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/info-and-relationships.html",
    };
}

impl Rule for TdHasHeader {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "table" || !doc.is_in_accessibility_tree(el.id) || is_layout_table(el) {
                continue;
            }
            let grid = build_table_grid(doc, el.id);
            if grid.num_rows < 3 || grid.num_cols < 3 {
                continue;
            }
            let id_to_cell: BTreeMap<&str, NodeId> = grid
                .cells
                .iter()
                .filter_map(|c| {
                    let node = doc.node(c.id)?;
                    Some((node.id_attr()?, c.id))
                })
                .collect();

            let mut missing_header_td = None;
            for cell in &grid.cells {
                if !cell.is_data || !doc.is_in_accessibility_tree(cell.id) {
                    continue;
                }
                if accessible_name(doc, cell.id).is_empty() {
                    continue;
                }
                let Some(td_node) = doc.node(cell.id) else {
                    continue;
                };
                let has_valid_headers_attr = td_node.attr("headers").is_some_and(|raw| {
                    raw.split_ascii_whitespace()
                        .any(|ref_id| id_to_cell.get(ref_id).is_some_and(|&tid| tid != cell.id))
                });
                if has_valid_headers_attr {
                    continue;
                }
                let has_th = grid.cells.iter().any(|other| {
                    other.is_header
                        && doc.is_in_accessibility_tree(other.id)
                        && !accessible_name(doc, other.id).is_empty()
                        && (ranges_overlap(other.col, other.colspan, cell.col, cell.colspan)
                            || ranges_overlap(other.row, other.rowspan, cell.row, cell.rowspan))
                });
                if !has_th {
                    missing_header_td = Some(cell.id);
                    break;
                }
            }

            if missing_header_td.is_some() {
                out.push(Violation::with_detail(
                    &self.meta(),
                    doc,
                    el.id,
                    "Non-empty <td> in a large data table (>= 3x3) has no associated <th> or headers attribute.",
                ));
            }
        }
    }
}

impl TdHeadersAttr {
    pub const META: RuleMeta = RuleMeta {
        id: "td-headers-attr",
        impact: Impact::Serious,
        wcag: "1.3.1 (A)",
        tags: &[
            "cat.tables",
            "wcag2a",
            "wcag131",
            "section508",
            "section508.22.g",
            "TTv5",
            "TT14.b",
            "EN-301-549",
            "EN-9.1.3.1",
            "RGAAv4",
            "RGAA-5.7.4",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Table cell headers attributes must refer to other <th> elements in the same table",
        description: "Ensure every ID in a table cell's headers attribute references another cell inside the same table.",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/info-and-relationships.html",
    };
}

impl Rule for TdHeadersAttr {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag != "table" || !doc.is_in_accessibility_tree(el.id) || is_layout_table(el) {
                continue;
            }
            let grid = build_table_grid(doc, el.id);
            let header_ids: BTreeMap<&str, NodeId> = grid
                .cells
                .iter()
                .filter(|c| c.is_header)
                .filter_map(|c| {
                    let node = doc.node(c.id)?;
                    Some((node.id_attr()?, c.id))
                })
                .collect();

            let mut table_detail: Option<String> = None;
            'cells: for cell in &grid.cells {
                if !doc.is_in_accessibility_tree(cell.id) {
                    continue;
                }
                let Some(node) = doc.node(cell.id) else {
                    continue;
                };
                let Some(headers_raw) = node.attr("headers") else {
                    continue;
                };
                for ref_id in headers_raw.split_ascii_whitespace() {
                    match header_ids.get(ref_id) {
                        None => {
                            table_detail = Some(format!(
                                "headers attribute references '{ref_id}', which is not a header cell inside the same table."
                            ));
                            break 'cells;
                        }
                        Some(&target_id) if target_id == cell.id => {
                            table_detail = Some(format!(
                                "headers attribute must not reference itself ('{ref_id}')."
                            ));
                            break 'cells;
                        }
                        Some(_) => {}
                    }
                }
            }
            if let Some(detail) = table_detail {
                out.push(Violation::with_detail(&self.meta(), doc, el.id, detail));
            }
        }
    }
}

impl ThHasDataCells {
    pub const META: RuleMeta = RuleMeta {
        id: "th-has-data-cells",
        impact: Impact::Serious,
        wcag: "1.3.1 (A)",
        tags: &[
            "cat.tables",
            "wcag2a",
            "wcag131",
            "section508",
            "section508.22.g",
            "TTv5",
            "TT14.b",
            "EN-301-549",
            "EN-9.1.3.1",
            "RGAAv4",
            "RGAA-5.7.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "Table headers in a data table must refer to data cells",
        description: "Ensure every non-empty table header corresponds to at least one data cell in the table.",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/info-and-relationships.html",
    };
}

impl Rule for ThHasDataCells {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if !doc.is_in_accessibility_tree(el.id) || !is_semantic_table(doc, el) {
                continue;
            }
            let grid = build_table_grid(doc, el.id);
            let rows_with_data: HashSet<usize> = grid
                .cells
                .iter()
                .filter(|c| c.is_data && doc.is_in_accessibility_tree(c.id))
                .map(|c| c.row)
                .collect();

            for header in &grid.cells {
                if !header.is_header || !doc.is_in_accessibility_tree(header.id) {
                    continue;
                }
                if accessible_name(doc, header.id).is_empty() {
                    continue;
                }
                let Some(header_node) = doc.node(header.id) else {
                    continue;
                };
                let header_id_attr = header_node.id_attr();
                let header_kind = classify_header(header_node);

                let has_assigned = grid.cells.iter().any(|cell| {
                    if cell.id == header.id || !doc.is_in_accessibility_tree(cell.id) {
                        return false;
                    }
                    let Some(cell_node) = doc.node(cell.id) else {
                        return false;
                    };
                    // Explicit `headers="..."` overrides automatic position-based association.
                    if let Some(headers_raw) = cell_node
                        .attr("headers")
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                    {
                        return header_id_attr.is_some_and(|hid| {
                            headers_raw.split_ascii_whitespace().any(|t| t == hid)
                        });
                    }
                    // Determine whether `cell` is a valid target for `header`.
                    let is_valid_target = if cell.is_data {
                        true
                    } else if cell.is_header
                        && header_kind != HeaderKind::Row
                        && cell.row != header.row
                    {
                        // A column header may have row-header <th> cells as assigned cells.
                        let cell_kind = classify_header(cell_node);
                        cell_kind == HeaderKind::Row
                            || (cell_kind == HeaderKind::Auto && rows_with_data.contains(&cell.row))
                    } else {
                        false
                    };
                    if !is_valid_target {
                        return false;
                    }
                    match header_kind {
                        HeaderKind::Col => {
                            ranges_overlap(header.col, header.colspan, cell.col, cell.colspan)
                        }
                        HeaderKind::Row => {
                            ranges_overlap(header.row, header.rowspan, cell.row, cell.rowspan)
                        }
                        HeaderKind::Auto => {
                            if cell.is_header {
                                ranges_overlap(header.col, header.colspan, cell.col, cell.colspan)
                            } else {
                                ranges_overlap(header.col, header.colspan, cell.col, cell.colspan)
                                    || ranges_overlap(
                                        header.row,
                                        header.rowspan,
                                        cell.row,
                                        cell.rowspan,
                                    )
                            }
                        }
                    }
                });

                if !has_assigned {
                    out.push(Violation::from_node(&self.meta(), doc, header.id));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderKind {
    Col,
    Row,
    Auto,
}

fn classify_header(node: &Node) -> HeaderKind {
    if let Some(role) = explicit_role(node) {
        if role == "columnheader" {
            return HeaderKind::Col;
        }
        if role == "rowheader" {
            return HeaderKind::Row;
        }
    }
    match node
        .attr("scope")
        .map(|s| s.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("col" | "colgroup") => HeaderKind::Col,
        Some("row" | "rowgroup") => HeaderKind::Row,
        _ => HeaderKind::Auto,
    }
}
