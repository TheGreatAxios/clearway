//! WAI-ARIA accessible name rules: `aria-command-name`, `aria-dialog-name`,
//! `aria-input-field-name`, `aria-meter-name`, `aria-progressbar-name`,
//! `aria-tab-name`, `aria-toggle-field-name`, `aria-tooltip-name`, and
//! `aria-treeitem-name`.

use crate::accname::accessible_name;
use crate::aria::explicit_role;
use crate::ir::Document;
use crate::rule::{Impact, Rule, RuleMeta, Violation};

pub struct AriaCommandName;
pub struct AriaDialogName;
pub struct AriaInputFieldName;
pub struct AriaMeterName;
pub struct AriaProgressbarName;
pub struct AriaTabName;
pub struct AriaToggleFieldName;
pub struct AriaTooltipName;
pub struct AriaTreeitemName;

fn check_role_names(
    meta: &RuleMeta,
    doc: &Document,
    roles: &[&str],
    skip_tags: &[&str],
    out: &mut Vec<Violation>,
) {
    for el in doc.elements() {
        if !doc.is_in_accessibility_tree(el.id) {
            continue;
        }
        if skip_tags.contains(&el.tag.as_str()) {
            continue;
        }
        let Some(role) = explicit_role(el) else {
            continue;
        };
        if !roles.contains(&role) {
            continue;
        }
        if accessible_name(doc, el.id).is_empty() {
            out.push(Violation::from_node(meta, doc, el.id));
        }
    }
}

impl AriaCommandName {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-command-name",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag412",
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
        help: "ARIA commands must have an accessible name",
        description: "Ensure every ARIA button, link and menuitem has an accessible name",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/name-role-value.html",
    };
}

impl Rule for AriaCommandName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_role_names(&Self::META, doc, &["button", "link", "menuitem"], &[], out);
    }
}

impl AriaDialogName {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-dialog-name",
        impact: Impact::Serious,
        wcag: "Best Practice",
        tags: &["cat.aria", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "ARIA dialog and alertdialog nodes should have an accessible name",
        description: "Ensure every ARIA dialog and alertdialog node has an accessible name",
        docs_url: "https://www.w3.org/TR/wai-aria-1.2/#dialog",
    };
}

impl Rule for AriaDialogName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_role_names(&Self::META, doc, &["dialog", "alertdialog"], &[], out);
    }
}

impl AriaInputFieldName {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-input-field-name",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag412",
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
        help: "ARIA input fields must have an accessible name",
        description: "Ensure every ARIA input field has an accessible name",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/name-role-value.html",
    };
}

impl Rule for AriaInputFieldName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_role_names(
            &Self::META,
            doc,
            &[
                "combobox",
                "listbox",
                "searchbox",
                "slider",
                "spinbutton",
                "textbox",
            ],
            &["input", "select", "textarea"],
            out,
        );
    }
}

impl AriaMeterName {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-meter-name",
        impact: Impact::Serious,
        wcag: "1.1.1 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag111",
            "EN-301-549",
            "EN-9.1.1.1",
            "RGAAv4",
            "RGAA-11.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "ARIA meter nodes must have an accessible name",
        description: "Ensure every ARIA meter node has an accessible name",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/non-text-content.html",
    };
}

impl Rule for AriaMeterName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_role_names(&Self::META, doc, &["meter"], &[], out);
    }
}

impl AriaProgressbarName {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-progressbar-name",
        impact: Impact::Serious,
        wcag: "1.1.1 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag111",
            "EN-301-549",
            "EN-9.1.1.1",
            "RGAAv4",
            "RGAA-11.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "ARIA progressbar nodes must have an accessible name",
        description: "Ensure every ARIA progressbar node has an accessible name",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/non-text-content.html",
    };
}

impl Rule for AriaProgressbarName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_role_names(&Self::META, doc, &["progressbar"], &[], out);
    }
}

impl AriaTabName {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-tab-name",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag412",
            "TTv5",
            "TT5.c",
            "EN-301-549",
            "EN-9.4.1.2",
            "ACT",
            "RGAAv4",
            "RGAA-7.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "ARIA tab nodes must have an accessible name",
        description: "Ensure every ARIA tab node has an accessible name",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/name-role-value.html",
    };
}

impl Rule for AriaTabName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_role_names(&Self::META, doc, &["tab"], &[], out);
    }
}

impl AriaToggleFieldName {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-toggle-field-name",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &[
            "cat.aria",
            "wcag2a",
            "wcag412",
            "TTv5",
            "TT5.c",
            "EN-301-549",
            "EN-9.4.1.2",
            "ACT",
            "RGAAv4",
            "RGAA-7.1.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "ARIA toggle fields must have an accessible name",
        description: "Ensure every ARIA toggle field has an accessible name",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/name-role-value.html",
    };
}

impl Rule for AriaToggleFieldName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_role_names(
            &Self::META,
            doc,
            &[
                "checkbox",
                "menu",
                "menuitemcheckbox",
                "menuitemradio",
                "option",
                "radio",
                "radiogroup",
                "switch",
            ],
            &["input"],
            out,
        );
    }
}

impl AriaTooltipName {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-tooltip-name",
        impact: Impact::Serious,
        wcag: "4.1.2 (A)",
        tags: &["cat.aria", "wcag2a", "wcag412", "EN-301-549", "EN-9.4.1.2"],
        enabled_by_default: true,
        needs_review: false,
        help: "ARIA tooltip nodes must have an accessible name",
        description: "Ensure every ARIA tooltip node has an accessible name",
        docs_url: "https://www.w3.org/WAI/WCAG22/Understanding/name-role-value.html",
    };
}

impl Rule for AriaTooltipName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_role_names(&Self::META, doc, &["tooltip"], &[], out);
    }
}

impl AriaTreeitemName {
    pub const META: RuleMeta = RuleMeta {
        id: "aria-treeitem-name",
        impact: Impact::Serious,
        wcag: "Best Practice",
        tags: &["cat.aria", "best-practice"],
        enabled_by_default: true,
        needs_review: false,
        help: "ARIA treeitem nodes should have an accessible name",
        description: "Ensure every ARIA treeitem node has an accessible name",
        docs_url: "https://www.w3.org/TR/wai-aria-1.2/#treeitem",
    };
}

impl Rule for AriaTreeitemName {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        check_role_names(&Self::META, doc, &["treeitem"], &[], out);
    }
}
