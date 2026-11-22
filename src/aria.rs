//! WAI-ARIA 1.2 roles, attributes, required properties, value validation, and HTML-AAM implicit role mappings.

use std::collections::HashSet;

use crate::ir::{Document, Node, NodeId};

/// Sorted list of valid concrete WAI-ARIA 1.2 roles (abstract roles excluded).
pub const VALID_ROLES: &[&str] = &[
    "alert",
    "alertdialog",
    "application",
    "article",
    "banner",
    "blockquote",
    "button",
    "caption",
    "cell",
    "checkbox",
    "code",
    "columnheader",
    "combobox",
    "comment",
    "complementary",
    "contentinfo",
    "definition",
    "deletion",
    "dialog",
    "directory",
    "doc-abstract",
    "doc-acknowledgments",
    "doc-afterword",
    "doc-appendix",
    "doc-backlink",
    "doc-biblioentry",
    "doc-bibliography",
    "doc-biblioref",
    "doc-chapter",
    "doc-colophon",
    "doc-conclusion",
    "doc-cover",
    "doc-credit",
    "doc-credits",
    "doc-dedication",
    "doc-endnote",
    "doc-endnotes",
    "doc-epigraph",
    "doc-epilogue",
    "doc-errata",
    "doc-example",
    "doc-footnote",
    "doc-foreword",
    "doc-glossary",
    "doc-glossref",
    "doc-index",
    "doc-introduction",
    "doc-noteref",
    "doc-notice",
    "doc-pagebreak",
    "doc-pagefooter",
    "doc-pageheader",
    "doc-pagelist",
    "doc-part",
    "doc-preface",
    "doc-prologue",
    "doc-pullquote",
    "doc-qna",
    "doc-subtitle",
    "doc-tip",
    "doc-toc",
    "document",
    "emphasis",
    "feed",
    "figure",
    "form",
    "generic",
    "graphics-document",
    "graphics-object",
    "graphics-symbol",
    "grid",
    "gridcell",
    "group",
    "heading",
    "image",
    "img",
    "insertion",
    "link",
    "list",
    "listbox",
    "listitem",
    "log",
    "main",
    "mark",
    "marquee",
    "math",
    "menu",
    "menubar",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "meter",
    "navigation",
    "none",
    "note",
    "option",
    "paragraph",
    "presentation",
    "progressbar",
    "radio",
    "radiogroup",
    "region",
    "row",
    "rowgroup",
    "rowheader",
    "scrollbar",
    "search",
    "searchbox",
    "sectionfooter",
    "sectionheader",
    "separator",
    "slider",
    "spinbutton",
    "status",
    "strong",
    "subscript",
    "suggestion",
    "superscript",
    "switch",
    "tab",
    "table",
    "tablist",
    "tabpanel",
    "term",
    "text",
    "textbox",
    "time",
    "timer",
    "toolbar",
    "tooltip",
    "tree",
    "treegrid",
    "treeitem",
];

/// Sorted list of valid WAI-ARIA 1.2/1.3 `aria-*` attributes.
pub const VALID_ARIA_ATTRS: &[&str] = &[
    "aria-actions",
    "aria-activedescendant",
    "aria-atomic",
    "aria-autocomplete",
    "aria-braillelabel",
    "aria-brailleroledescription",
    "aria-busy",
    "aria-checked",
    "aria-colcount",
    "aria-colindex",
    "aria-colindextext",
    "aria-colspan",
    "aria-controls",
    "aria-current",
    "aria-describedby",
    "aria-description",
    "aria-details",
    "aria-disabled",
    "aria-dropeffect",
    "aria-errormessage",
    "aria-expanded",
    "aria-flowto",
    "aria-grabbed",
    "aria-haspopup",
    "aria-hidden",
    "aria-invalid",
    "aria-keyshortcuts",
    "aria-label",
    "aria-labelledby",
    "aria-level",
    "aria-live",
    "aria-modal",
    "aria-multiline",
    "aria-multiselectable",
    "aria-orientation",
    "aria-owns",
    "aria-placeholder",
    "aria-posinset",
    "aria-pressed",
    "aria-readonly",
    "aria-relevant",
    "aria-required",
    "aria-roledescription",
    "aria-rowcount",
    "aria-rowindex",
    "aria-rowindextext",
    "aria-rowspan",
    "aria-selected",
    "aria-setsize",
    "aria-sort",
    "aria-valuemax",
    "aria-valuemin",
    "aria-valuenow",
    "aria-valuetext",
];

/// ARIA attributes that reference element IDs (IDREF / IDREFS).
pub const ARIA_IDREF_ATTRS: &[&str] = &[
    "aria-actions",
    "aria-activedescendant",
    "aria-controls",
    "aria-describedby",
    "aria-details",
    "aria-errormessage",
    "aria-flowto",
    "aria-labelledby",
    "aria-owns",
];

/// Sorted list of WAI-ARIA 1.2 abstract roles (not valid for authoring).
pub const ABSTRACT_ROLES: &[&str] = &[
    "command",
    "composite",
    "input",
    "landmark",
    "range",
    "roletype",
    "section",
    "sectionhead",
    "select",
    "structure",
    "widget",
    "window",
];

