//! Media and deprecated animation rules: `blink`, `marquee`, `video-caption`,
//! and `audio-caption`.

use crate::ir::{Document, Node};
use crate::rule::{Impact, Rule, RuleMeta, Violation};

pub struct Blink;
pub struct Marquee;
pub struct VideoCaption;
pub struct AudioCaption;

fn has_captions_track(doc: &Document, media: &Node) -> bool {
    media.children.iter().any(|&cid| {
        doc.node(cid).is_some_and(|child| {
            child.tag == "track"
                && child
                    .attr("kind")
                    .is_some_and(|k| k.trim().eq_ignore_ascii_case("captions"))
        })
    })
}

impl Blink {
    pub const META: RuleMeta = RuleMeta {
        id: "blink",
        impact: Impact::Serious,
        wcag: "2.2.2 (A)",
        tags: &[
            "cat.time-and-media",
            "wcag2a",
            "wcag222",
            "section508",
            "section508.22.j",
            "TTv5",
            "TT2.b",
            "EN-301-549",
            "EN-9.2.2.2",
            "RGAAv4",
            "RGAA-13.8.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "<blink> elements are deprecated and must not be used",
        description: "Remove deprecated <blink> elements, which cause distracting flashing content without user controls.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/pause-stop-hide.html",
    };
}

impl Rule for Blink {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag == "blink" && doc.is_rendered(el.id) {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl Marquee {
    pub const META: RuleMeta = RuleMeta {
        id: "marquee",
        impact: Impact::Serious,
        wcag: "2.2.2 (A)",
        tags: &[
            "cat.parsing",
            "wcag2a",
            "wcag222",
            "TTv5",
            "TT2.b",
            "EN-301-549",
            "EN-9.2.2.2",
            "RGAAv4",
            "RGAA-13.8.1",
        ],
        enabled_by_default: true,
        needs_review: false,
        help: "<marquee> elements are deprecated and must not be used",
        description: "Remove deprecated <marquee> elements, which create moving content that cannot be paused or stopped.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/pause-stop-hide.html",
    };
}

impl Rule for Marquee {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag == "marquee" && doc.is_rendered(el.id) {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl VideoCaption {
    pub const META: RuleMeta = RuleMeta {
        id: "video-caption",
        impact: Impact::Critical,
        wcag: "1.2.2 (A)",
        tags: &[
            "cat.text-alternatives",
            "wcag2a",
            "wcag122",
            "section508",
            "section508.22.a",
            "TTv5",
            "TT17.a",
            "EN-301-549",
            "EN-9.1.2.2",
            "RGAAv4",
            "RGAA-4.3.1",
        ],
        enabled_by_default: true,
        needs_review: true,
        help: "<video> elements must have captions",
        description: "Include a <track kind=\"captions\"> child inside every accessible <video> element.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/captions-prerecorded.html",
    };
}

impl Rule for VideoCaption {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag == "video"
                && doc.is_in_accessibility_tree(el.id)
                && !has_captions_track(doc, el)
            {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}

impl AudioCaption {
    pub const META: RuleMeta = RuleMeta {
        id: "audio-caption",
        impact: Impact::Critical,
        wcag: "1.2.1 (A)",
        tags: &[
            "cat.time-and-media",
            "wcag2a",
            "wcag121",
            "EN-301-549",
            "EN-9.1.2.1",
            "section508",
            "section508.22.a",
            "deprecated",
        ],
        enabled_by_default: false,
        needs_review: true,
        help: "<audio> elements must have a captions track",
        description: "Include a <track kind=\"captions\"> child inside every accessible <audio> element.",
        docs_url: "https://www.w3.org/WAI/WCAG21/Understanding/audio-only-and-video-only-prerecorded.html",
    };
}

impl Rule for AudioCaption {
    fn meta(&self) -> RuleMeta {
        Self::META
    }

    fn run(&self, doc: &Document, out: &mut Vec<Violation>) {
        for el in doc.elements() {
            if el.tag == "audio"
                && doc.is_in_accessibility_tree(el.id)
                && !has_captions_track(doc, el)
            {
                out.push(Violation::from_node(&self.meta(), doc, el.id));
            }
        }
    }
}
