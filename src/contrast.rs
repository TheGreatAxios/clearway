//! WCAG 2.1 relative luminance, alpha compositing, and contrast ratio calculations.

use crate::ir::{Document, NodeId, Rgba};

/// Normal text AA minimum contrast ratio (WCAG 2.1 SC 1.4.3).
pub const AA_NORMAL_THRESHOLD: f64 = 4.5;

/// Large text AA minimum contrast ratio (WCAG 2.1 SC 1.4.3).
pub const AA_LARGE_THRESHOLD: f64 = 3.0;

/// Normal text AAA minimum contrast ratio (WCAG 2.1 SC 1.4.6).
pub const AAA_NORMAL_THRESHOLD: f64 = 7.0;

/// Large text AAA minimum contrast ratio (WCAG 2.1 SC 1.4.6).
pub const AAA_LARGE_THRESHOLD: f64 = 4.5;

/// Converts a single 8-bit sRGB channel into linear light in `[0.0, 1.0]`.
pub fn srgb_channel_to_linear(channel: u8) -> f64 {
    let s = f64::from(channel) / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// Computes WCAG 2.1 relative luminance in `[0.0, 1.0]` for an opaque RGB color.
pub fn relative_luminance(color: Rgba) -> f64 {
    let r = srgb_channel_to_linear(color.r);
    let g = srgb_channel_to_linear(color.g);
    let b = srgb_channel_to_linear(color.b);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Composites `fg` over `bg` using Porter-Duff `source-over` alpha blending.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn composite_over(fg: Rgba, bg: Rgba) -> Rgba {
    let fa = f64::from(fg.a.clamp(0.0, 1.0));
    let ba = f64::from(bg.a.clamp(0.0, 1.0));
    let out_a = fa + ba * (1.0 - fa);
    if out_a <= f64::EPSILON {
        return Rgba::new(0, 0, 0, 0.0);
    }

    let blend = |fc: u8, bc: u8| -> u8 {
        let val = (f64::from(fc) * fa + f64::from(bc) * ba * (1.0 - fa)) / out_a;
        val.round().clamp(0.0, 255.0) as u8
    };

    Rgba {
        r: blend(fg.r, bg.r),
        g: blend(fg.g, bg.g),
        b: blend(fg.b, bg.b),
        a: out_a as f32,
    }
}

/// Computes the WCAG 2.1 contrast ratio in `[1.0, 21.0]` between two opaque colors.
///
/// If `fg` has alpha `< 1.0`, it is composited over `bg` first.
pub fn contrast_ratio(fg: Rgba, bg: Rgba) -> f64 {
    let resolved_fg = if fg.a < 1.0 {
        composite_over(fg, bg)
    } else {
        fg
    };
    let l1 = relative_luminance(resolved_fg);
    let l2 = relative_luminance(bg);
    let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

/// Computes the APCA-W3 0.0.98G-4g perceptual lightness contrast (`Lc`) value.
///
/// Positive values indicate dark text on a light background; negative values
/// indicate light text on a dark background. If `fg` has alpha `< 1.0`, it is
/// composited over `bg` first.
pub fn apca_lc(fg: Rgba, bg: Rgba) -> f64 {
    const S_RCO: f64 = 0.212_672_9;
    const S_GCO: f64 = 0.715_152_2;
    const S_BCO: f64 = 0.072_175_0;
    const MAIN_TRC: f64 = 2.4;
    const BLK_THRS: f64 = 0.022;
    const BLK_CLMP: f64 = 1.414;
    const NORM_BG: f64 = 0.56;
    const NORM_TXT: f64 = 0.57;
    const REV_BG: f64 = 0.65;
    const REV_TXT: f64 = 0.62;
    const SCALE: f64 = 1.14;
    const LO_OFFSET: f64 = 0.027;
    const LO_CLIP: f64 = 0.1;
    const DELTA_Y_MIN: f64 = 0.0005;

    let resolved_fg = if fg.a < 1.0 {
        composite_over(fg, bg)
    } else {
        fg
    };

    let lum = |c: Rgba| -> f64 {
        let r = (f64::from(c.r) / 255.0).powf(MAIN_TRC);
        let g = (f64::from(c.g) / 255.0).powf(MAIN_TRC);
        let b = (f64::from(c.b) / 255.0).powf(MAIN_TRC);
        let y = S_RCO * r + S_GCO * g + S_BCO * b;
        if y > BLK_THRS {
            y
        } else {
            y + (BLK_THRS - y).powf(BLK_CLMP)
        }
    };

    let y_txt = lum(resolved_fg);
    let y_bg = lum(bg);
    if (y_bg - y_txt).abs() < DELTA_Y_MIN {
        return 0.0;
    }

    if y_bg > y_txt {
        let sapc = (y_bg.powf(NORM_BG) - y_txt.powf(NORM_TXT)) * SCALE;
        if sapc < LO_CLIP {
            0.0
        } else {
            (sapc - LO_OFFSET) * 100.0
        }
    } else {
        let sapc = (y_bg.powf(REV_BG) - y_txt.powf(REV_TXT)) * SCALE;
        if sapc > -LO_CLIP {
            0.0
        } else {
            (sapc + LO_OFFSET) * 100.0
        }
    }
}

/// Returns `true` if the typography qualifies as WCAG 2.1 "large scale" text:
/// at least `24px` (18pt), or at least `18.66px` (14pt) and bold (`>= 700`).
pub fn is_large_text(font_size_px: f32, font_weight: u16) -> bool {
    font_size_px >= 24.0 || (font_size_px >= 18.66 && font_weight >= 700)
}

/// Returns the required WCAG 2.1 AA contrast threshold (`3.0` or `4.5`).
pub fn aa_threshold(font_size_px: Option<f32>, font_weight: Option<u16>) -> f64 {
    let size = font_size_px.unwrap_or(16.0);
    let weight = font_weight.unwrap_or(400);
    if is_large_text(size, weight) {
        AA_LARGE_THRESHOLD
    } else {
        AA_NORMAL_THRESHOLD
    }
}

/// Returns the required WCAG 2.1 AAA contrast threshold (`4.5` or `7.0`).
pub fn aaa_threshold(font_size_px: Option<f32>, font_weight: Option<u16>) -> f64 {
    let size = font_size_px.unwrap_or(16.0);
    let weight = font_weight.unwrap_or(400);
    if is_large_text(size, weight) {
        AAA_LARGE_THRESHOLD
    } else {
        AAA_NORMAL_THRESHOLD
    }
}

/// Computes the effective opaque background behind `id` by compositing all
/// ancestor backgrounds from the canvas (opaque white) down to `id`.
///
/// When the element has a browser-sampled `visual_background` (from paint-order
/// hit-testing such as `elementsFromPoint`) and does not declare its own opaque
/// `background_color` or `background_gradient_stops`, `visual_background` is
/// returned directly.
pub fn effective_background(doc: &Document, id: NodeId) -> Rgba {
    if let Some(computed) = doc.node(id).and_then(|n| n.computed.as_ref())
        && let Some(vbg) = computed.visual_background
    {
        let opacity = computed.opacity.unwrap_or(1.0).clamp(0.0, 1.0);
        let has_own_opaque_bg = computed
            .background_color
            .is_some_and(|bg| bg.a * opacity >= 0.99);
        if !has_own_opaque_bg && computed.background_gradient_stops.is_empty() {
            return vbg;
        }
    }

    let mut chain = doc.ancestors(id);
    chain.reverse();
    chain.push(id);

    let mut bg = Rgba::opaque(255, 255, 255);
    for nid in chain {
        let Some(node) = doc.node(nid) else {
            continue;
        };
        let Some(computed) = &node.computed else {
            continue;
        };
        let opacity = computed.opacity.unwrap_or(1.0).clamp(0.0, 1.0);
        if let Some(mut layer) = computed.background_color {
            layer.a = (layer.a * opacity).clamp(0.0, 1.0);
            if layer.a > 0.0 {
                bg = composite_over(layer, bg);
            }
        }
    }
    bg
}

/// Computes the effective opaque foreground color of `id` over `bg`, applying
/// the element's own alpha and cumulative opacity.
pub fn effective_foreground(doc: &Document, id: NodeId, bg: Rgba) -> Option<Rgba> {
    let node = doc.node(id)?;
    let mut fg = node.computed.as_ref()?.color?;

    let mut cumulative_opacity = node
        .computed
        .as_ref()
        .and_then(|c| c.opacity)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    for anc_id in doc.ancestors(id) {
        if let Some(op) = doc
            .node(anc_id)
            .and_then(|n| n.computed.as_ref())
            .and_then(|c| c.opacity)
        {
            cumulative_opacity *= op.clamp(0.0, 1.0);
        }
    }

    fg.a = (fg.a * cumulative_opacity).clamp(0.0, 1.0);
    Some(composite_over(fg, bg))
}

#[cfg(test)]
mod tests {
    use super::{
        AA_LARGE_THRESHOLD, AA_NORMAL_THRESHOLD, aa_threshold, effective_background, is_large_text,
    };
    use crate::html::parse_html;
    use crate::ir::{ComputedStyle, Rgba};

    #[test]
    fn large_text_thresholds_match_wcag_21() {
        assert!(!is_large_text(16.0, 400));
        assert!(!is_large_text(18.66, 400));
        assert!(is_large_text(18.66, 700));
        assert!(is_large_text(24.0, 400));
        assert!((aa_threshold(Some(24.0), Some(400)) - AA_LARGE_THRESHOLD).abs() < f64::EPSILON);
        assert!((aa_threshold(Some(16.0), Some(400)) - AA_NORMAL_THRESHOLD).abs() < f64::EPSILON);
    }

    #[test]
    fn effective_background_uses_visual_background_for_transparent_elements() {
        let mut doc =
            parse_html(r#"<html lang="en"><body><p id="a">Hi</p><p id="b">Hi</p></body></html>"#);
        let dark = Rgba::opaque(17, 17, 17);
        let red = Rgba::opaque(200, 0, 0);

        if let Some(a_id) = doc.find_by_id("a") {
            if let Some(node) = doc.nodes.get_mut(a_id) {
                node.computed = Some(ComputedStyle {
                    background_color: Some(Rgba::new(0, 0, 0, 0.0)),
                    visual_background: Some(dark),
                    ..ComputedStyle::default()
                });
            }
            assert_eq!(effective_background(&doc, a_id), dark);
        }

        if let Some(b_id) = doc.find_by_id("b") {
            if let Some(node) = doc.nodes.get_mut(b_id) {
                node.computed = Some(ComputedStyle {
                    background_color: Some(red),
                    visual_background: Some(dark),
                    ..ComputedStyle::default()
                });
            }
            assert_eq!(effective_background(&doc, b_id), red);
        }
    }
}