/// Sorted list of WAI-ARIA 1.2/1.3 global states and properties allowed on any role
/// (unless explicitly prohibited by that role).
pub const GLOBAL_ARIA_ATTRS: &[&str] = &[
    "aria-actions",
    "aria-atomic",
    "aria-braillelabel",
    "aria-brailleroledescription",
    "aria-busy",
    "aria-controls",
    "aria-current",
    "aria-describedby",
    "aria-description",
    "aria-details",
    "aria-disabled",
    "aria-dropeffect",
    "aria-errormessage",
    "aria-flowto",
    "aria-grabbed",
    "aria-haspopup",
    "aria-hidden",
    "aria-invalid",
    "aria-keyshortcuts",
    "aria-label",
    "aria-labelledby",
    "aria-live",
    "aria-owns",
    "aria-relevant",
    "aria-roledescription",
];

/// Sorted list of valid ISO 639-1 two-letter primary language subtags (BCP 47).
const ISO_639_1_LANGS: &[&str] = &[
    "aa", "ab", "ae", "af", "ak", "am", "an", "ar", "as", "av", "ay", "az", "ba", "be", "bg", "bh",
    "bi", "bm", "bn", "bo", "br", "bs", "ca", "ce", "ch", "co", "cr", "cs", "cu", "cv", "cy", "da",
    "de", "dv", "dz", "ee", "el", "en", "eo", "es", "et", "eu", "fa", "ff", "fi", "fj", "fo", "fr",
    "fy", "ga", "gd", "gl", "gn", "gu", "gv", "ha", "he", "hi", "ho", "hr", "ht", "hu", "hy", "hz",
    "ia", "id", "ie", "ig", "ii", "ik", "in", "io", "is", "it", "iu", "iw", "ja", "ji", "jv", "jw",
    "ka", "kg", "ki", "kj", "kk", "kl", "km", "kn", "ko", "kr", "ks", "ku", "kv", "kw", "ky", "la",
    "lb", "lg", "li", "ln", "lo", "lt", "lu", "lv", "mg", "mh", "mi", "mk", "ml", "mn", "mo", "mr",
    "ms", "mt", "my", "na", "nb", "nd", "ne", "ng", "nl", "nn", "no", "nr", "nv", "ny", "oc", "oj",
    "om", "or", "os", "pa", "pi", "pl", "ps", "pt", "qu", "rm", "rn", "ro", "ru", "rw", "sa", "sc",
    "sd", "se", "sg", "sh", "si", "sk", "sl", "sm", "sn", "so", "sq", "sr", "ss", "st", "su", "sv",
    "sw", "ta", "te", "tg", "th", "ti", "tk", "tl", "tn", "to", "tr", "ts", "tt", "tw", "ty", "ug",
    "uk", "ur", "uz", "ve", "vi", "vo", "wa", "wo", "xh", "yi", "yo", "za", "zh", "zu",
];

/// Sorted list of ISO 639-2 (T and B) three-letter codes that have an ISO 639-1
/// two-letter equivalent and are therefore invalid as BCP 47 primary subtags
/// per RFC 5646 Section 2.2.1.
const ISO_639_2_WITH_1_EQUIV: &[&str] = &[
    "aar", "abk", "afr", "aka", "alb", "amh", "ara", "arg", "arm", "asm", "ava", "ave", "aym",
    "aze", "bak", "bam", "baq", "bel", "ben", "bih", "bis", "bod", "bos", "bre", "bul", "bur",
    "cat", "ces", "cha", "che", "chi", "chu", "chv", "cor", "cos", "cre", "cym", "cze", "dan",
    "deu", "div", "dut", "dzo", "ell", "eng", "epo", "est", "eus", "ewe", "fao", "fas", "fij",
    "fin", "fra", "fre", "fry", "ful", "geo", "ger", "gla", "gle", "glg", "glv", "gre", "grn",
    "guj", "hat", "hau", "heb", "her", "hin", "hmo", "hrv", "hun", "hye", "ibo", "ice", "ido",
    "iii", "iku", "ile", "ina", "ind", "ipk", "isl", "ita", "jav", "jpn", "kal", "kan", "kas",
    "kat", "kau", "kaz", "khm", "kik", "kin", "kir", "kom", "kon", "kor", "kua", "kur", "lao",
    "lat", "lav", "lim", "lin", "lit", "ltz", "lub", "lug", "mac", "mah", "mal", "mao", "mar",
    "may", "mkd", "mlg", "mlt", "mon", "mri", "msa", "mya", "nau", "nav", "nbl", "nde", "ndo",
    "nep", "nld", "nno", "nob", "nor", "nya", "oci", "oji", "ori", "orm", "oss", "pan", "per",
    "pli", "pol", "por", "pus", "que", "roh", "ron", "rum", "run", "rus", "sag", "san", "sin",
    "slk", "slo", "slv", "sme", "smo", "sna", "snd", "som", "sot", "spa", "sqi", "srd", "srp",
    "ssw", "sun", "swa", "swe", "tah", "tam", "tat", "tel", "tgk", "tgl", "tha", "tib", "tir",
    "ton", "tsn", "tso", "tuk", "tur", "twi", "uig", "ukr", "urd", "uzb", "ven", "vie", "vol",
    "wel", "wln", "wol", "xho", "yid", "yor", "zha", "zho", "zul",
];

/// Returns `true` if `role` is a valid concrete WAI-ARIA 1.2 / DPUB-ARIA role.
pub fn is_valid_role(role: &str) -> bool {
    VALID_ROLES.binary_search(&role).is_ok()
}

/// Returns `true` if `role` is a deprecated WAI-ARIA 1.2 / DPUB-ARIA 1.1 role.
pub fn is_deprecated_role(role: &str) -> bool {
    matches!(role, "directory" | "doc-biblioentry" | "doc-endnote")
}

/// Returns `true` if `role` is a WAI-ARIA 1.2 abstract role.
pub fn is_abstract_role(role: &str) -> bool {
    ABSTRACT_ROLES.binary_search(&role).is_ok()
}

/// Returns `true` if `attr` is a valid WAI-ARIA 1.2 `aria-*` attribute name.
pub fn is_valid_aria_attr(attr: &str) -> bool {
    VALID_ARIA_ATTRS.binary_search(&attr).is_ok()
}

/// Returns `true` if `attr` is a WAI-ARIA 1.2/1.3 global state or property.
pub fn is_global_aria_attr(attr: &str) -> bool {
    GLOBAL_ARIA_ATTRS.binary_search(&attr).is_ok()
}

/// Returns `true` if assistive technologies require lowercase token values for `attr`
/// (e.g. NVDA/Firefox, JAWS/Chrome, and VoiceOver/Safari ignore or misannounce uppercase values).
pub fn is_case_sensitive_aria_attr(attr: &str) -> bool {
    matches!(
        attr,
        "aria-busy"
            | "aria-checked"
            | "aria-current"
            | "aria-multiline"
            | "aria-pressed"
            | "aria-readonly"
            | "aria-relevant"
            | "aria-required"
            | "aria-sort"
    )
}

/// Returns WAI-ARIA 1.2/1.3 attributes prohibited on `role`.
pub fn prohibited_attrs_for_role(role: &str) -> &'static [&'static str] {
    match role {
        "generic" => &[
            "aria-braillelabel",
            "aria-brailleroledescription",
            "aria-label",
            "aria-labelledby",
            "aria-roledescription",
        ],
        "none" | "presentation" => &[
            "aria-braillelabel",
            "aria-brailleroledescription",
            "aria-label",
            "aria-labelledby",
        ],
        "code" | "mark" => &[
            "aria-actions",
            "aria-braillelabel",
            "aria-label",
            "aria-labelledby",
        ],
        "caption" | "deletion" | "emphasis" | "insertion" | "paragraph" | "strong"
        | "subscript" | "superscript" | "suggestion" | "time" => {
            &["aria-braillelabel", "aria-label", "aria-labelledby"]
        }
        _ => &[],
    }
}

/// Returns role-specific (non-global) WAI-ARIA 1.2 attributes allowed on `role`.
///
/// Global attributes in [`GLOBAL_ARIA_ATTRS`] are also allowed unless listed in
/// [`prohibited_attrs_for_role`]. Use [`is_attr_allowed_for_role`] to check both.
#[allow(clippy::too_many_lines)]
pub fn allowed_attrs_for_role(role: &str) -> &'static [&'static str] {
    match role {
        "alertdialog" | "dialog" => &["aria-modal"],
        "application" => &[
            "aria-activedescendant",
            "aria-disabled",
            "aria-errormessage",
            "aria-expanded",
            "aria-haspopup",
            "aria-invalid",
        ],
        "article" => &["aria-posinset", "aria-setsize"],
        "button" => &[
            "aria-disabled",
            "aria-expanded",
            "aria-haspopup",
            "aria-pressed",
        ],
        "cell" => &[
            "aria-colindex",
            "aria-colindextext",
            "aria-colspan",
            "aria-rowindex",
            "aria-rowindextext",
            "aria-rowspan",
        ],
        "checkbox" => &[
            "aria-checked",
            "aria-disabled",
            "aria-errormessage",
            "aria-expanded",
            "aria-invalid",
            "aria-readonly",
            "aria-required",
        ],
        "columnheader" | "rowheader" => &[
            "aria-colindex",
            "aria-colindextext",
            "aria-colspan",
            "aria-disabled",
            "aria-errormessage",
            "aria-expanded",
            "aria-haspopup",
            "aria-invalid",
            "aria-readonly",
            "aria-required",
            "aria-rowindex",
            "aria-rowindextext",
            "aria-rowspan",
            "aria-selected",
            "aria-sort",
        ],
        "combobox" => &[
            "aria-activedescendant",
            "aria-autocomplete",
            "aria-disabled",
            "aria-errormessage",
            "aria-expanded",
            "aria-haspopup",
            "aria-invalid",
            "aria-orientation",
            "aria-readonly",
            "aria-required",
        ],
        "comment" => &["aria-level", "aria-posinset", "aria-setsize"],
        "grid" => &[
            "aria-activedescendant",
            "aria-colcount",
            "aria-disabled",
            "aria-level",
            "aria-multiselectable",
            "aria-readonly",
            "aria-rowcount",
        ],
        "gridcell" => &[
            "aria-colindex",
            "aria-colindextext",
            "aria-colspan",
            "aria-disabled",
            "aria-errormessage",
            "aria-expanded",
            "aria-haspopup",
            "aria-invalid",
            "aria-readonly",
            "aria-required",
            "aria-rowindex",
            "aria-rowindextext",
            "aria-rowspan",
            "aria-selected",
        ],
        "group" => &["aria-activedescendant", "aria-disabled"],
        "heading" => &["aria-level"],
        "link" | "doc-backlink" | "doc-biblioref" | "doc-glossref" | "doc-noteref" => {
            &["aria-disabled", "aria-expanded", "aria-haspopup"]
        }
        "listbox" => &[
            "aria-activedescendant",
            "aria-disabled",
            "aria-errormessage",
            "aria-expanded",
            "aria-invalid",
            "aria-multiselectable",
            "aria-orientation",
            "aria-readonly",
            "aria-required",
        ],
        "listitem" | "doc-biblioentry" | "doc-endnote" => {
            &["aria-level", "aria-posinset", "aria-setsize"]
        }
        "menu" | "menubar" | "toolbar" => {
            &["aria-activedescendant", "aria-disabled", "aria-orientation"]
        }
        "menuitem" => &[
            "aria-disabled",
            "aria-expanded",
            "aria-haspopup",
            "aria-posinset",
            "aria-setsize",
        ],
        "menuitemcheckbox" | "menuitemradio" => &[
            "aria-checked",
            "aria-disabled",
            "aria-expanded",
            "aria-haspopup",
            "aria-posinset",
            "aria-setsize",
        ],
        "meter" | "progressbar" => &[
            "aria-valuemax",
            "aria-valuemin",
            "aria-valuenow",
            "aria-valuetext",
        ],
        "option" => &[
            "aria-checked",
            "aria-disabled",
            "aria-posinset",
            "aria-selected",
            "aria-setsize",
        ],
        "radio" => &[
            "aria-checked",
            "aria-disabled",
            "aria-posinset",
            "aria-required",
            "aria-setsize",
        ],
        "radiogroup" => &[
            "aria-activedescendant",
            "aria-disabled",
            "aria-errormessage",
            "aria-invalid",
            "aria-orientation",
            "aria-readonly",
            "aria-required",
        ],
        "row" => &[
            "aria-activedescendant",
            "aria-colindex",
            "aria-colindextext",
            "aria-disabled",
            "aria-expanded",
            "aria-level",
            "aria-posinset",
            "aria-rowindex",
            "aria-rowindextext",
            "aria-selected",
            "aria-setsize",
        ],
        "scrollbar" | "separator" | "doc-pagebreak" => &[
            "aria-disabled",
            "aria-orientation",
            "aria-valuemax",
            "aria-valuemin",
            "aria-valuenow",
            "aria-valuetext",
        ],
        "searchbox" | "textbox" => &[
            "aria-activedescendant",
            "aria-autocomplete",
            "aria-disabled",
            "aria-errormessage",
            "aria-haspopup",
            "aria-invalid",
            "aria-multiline",
            "aria-placeholder",
            "aria-readonly",
            "aria-required",
        ],
        "slider" => &[
            "aria-disabled",
            "aria-errormessage",
            "aria-haspopup",
            "aria-invalid",
            "aria-orientation",
            "aria-readonly",
            "aria-required",
            "aria-valuemax",
            "aria-valuemin",
            "aria-valuenow",
            "aria-valuetext",
        ],
        "spinbutton" => &[
            "aria-disabled",
            "aria-errormessage",
            "aria-invalid",
            "aria-readonly",
            "aria-required",
            "aria-valuemax",
            "aria-valuemin",
            "aria-valuenow",
            "aria-valuetext",
        ],
        "switch" => &[
            "aria-checked",
            "aria-disabled",
            "aria-errormessage",
            "aria-invalid",
            "aria-readonly",
            "aria-required",
        ],
        "tab" => &[
            "aria-disabled",
            "aria-expanded",
            "aria-haspopup",
            "aria-posinset",
            "aria-selected",
            "aria-setsize",
        ],
        "table" => &["aria-colcount", "aria-rowcount"],
        "tablist" => &[
            "aria-activedescendant",
            "aria-disabled",
            "aria-level",
            "aria-multiselectable",
            "aria-orientation",
        ],
        "tree" => &[
            "aria-activedescendant",
            "aria-disabled",
            "aria-errormessage",
            "aria-invalid",
            "aria-multiselectable",
            "aria-orientation",
            "aria-required",
        ],
        "treegrid" => &[
            "aria-activedescendant",
            "aria-colcount",
            "aria-disabled",
            "aria-errormessage",
            "aria-invalid",
            "aria-level",
            "aria-multiselectable",
            "aria-orientation",
            "aria-readonly",
            "aria-required",
            "aria-rowcount",
        ],
        "treeitem" => &[
            "aria-checked",
            "aria-disabled",
            "aria-expanded",
            "aria-haspopup",
            "aria-level",
            "aria-posinset",
            "aria-selected",
            "aria-setsize",
        ],
        _ => &[],
    }
}

/// Returns `true` if `attr` is permitted on `role` (including global ARIA attributes
/// unless prohibited by `role`).
pub fn is_attr_allowed_for_role(role: Option<&str>, attr: &str) -> bool {
    if !is_valid_aria_attr(attr) {
        return false;
    }
    if let Some(r) = role
        && prohibited_attrs_for_role(r).contains(&attr)
    {
        return false;
    }
    if attr == "aria-expanded"
        && role.is_some_and(|r| !matches!(r, "generic" | "none" | "presentation"))
    {
        return true;
    }
    is_global_aria_attr(attr) || role.is_some_and(|r| allowed_attrs_for_role(r).contains(&attr))
}

/// Returns required owned child roles for `role` per WAI-ARIA 1.2/1.3.
pub fn required_children_for_role(role: &str) -> &'static [&'static str] {
    match role {
        "feed" => &["article"],
        "grid" | "table" | "treegrid" => &["row", "rowgroup"],
        "list" => &["listitem"],
        "listbox" => &["option", "group"],
        "menu" | "menubar" => &["menuitem", "menuitemcheckbox", "menuitemradio", "group"],
        "radiogroup" => &["radio"],
        "row" => &["cell", "columnheader", "gridcell", "rowheader"],
        "rowgroup" => &["row"],
        "suggestion" => &["insertion", "deletion"],
        "tablist" => &["tab"],
        "tree" => &["treeitem", "group"],
        _ => &[],
    }
}

/// Returns required context (parent) roles for `role` per WAI-ARIA 1.2 / DPUB-ARIA.
pub fn required_parent_for_role(role: &str) -> &'static [&'static str] {
    match role {
        "caption" => &["figure", "grid", "table", "treegrid"],
        "cell" | "columnheader" | "gridcell" | "rowheader" => &["row"],
        "doc-biblioentry" => &["doc-bibliography"],
        "doc-endnote" => &["doc-endnotes"],
        "listitem" => &["list"],
        "menuitem" | "menuitemcheckbox" | "menuitemradio" => &["menu", "menubar", "group"],
        "option" => &["listbox", "group"],
        "row" => &["grid", "rowgroup", "table", "treegrid"],
        "rowgroup" => &["grid", "table", "treegrid"],
        "tab" => &["tablist"],
        "treeitem" => &["tree", "group"],
        _ => &[],
    }
}

/// Validates an ARIA attribute's value per WAI-ARIA 1.2/1.3 value types (boolean,
/// tristate, integer, number, token, token list, IDREF, IDREFS, and string).
///
/// Per WAI-ARIA and ACT `6a7281`, enumerated token values are case-sensitive lowercase.
#[allow(clippy::implicit_hasher, clippy::too_many_lines)]
pub fn validate_aria_attr_value(attr: &str, value: &str, all_ids: &HashSet<&str>) -> bool {
    let trimmed = value.trim();
    match attr {
        // Strict boolean (case-sensitive lowercase per WAI-ARIA)
        "aria-atomic"
        | "aria-busy"
        | "aria-disabled"
        | "aria-hidden"
        | "aria-modal"
        | "aria-multiline"
        | "aria-multiselectable"
        | "aria-readonly"
        | "aria-required" => matches!(trimmed, "true" | "false"),
        // Tristate
        "aria-checked" | "aria-pressed" => {
            matches!(trimmed, "true" | "false" | "mixed" | "undefined")
        }
        // Boolean or undefined
        "aria-expanded" | "aria-grabbed" | "aria-selected" => {
            matches!(trimmed, "true" | "false" | "undefined")
        }
        // Positive integer (>= 1)
        "aria-colindex" | "aria-colspan" | "aria-level" | "aria-posinset" | "aria-rowindex"
        | "aria-rowspan" => trimmed.parse::<i64>().is_ok_and(|n| n >= 1),
        // Integer (>= -1)
        "aria-colcount" | "aria-rowcount" | "aria-setsize" => {
            trimmed.parse::<i64>().is_ok_and(|n| n >= -1)
        }
        // Floating-point number
        "aria-valuemax" | "aria-valuemin" | "aria-valuenow" => {
            trimmed.parse::<f64>().is_ok_and(f64::is_finite)
        }
        // Single token enumerations (case-sensitive lowercase per WAI-ARIA)
        "aria-autocomplete" => matches!(trimmed, "inline" | "list" | "both" | "none"),
        "aria-current" => matches!(
            trimmed,
            "page" | "step" | "location" | "date" | "time" | "true" | "false"
        ),
        "aria-haspopup" => matches!(
            trimmed,
            "false" | "true" | "menu" | "listbox" | "tree" | "grid" | "dialog"
        ),
        "aria-invalid" => matches!(trimmed, "grammar" | "false" | "spelling" | "true"),
        "aria-live" => matches!(trimmed, "off" | "polite" | "assertive"),
        "aria-orientation" => matches!(trimmed, "horizontal" | "vertical" | "undefined"),
        "aria-sort" => matches!(trimmed, "ascending" | "descending" | "none" | "other"),
        // Token lists
        "aria-dropeffect" => {
            let mut tokens = trimmed.split_ascii_whitespace().peekable();
            tokens.peek().is_some()
                && tokens.all(|tok| {
                    matches!(tok, "copy" | "execute" | "link" | "move" | "none" | "popup")
                })
        }
        "aria-relevant" => {
            let mut tokens = trimmed.split_ascii_whitespace().peekable();
            tokens.peek().is_some()
                && tokens.all(|tok| matches!(tok, "additions" | "all" | "removals" | "text"))
        }
        // Single IDREF
        "aria-activedescendant" | "aria-errormessage" => {
            !trimmed.is_empty()
                && !trimmed.bytes().any(|b| b.is_ascii_whitespace())
                && all_ids.contains(trimmed)
        }
        // Space-separated IDREFS: valid when at least one token matches an existing ID
        "aria-actions" | "aria-controls" | "aria-describedby" | "aria-details" | "aria-flowto"
        | "aria-labelledby" | "aria-owns" => trimmed
            .split_ascii_whitespace()
            .any(|tok| all_ids.contains(tok)),
        // Free-form string attributes
        "aria-braillelabel"
        | "aria-brailleroledescription"
        | "aria-colindextext"
        | "aria-description"
        | "aria-keyshortcuts"
        | "aria-label"
        | "aria-placeholder"
        | "aria-roledescription"
        | "aria-rowindextext"
        | "aria-valuetext" => true,
        _ => false,
    }
}

/// Validates a BCP 47 language tag (e.g. `"en"`, `"en-US"`, `"es-419"`, `"fil"`),
/// verifying syntax, ISO 639-1 registration for 2-letter primary subtags, and
/// RFC 5646 Section 2.2.1 (rejecting 3-letter ISO 639-2 codes when a 2-letter
/// ISO 639-1 equivalent exists).
pub fn is_valid_bcp47_primary(lang: &str) -> bool {
    let trimmed = lang.trim();
    let mut parts = trimmed.split('-');
    let Some(primary) = parts.next() else {
        return false;
    };
    let primary_lower = primary.to_ascii_lowercase();
    let valid_primary = match primary_lower.len() {
        2 => ISO_639_1_LANGS
            .binary_search(&primary_lower.as_str())
            .is_ok(),
        3 => {
            primary_lower.bytes().all(|b| b.is_ascii_alphabetic())
                && ISO_639_2_WITH_1_EQUIV
                    .binary_search(&primary_lower.as_str())
                    .is_err()
        }
        _ => false,
    };
    if !valid_primary {
        return false;
    }
    for subtag in parts {
        if subtag.is_empty()
            || subtag.len() > 8
            || !subtag.bytes().all(|b| b.is_ascii_alphanumeric())
        {
            return false;
        }
    }
    true
}

/// Returns required WAI-ARIA 1.2 attributes for an explicit `role`.
pub fn required_attrs_for_role(role: &str) -> &'static [&'static str] {
    match role {
        "checkbox" | "menuitemcheckbox" | "menuitemradio" | "radio" | "switch" => &["aria-checked"],
        "combobox" => &["aria-controls", "aria-expanded"],
        "heading" => &["aria-level"],
        "meter" | "slider" => &["aria-valuenow"],
        "scrollbar" => &["aria-controls", "aria-valuenow"],
        _ => &[],
    }
}

/// Returns `true` if the native HTML element already satisfies `required_attr`
/// without needing an explicit ARIA attribute.
pub fn native_element_satisfies_attr(node: &Node, required_attr: &str) -> bool {
    match required_attr {
        "aria-checked" => {
            node.tag == "input"
                && matches!(
                    node.attr("type").map(str::to_ascii_lowercase).as_deref(),
                    Some("checkbox" | "radio")
                )
        }
        "aria-level" => matches!(node.tag.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6"),
        "aria-valuenow" => {
            node.tag == "meter"
                || (node.tag == "input"
                    && node
                        .attr("type")
                        .is_some_and(|t| t.eq_ignore_ascii_case("range")))
        }
        _ => false,
    }
}

/// Returns the first valid explicit role token from the element's `role` attribute.
pub fn explicit_role(node: &Node) -> Option<&str> {
    let raw = node.attr("role")?;
    raw.split_ascii_whitespace()
        .find(|token| is_valid_role(token))
}

/// Returns the HTML-AAM implicit role for an HTML element.
pub fn implicit_role(doc: &Document, node: &Node) -> Option<&'static str> {
    match node.tag.as_str() {
        "a" | "area" => {
            if node.has_attr("href") {
                Some("link")
            } else {
                Some("generic")
            }
        }
        "abbr" | "b" | "bdi" | "bdo" | "data" | "dfn" | "div" | "hgroup" | "i" | "kbd" | "pre"
        | "q" | "samp" | "small" | "span" | "u" | "var" => Some("generic"),
        "article" => Some("article"),
        "aside" => Some("complementary"),
        "blockquote" => Some("blockquote"),
        "button" | "summary" => Some("button"),
        "caption" => Some("caption"),
        "code" => Some("code"),
        "del" | "s" => Some("deletion"),
        "dialog" => Some("dialog"),
        "em" => Some("emphasis"),
        "footer" => {
            if has_sectioning_ancestor(doc, node) {
                Some("generic")
            } else {
                Some("contentinfo")
            }
        }
        "form" => Some("form"),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Some("heading"),
        "header" => {
            if has_sectioning_ancestor(doc, node) {
                Some("generic")
            } else {
                Some("banner")
            }
        }
        "hr" => Some("separator"),
        "img" => {
            if node.attr("alt") == Some("") && !has_presentational_role_conflict(node) {
                Some("presentation")
            } else {
                Some("img")
            }
        }
        "input" => implicit_input_role(node),
        "ins" => Some("insertion"),
        "li" => Some("listitem"),
        "main" => Some("main"),
        "mark" => Some("mark"),
        "math" => Some("math"),
        "menu" | "ol" | "ul" => Some("list"),
        "meter" => Some("meter"),
        "nav" => Some("navigation"),
        "option" => Some("option"),
        "p" => Some("paragraph"),
        "progress" => Some("progressbar"),
        "search" => Some("search"),
        "select" => {
            if node.has_attr("multiple")
                || node
                    .attr("size")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .is_some_and(|n| n > 1)
            {
                Some("listbox")
            } else {
                Some("combobox")
            }
        }
        "strong" => Some("strong"),
        "sub" => Some("subscript"),
        "sup" => Some("superscript"),
        "table" => Some("table"),
        "tbody" | "tfoot" | "thead" => Some("rowgroup"),
        "td" => {
            let in_grid = doc.ancestors(node.id).into_iter().find_map(|aid| {
                let anc = doc.node(aid)?;
                if anc.tag == "table" {
                    Some(matches!(
                        effective_explicit_role(anc),
                        Some("grid" | "treegrid")
                    ))
                } else {
                    None
                }
            });
            if in_grid == Some(true) {
                Some("gridcell")
            } else {
                Some("cell")
            }
        }
        "textarea" => Some("textbox"),
        "th" => Some("columnheader"),
        "time" => Some("time"),
        "tr" => Some("row"),
        _ => None,
    }
}

fn implicit_input_role(node: &Node) -> Option<&'static str> {
    let ty = node
        .attr("type")
        .map_or("text", str::trim)
        .to_ascii_lowercase();
    match ty.as_str() {
        "button" | "image" | "reset" | "submit" => Some("button"),
        "checkbox" => Some("checkbox"),
        "number" => Some("spinbutton"),
        "radio" => Some("radio"),
        "range" => Some("slider"),
        "search" => Some("searchbox"),
        "email" | "tel" | "text" | "url" => Some("textbox"),
        _ => None,
    }
}

fn has_sectioning_ancestor(doc: &Document, node: &Node) -> bool {
    for anc_id in doc.ancestors(node.id) {
        if let Some(anc) = doc.node(anc_id)
            && matches!(
                anc.tag.as_str(),
                "article" | "aside" | "main" | "nav" | "section"
            )
        {
            return true;
        }
    }
    false
}

/// Returns `true` if `node` has an explicit `role="none"` / `role="presentation"`
/// or implicit decorative `<img alt="">` that must be ignored per WAI-ARIA 1.2
/// Presentational Roles Conflict Resolution (because the element is focusable or
/// carries a global ARIA state/property).
pub fn has_presentational_role_conflict(node: &Node) -> bool {
    let is_explicit_pres = matches!(explicit_role(node), Some("none" | "presentation"));
    let is_implicit_pres_img =
        node.tag == "img" && explicit_role(node).is_none() && node.attr("alt") == Some("");
    if !is_explicit_pres && !is_implicit_pres_img {
        return false;
    }
    let focusable = !node.has_attr("disabled")
        && (is_natively_focusable(node)
            || node
                .attr("tabindex")
                .is_some_and(|t| t.trim().parse::<i32>().is_ok()));
    let has_global_aria = node
        .attrs
        .keys()
        .any(|k| k != "aria-hidden" && is_global_aria_attr(k));
    focusable || has_global_aria
}

/// Returns `true` if `node` has an effective `role="none"` or `role="presentation"`
/// (or decorative `<img alt="">`) without a presentational role conflict.
pub fn is_effectively_presentational(node: &Node) -> bool {
    let is_pres = matches!(explicit_role(node), Some("none" | "presentation"))
        || (node.tag == "img" && explicit_role(node).is_none() && node.attr("alt") == Some(""));
    is_pres && !has_presentational_role_conflict(node)
}

/// Returns the explicit role unless it is a `none`/`presentation` role overridden
/// by WAI-ARIA 1.2 Presentational Roles Conflict Resolution.
pub fn effective_explicit_role(node: &Node) -> Option<&str> {
    let role = explicit_role(node)?;
    if matches!(role, "none" | "presentation") && has_presentational_role_conflict(node) {
        return None;
    }
    Some(role)
}

/// Returns the effective role (explicit valid role unless overridden by presentational
/// role conflict resolution, falling back to implicit role).
pub fn effective_role<'a>(doc: &Document, node: &'a Node) -> Option<&'a str> {
    effective_explicit_role(node).or_else(|| implicit_role(doc, node))
}

/// Returns `true` if `id` or any ancestor has the HTML `inert` attribute.
pub fn is_inert(doc: &Document, id: NodeId) -> bool {
    doc.node(id).is_some_and(|n| n.has_attr("inert"))
        || doc
            .ancestors(id)
            .into_iter()
            .any(|aid| doc.node(aid).is_some_and(|anc| anc.has_attr("inert")))
}

/// Returns `true` if the element is sequentially keyboard-focusable (in the Tab order).
pub fn is_sequentially_focusable(node: &Node) -> bool {
    if !node.is_element() || node.has_attr("disabled") {
        return false;
    }
    if let Some(ti) = node.attr("tabindex")
        && let Ok(val) = ti.trim().parse::<i32>()
    {
        return val >= 0;
    }
    is_natively_focusable(node)
}

/// Returns `true` if the HTML element is natively keyboard-focusable.
pub fn is_natively_focusable(node: &Node) -> bool {
    if node.has_attr("disabled") {
        return false;
    }
    if node
        .attr("contenteditable")
        .is_some_and(|v| !v.eq_ignore_ascii_case("false"))
    {
        return true;
    }
    match node.tag.as_str() {
        "a" | "area" => node.has_attr("href"),
        "button" | "select" | "summary" | "textarea" => true,
        "input" => !node
            .attr("type")
            .is_some_and(|t| t.trim().eq_ignore_ascii_case("hidden")),
        "audio" | "video" => node.has_attr("controls"),
        _ => false,
    }
}

/// Returns `true` if the element is an interactive control (for nested-interactive
/// and target-size checks).
pub fn is_interactive_control(doc: &Document, node: &Node) -> bool {
    if !node.is_element() || node.has_attr("disabled") {
        return false;
    }
    if let Some(role) = effective_role(doc, node)
        && matches!(
            role,
            "button"
                | "checkbox"
                | "combobox"
                | "link"
                | "listbox"
                | "menuitem"
                | "menuitemcheckbox"
                | "menuitemradio"
                | "option"
                | "radio"
                | "searchbox"
                | "slider"
                | "spinbutton"
                | "switch"
                | "tab"
                | "textbox"
                | "treeitem"
        )
    {
        return true;
    }
    match node.tag.as_str() {
        "a" | "area" => node.has_attr("href"),
        "button" | "select" | "summary" | "textarea" => true,
        "input" => !node
            .attr("type")
            .is_some_and(|t| t.trim().eq_ignore_ascii_case("hidden")),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        ABSTRACT_ROLES, GLOBAL_ARIA_ATTRS, ISO_639_1_LANGS, ISO_639_2_WITH_1_EQUIV,
        VALID_ARIA_ATTRS, VALID_ROLES, allowed_attrs_for_role, is_abstract_role,
        is_attr_allowed_for_role, is_deprecated_role, is_valid_aria_attr, is_valid_bcp47_primary,
        is_valid_role, prohibited_attrs_for_role, required_children_for_role,
        required_parent_for_role, validate_aria_attr_value,
    };

    #[test]
    fn aria_tables_are_sorted_for_binary_search() {
        assert!(VALID_ROLES.windows(2).all(|w| w[0] < w[1]));
        assert!(ABSTRACT_ROLES.windows(2).all(|w| w[0] < w[1]));
        assert!(VALID_ARIA_ATTRS.windows(2).all(|w| w[0] < w[1]));
        assert!(GLOBAL_ARIA_ATTRS.windows(2).all(|w| w[0] < w[1]));
        assert!(ISO_639_1_LANGS.windows(2).all(|w| w[0] < w[1]));
        assert!(ISO_639_2_WITH_1_EQUIV.windows(2).all(|w| w[0] < w[1]));
        assert!(is_valid_role("button"));
        assert!(is_valid_role("doc-chapter"));
        assert!(is_valid_role("doc-toc"));
        assert!(!is_valid_role("widget"));
        assert!(is_abstract_role("widget"));
        assert!(!is_abstract_role("button"));
        assert!(is_deprecated_role("directory"));
        assert!(is_deprecated_role("doc-biblioentry"));
        assert!(is_deprecated_role("doc-endnote"));
        assert!(!is_deprecated_role("list"));
        assert!(is_valid_aria_attr("aria-labelledby"));
        assert!(is_valid_aria_attr("aria-actions"));
        assert!(!is_valid_aria_attr("aria-bogus"));
    }

    #[test]
    fn aria_role_relationships_and_prohibited_attrs() {
        assert!(prohibited_attrs_for_role("presentation").contains(&"aria-label"));
        assert!(prohibited_attrs_for_role("generic").contains(&"aria-labelledby"));
        assert!(!is_attr_allowed_for_role(Some("none"), "aria-label"));
        assert!(is_attr_allowed_for_role(Some("button"), "aria-pressed"));
        assert!(is_attr_allowed_for_role(Some("button"), "aria-label"));
        assert!(!is_attr_allowed_for_role(Some("button"), "aria-checked"));
        assert!(allowed_attrs_for_role("slider").contains(&"aria-valuenow"));

        assert!(required_children_for_role("tablist").contains(&"tab"));
        assert!(required_parent_for_role("tab").contains(&"tablist"));
        assert!(required_children_for_role("row").contains(&"gridcell"));
        assert!(required_parent_for_role("gridcell").contains(&"row"));
    }

    #[test]
    fn aria_attr_value_validation_covers_all_types() {
        let ids: HashSet<&str> = ["a", "b"].into_iter().collect();

        assert!(validate_aria_attr_value("aria-hidden", "true", &ids));
        assert!(!validate_aria_attr_value("aria-hidden", "TRUE", &ids));
        assert!(!validate_aria_attr_value("aria-hidden", "mixed", &ids));
        assert!(validate_aria_attr_value("aria-checked", "mixed", &ids));
        assert!(!validate_aria_attr_value("aria-checked", "TRUE", &ids));
        assert!(!validate_aria_attr_value("aria-checked", "yes", &ids));
        assert!(validate_aria_attr_value("aria-level", "2", &ids));
        assert!(!validate_aria_attr_value("aria-level", "0", &ids));
        assert!(validate_aria_attr_value("aria-setsize", "-1", &ids));
        assert!(!validate_aria_attr_value("aria-setsize", "-2", &ids));
        assert!(validate_aria_attr_value("aria-valuenow", "3.5", &ids));
        assert!(!validate_aria_attr_value("aria-valuenow", "NaN", &ids));
        assert!(validate_aria_attr_value("aria-live", "polite", &ids));
        assert!(!validate_aria_attr_value("aria-live", "loud", &ids));
        assert!(validate_aria_attr_value(
            "aria-relevant",
            "additions text",
            &ids
        ));
        assert!(!validate_aria_attr_value(
            "aria-relevant",
            "additions bogus",
            &ids
        ));
        assert!(validate_aria_attr_value("aria-activedescendant", "a", &ids));
        assert!(!validate_aria_attr_value(
            "aria-activedescendant",
            "missing",
            &ids
        ));
        assert!(validate_aria_attr_value("aria-labelledby", "a b", &ids));
        assert!(validate_aria_attr_value("aria-labelledby", "a c", &ids));
        assert!(!validate_aria_attr_value("aria-labelledby", "c d", &ids));
        assert!(!validate_aria_attr_value("aria-labelledby", "   ", &ids));
    }

    #[test]
    fn bcp47_primary_validation() {
        assert!(is_valid_bcp47_primary("en"));
        assert!(is_valid_bcp47_primary("en-US"));
        assert!(is_valid_bcp47_primary("es-419"));
        assert!(is_valid_bcp47_primary("zh-Hant-TW"));
        assert!(is_valid_bcp47_primary("fil"));
        assert!(!is_valid_bcp47_primary("eng"));
        assert!(!is_valid_bcp47_primary("fre"));
        assert!(!is_valid_bcp47_primary("fra"));
        assert!(!is_valid_bcp47_primary(""));
        assert!(!is_valid_bcp47_primary("123-bad"));
        assert!(!is_valid_bcp47_primary("zz"));
        assert!(!is_valid_bcp47_primary("english"));
        assert!(!is_valid_bcp47_primary("en-"));
    }
}
