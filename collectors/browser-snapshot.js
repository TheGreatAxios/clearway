/**
 * @fileoverview Browser-agnostic Clearway IR snapshot collector with zero
 * runtime dependencies.
 *
 * Works in any browser context: Chrome, Firefox, Safari/WebKit, Playwright,
 * Puppeteer, CDP `Runtime.evaluate`, or browser extensions. Produces a
 * JSON-serializable object matching `clearway::ir::Document`.
 *
 * Licensed under the MIT OR Apache-2.0 license.
 */

((globalScope, factory) => {
  const exportsObj = factory();
  if (typeof module !== 'undefined' && module.exports) {
    module.exports = exportsObj;
  } else if (globalScope) {
    globalScope.collectClearwayDocument = exportsObj.collectClearwayDocument;
  }
})(
    typeof globalThis !== 'undefined' ?
      globalThis :
      (typeof window !== 'undefined' ? window : null),
    () => {
      'use strict';

      // =======================================================================
      // 1. Constants
      // =======================================================================

      /**
       * Selector matching natively focusable or positively tabindexed elements.
       * @const {string}
       */
      const FOCUSABLE_SELECTOR = [
        'a[href]:not([tabindex="-1"])',
        'area[href]:not([tabindex="-1"])',
        'button:not([disabled]):not([tabindex="-1"])',
        'input:not([disabled]):not([type="hidden"]):not([tabindex="-1"])',
        'select:not([disabled]):not([tabindex="-1"])',
        'textarea:not([disabled]):not([tabindex="-1"])',
        'summary:not([tabindex="-1"])',
        'iframe:not([tabindex="-1"])',
        '[tabindex]:not([tabindex="-1"])',
        '[contenteditable]:not([contenteditable="false"]):not([tabindex="-1"])',
      ].join(',');

      /**
       * Tags whose descendants are never inspected by accessibility rules.
       * @const {!Set<string>}
       */
      const SKIP_CHILDREN_TAGS = new Set([
        'script',
        'template',
        'noscript',
      ]);

      /**
       * Graphic element tags that prevent solid-color backdrop resolution.
       * @const {!Set<string>}
       */
      const GRAPHIC_BACKDROP_TAGS = new Set([
        'img',
        'picture',
        'svg',
        'video',
        'canvas',
      ]);

      /**
       * SVG XML namespace URI.
       * @const {string}
       */
      const SVG_NAMESPACE = 'http://www.w3.org/2000/svg';

      /**
       * Default opaque white canvas backdrop color.
       * @const {{r: number, g: number, b: number, a: number}}
       */
      const TRANSPARENT_WHITE = Object.freeze({
        r: 255,
        g: 255,
        b: 255,
        a: 1,
      });

      // =======================================================================
      // 2. Color & CSS Parsing Helpers
      // =======================================================================

      /**
       * Clamps a numeric value to the inclusive range `[min, max]`.
       * @param {number} value
       * @param {number} min
       * @param {number} max
       * @return {number}
       */
      function clamp(value, min, max) {
        return Math.max(min, Math.min(max, value));
      }

      /**
       * Returns `value` if it is a finite number, otherwise `null`.
       * @param {*} value
       * @return {?number}
       */
      function finiteNumber(value) {
        return typeof value === 'number' && Number.isFinite(value) ?
          value :
          null;
      }

      /**
       * Parses an optional CSS alpha channel token into `[0, 1]`.
       * @param {string|undefined} rawAlpha
       * @return {number}
       */
      function parseAlpha(rawAlpha) {
        if (rawAlpha === undefined) {
          return 1;
        }
        const parsed = rawAlpha.endsWith('%') ?
          parseFloat(rawAlpha) / 100 :
          parseFloat(rawAlpha);
        return Number.isFinite(parsed) ? clamp(parsed, 0, 1) : 1;
      }

      /**
       * Parses a computed CSS color string (`transparent`, `rgb()`, `rgba()`, or
       * `color(srgb ...)`) into an RGBA object.
       * @param {?string} value
       * @return {?{r: number, g: number, b: number, a: number}}
       */
      function parseRgba(value) {
        if (!value) {
          return null;
        }
        if (value === 'transparent') {
          return {r: 0, g: 0, b: 0, a: 0};
        }
        const rgb = value.match(
            /^rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:\s*[,/]\s*([\d.]+%?))?\s*\)$/i,
        );
        if (rgb) {
          return {
            r: clamp(Math.round(parseFloat(rgb[1])), 0, 255),
            g: clamp(Math.round(parseFloat(rgb[2])), 0, 255),
            b: clamp(Math.round(parseFloat(rgb[3])), 0, 255),
            a: parseAlpha(rgb[4]),
          };
        }
        const srgb = value.match(
            /^color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)(?:\s*\/\s*([\d.]+%?))?\s*\)$/i,
        );
        if (srgb) {
          return {
            r: clamp(Math.round(parseFloat(srgb[1]) * 255), 0, 255),
            g: clamp(Math.round(parseFloat(srgb[2]) * 255), 0, 255),
            b: clamp(Math.round(parseFloat(srgb[3]) * 255), 0, 255),
            a: parseAlpha(srgb[4]),
          };
        }
        return null;
      }

      /**
       * Parses a CSS `font-weight` value into a numeric weight.
       * @param {?string} value
       * @return {?number}
       */
      function parseFontWeight(value) {
        if (!value) {
          return null;
        }
        if (value === 'normal') {
          return 400;
        }
        if (value === 'bold') {
          return 700;
        }
        const parsed = parseInt(value, 10);
        return Number.isFinite(parsed) && parsed >= 0 && parsed <= 65535 ?
          parsed :
          null;
      }

      /**
       * Returns `true` when every `background-size` layer has a hairline
       * dimension (`0`, `0%`, or `<= 4px`), such as CSS animated underlines.
       * @param {?string} bgSize
       * @return {boolean}
       */
      function isHairlineBackgroundSize(bgSize) {
        if (!bgSize) {
          return false;
        }
        const layers = bgSize.split(',');
        if (layers.length === 0) {
          return false;
        }
        for (let i = 0; i < layers.length; i++) {
          const tokens = layers[i].trim().split(/\s+/);
          let layerIsHairline = false;
          for (let j = 0; j < tokens.length; j++) {
            const tok = tokens[j].toLowerCase();
            if (tok === '0' || tok === '0%') {
              layerIsHairline = true;
              break;
            }
            const px = tok.match(/^(\d+(?:\.\d+)?|\.\d+)px$/);
            if (px && parseFloat(px[1]) <= 4) {
              layerIsHairline = true;
              break;
            }
          }
          if (!layerIsHairline) {
            return false;
          }
        }
        return true;
      }

      /**
       * Classifies a computed `background-image` into image vs. gradient stops.
       * @param {?string} bgImage
       * @param {?string} bgSize
       * @return {{
       *   has_background_image: boolean,
       *   background_gradient_stops: !Array<{r: number, g: number, b: number, a: number}>,
       * }}
       */
      function parseBackgroundImage(bgImage, bgSize) {
        if (!bgImage || bgImage === 'none') {
          return {
            has_background_image: false,
            background_gradient_stops: [],
          };
        }
        if (/url\(/i.test(bgImage)) {
          return {
            has_background_image: true,
            background_gradient_stops: [],
          };
        }
        // Decorative 1-4px gradient stripes (e.g. CSS animated underlines with
        // `background-size: 0% 2px` or `100% 2px`) are not text backgrounds.
        if (isHairlineBackgroundSize(bgSize)) {
          return {
            has_background_image: false,
            background_gradient_stops: [],
          };
        }
        const matches = bgImage.match(
            /rgba?\([^)]+\)|color\(srgb\s+[^)]+\)/gi,
        );
        const stops = [];
        if (matches) {
          for (let i = 0; i < matches.length; i++) {
            const parsed = parseRgba(matches[i]);
            if (parsed) {
              stops.push(parsed);
            }
          }
        }
        if (stops.length >= 2) {
          return {
            has_background_image: false,
            background_gradient_stops: stops,
          };
        }
        return {
          has_background_image: true,
          background_gradient_stops: [],
        };
      }

      /**
       * Returns `true` if computed `clip` or `clip-path` collapses the element.
       * @param {!CSSStyleDeclaration} cs
       * @return {boolean}
       */
      function isVisuallyClipped(cs) {
        const clip = (cs.clip || '').trim().toLowerCase();
        if (clip && clip !== 'auto') {
          const match = clip.match(
              /^rect\(\s*([^,]+)[,\s]+([^,]+)[,\s]+([^,]+)[,\s]+([^)]+)\)$/,
          );
          if (match) {
            const top = parseFloat(match[1]);
            const right = parseFloat(match[2]);
            const bottom = parseFloat(match[3]);
            const left = parseFloat(match[4]);
            if (
              Number.isFinite(top) &&
              Number.isFinite(right) &&
              Number.isFinite(bottom) &&
              Number.isFinite(left) &&
              (right <= left || bottom <= top)
            ) {
              return true;
            }
          }
        }
        const clipPath = (cs.clipPath || '').trim().toLowerCase();
        if (clipPath && clipPath !== 'none' && /^inset\(\s*50%/i.test(clipPath)) {
          return true;
        }
        return false;
      }

      /**
       * Parses a CSS angle literal (`deg`, `rad`, `grad`, `turn`, or `0`) into
       * degrees.
       * @param {?string} raw
       * @return {?number}
       */
      function parseAngleToDegrees(raw) {
        if (!raw) {
          return null;
        }
        const match = raw.trim().match(
            /^([+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)\s*(deg|rad|grad|turn)?$/i,
        );
        if (!match) {
          return null;
        }
        const val = parseFloat(match[1]);
        if (!Number.isFinite(val)) {
          return null;
        }
        const unit = (match[2] || '').toLowerCase();
        if (unit === 'deg') {
          return val;
        }
        if (unit === 'rad') {
          return (val * 180) / Math.PI;
        }
        if (unit === 'grad') {
          return val * 0.9;
        }
        if (unit === 'turn') {
          return val * 360;
        }
        return val === 0 ? 0 : null;
      }

      /**
       * Extracts the Z-axis rotation angle in degrees from a `matrix()` or
       * `matrix3d()` argument list.
       * @param {!Array<string>} args
       * @return {?number}
       */
      function extractMatrixRotationDeg(args) {
        const a = parseFloat(args[0]);
        const b = parseFloat(args[1]);
        if (Number.isFinite(a) && Number.isFinite(b) && (a !== 0 || b !== 0)) {
          return (Math.atan2(b, a) * 180) / Math.PI;
        }
        return null;
      }

      /**
       * Parses the CSS `rotate` property into Z-axis degrees.
       * @param {?string} value
       * @return {?number}
       */
      function parseRotatePropDeg(value) {
        if (!value) {
          return null;
        }
        const trimmed = value.trim().toLowerCase();
        if (!trimmed || trimmed === 'none') {
          return null;
        }
        const parts = trimmed.split(/\s+/);
        if (parts.length === 1) {
          return parseAngleToDegrees(parts[0]);
        }
        if (parts.length === 2 && parts[0] === 'z') {
          return parseAngleToDegrees(parts[1]);
        }
        if (parts.length === 4) {
          const x = parseFloat(parts[0]);
          const y = parseFloat(parts[1]);
          const z = parseFloat(parts[2]);
          if (Number.isFinite(z) && z !== 0 && x === 0 && y === 0) {
            return parseAngleToDegrees(parts[3]);
          }
        }
        return null;
      }

      /**
       * Sums Z-axis rotation angles in degrees from a CSS `transform` value.
       * @param {?string} value
       * @return {?number}
       */
      function parseTransformRotationDeg(value) {
        if (!value) {
          return null;
        }
        const trimmed = value.trim();
        if (!trimmed || trimmed.toLowerCase() === 'none') {
          return null;
        }
        let total = 0;
        let found = false;
        const fnRe = /\b(rotate|rotatez|rotate3d|matrix|matrix3d)\(\s*([^)]+)\)/gi;
        let match = fnRe.exec(trimmed);
        while (match) {
          const fn = match[1].toLowerCase();
          const args = match[2].split(',').map((s) => s.trim());
          if ((fn === 'rotate' || fn === 'rotatez') && args.length === 1) {
            const deg = parseAngleToDegrees(args[0]);
            if (deg !== null) {
              total += deg;
              found = true;
            }
          } else if (fn === 'rotate3d' && args.length === 4) {
            const x = parseFloat(args[0]);
            const y = parseFloat(args[1]);
            const z = parseFloat(args[2]);
            const deg = parseAngleToDegrees(args[3]);
            if (
              deg !== null &&
              Number.isFinite(z) &&
              z !== 0 &&
              x === 0 &&
              y === 0
            ) {
              total += z < 0 ? -deg : deg;
              found = true;
            }
          } else if (
            (fn === 'matrix' && args.length === 6) ||
            (fn === 'matrix3d' && args.length === 16)
          ) {
            const deg = extractMatrixRotationDeg(args);
            if (deg !== null) {
              total += deg;
              found = true;
            }
          }
          match = fnRe.exec(trimmed);
        }
        return found ? total : null;
      }

      /**
       * Returns `true` if `deg` is within 5 degrees of a 90 or 270 degree turn.
       * @param {number} deg
       * @return {boolean}
       */
      function isQuarterTurn(deg) {
        const norm = ((deg % 360) + 360) % 360;
        return Math.abs(norm - 90) <= 5 || Math.abs(norm - 270) <= 5;
      }

      // =======================================================================
      // 3. Computed Style, Visual Backdrop & Orientation-Lock Helpers
      // =======================================================================

      /**
       * Serializes the computed style fields required by Clearway IR.
       * @param {!Element} el
       * @param {?Window} fallbackWin
       * @return {?Object}
       */
      function readComputed(el, fallbackWin) {
        const elWin = (el && el.ownerDocument && el.ownerDocument.defaultView) ||
            fallbackWin;
        if (!elWin || typeof elWin.getComputedStyle !== 'function') {
          return null;
        }
        const cs = elWin.getComputedStyle(el);
        const out = {};
        const color = parseRgba(cs.color);
        if (color !== null) {
          out.color = color;
        }
        const bgColor = parseRgba(cs.backgroundColor);
        if (bgColor !== null) {
          out.background_color = bgColor;
        }
        const fontSize = parseFloat(cs.fontSize);
        if (Number.isFinite(fontSize)) {
          out.font_size_px = fontSize;
        }
        const fontWeight = parseFontWeight(cs.fontWeight);
        if (fontWeight !== null) {
          out.font_weight = fontWeight;
        }
        if (cs.display) {
          out.display = cs.display;
        }
        if (cs.visibility) {
          out.visibility = cs.visibility;
        }
        const opacity = parseFloat(cs.opacity);
        if (Number.isFinite(opacity)) {
          out.opacity = opacity;
        }
        if (cs.textDecorationLine) {
          out.text_decoration_line = cs.textDecorationLine;
        }
        if (cs.fontStyle) {
          out.font_style = cs.fontStyle;
        }
        if (cs.overflowX) {
          out.overflow_x = cs.overflowX;
        }
        if (cs.overflowY) {
          out.overflow_y = cs.overflowY;
        }
        const scrollWidth = finiteNumber(el.scrollWidth);
        if (scrollWidth !== null) {
          out.scroll_width = scrollWidth;
        }
        const clientWidth = finiteNumber(el.clientWidth);
        if (clientWidth !== null) {
          out.client_width = clientWidth;
        }
        const scrollHeight = finiteNumber(el.scrollHeight);
        if (scrollHeight !== null) {
          out.scroll_height = scrollHeight;
        }
        const clientHeight = finiteNumber(el.clientHeight);
        if (clientHeight !== null) {
          out.client_height = clientHeight;
        }
        if (cs.textShadow && cs.textShadow !== 'none') {
          out.has_text_shadow = true;
        }
        const borderBottomWidth = parseFloat(cs.borderBottomWidth);
        const borderBottomColor = parseRgba(cs.borderBottomColor);
        if (
          cs.borderBottomStyle &&
          cs.borderBottomStyle !== 'none' &&
          cs.borderBottomStyle !== 'hidden' &&
          Number.isFinite(borderBottomWidth) &&
          borderBottomWidth > 0 &&
          (!borderBottomColor || borderBottomColor.a > 0)
        ) {
          out.has_border_bottom = true;
        }
        if (cs.boxShadow && cs.boxShadow !== 'none') {
          out.has_box_shadow = true;
        }
        const bgClip = (cs.backgroundClip || cs.webkitBackgroundClip || '')
            .trim()
            .toLowerCase();
        const isBgClipText = bgClip === 'text' ||
            bgClip.split(',').some((part) => part.trim() === 'text');
        if (isBgClipText) {
          out.has_background_image = true;
        } else {
          const bgInfo = parseBackgroundImage(
              cs.backgroundImage,
              cs.backgroundSize,
          );
          if (bgInfo.has_background_image) {
            out.has_background_image = true;
          }
          if (bgInfo.background_gradient_stops.length > 0) {
            out.background_gradient_stops = bgInfo.background_gradient_stops;
          }
        }
        if (cs.pointerEvents) {
          out.pointer_events = cs.pointerEvents;
        }
        if (cs.flexDirection) {
          out.flex_direction = cs.flexDirection;
        }
        if (isVisuallyClipped(cs)) {
          out.is_clipped = true;
        }
        return out;
      }

      /**
       * Returns `true` if `el` has at least one non-whitespace direct text node.
       * @param {!Element} el
       * @return {boolean}
       */
      function hasDirectText(el) {
        for (let i = 0; i < el.childNodes.length; i++) {
          const child = el.childNodes[i];
          if (
            child.nodeType === 3 &&
            child.nodeValue &&
            child.nodeValue.trim().length > 0
          ) {
            return true;
          }
        }
        return false;
      }

      /**
       * Alpha-composites `fg` over `bg` using Porter-Duff source-over blending.
       * @param {{r: number, g: number, b: number, a: number}} fg
       * @param {{r: number, g: number, b: number, a: number}} bg
       * @return {{r: number, g: number, b: number, a: number}}
       */
      function compositeOver(fg, bg) {
        const fa = clamp(fg.a, 0, 1);
        const ba = clamp(bg.a, 0, 1);
        const outA = fa + ba * (1 - fa);
        if (outA <= 1e-6) {
          return {r: 0, g: 0, b: 0, a: 0};
        }
        const blend = (fc, bc) =>
          clamp(Math.round((fc * fa + bc * ba * (1 - fa)) / outA), 0, 255);
        return {
          r: blend(fg.r, bg.r),
          g: blend(fg.g, bg.g),
          b: blend(fg.b, bg.b),
          a: outA,
        };
      }

      /**
       * Resolves the effective visual background behind a direct-text element
       * via `elementsFromPoint`, handling z-index layers and canvas propagation.
       * @param {!Element} el
       * @param {?Object} computed
       * @param {?{x: number, y: number, width: number, height: number}} rect
       * @param {?Document} fallbackDoc
       * @param {?Window} fallbackWin
       */
      function populateVisualBackground(
          el,
          computed,
          rect,
          fallbackDoc,
          fallbackWin,
      ) {
        if (!computed || !rect || rect.width <= 0 || rect.height <= 0) {
          return;
        }
        if (
          computed.display === 'none' ||
          computed.visibility === 'hidden' ||
          computed.visibility === 'collapse'
        ) {
          return;
        }
        if (!hasDirectText(el)) {
          return;
        }
        if (
          computed.has_background_image ||
          (computed.background_gradient_stops &&
            computed.background_gradient_stops.length > 0)
        ) {
          return;
        }
        const ownOp =
            computed.opacity !== undefined && computed.opacity !== null ?
              clamp(computed.opacity, 0, 1) :
              1;
        if (
          computed.background_color &&
          computed.background_color.a * ownOp >= 0.99
        ) {
          return;
        }

        const ownerDoc = el.ownerDocument || fallbackDoc;
        const ownerWin = (ownerDoc && ownerDoc.defaultView) || fallbackWin;
        if (
          !ownerDoc ||
          typeof ownerDoc.elementsFromPoint !== 'function' ||
          !ownerWin
        ) {
          return;
        }

        const cx = rect.x + rect.width / 2;
        const cy = rect.y + rect.height / 2;
        if (
          cx < 0 ||
          cx >= ownerWin.innerWidth ||
          cy < 0 ||
          cy >= ownerWin.innerHeight
        ) {
          return;
        }

        const stack = ownerDoc.elementsFromPoint(cx, cy);
        const idx = stack.indexOf(el);
        if (idx === -1) {
          return;
        }

        const htmlEl = ownerDoc.documentElement;
        let bodyPropagatesToCanvas = false;
        if (htmlEl && ownerDoc.body) {
          const htmlCs = ownerWin.getComputedStyle(htmlEl);
          const htmlBg = parseRgba(htmlCs.backgroundColor);
          bodyPropagatesToCanvas =
              (!htmlBg || htmlBg.a === 0) &&
              (!htmlCs.backgroundImage || htmlCs.backgroundImage === 'none');
        }

        const layers = [];
        let deferredCanvasLayer = null;
        let deferredCanvasBgImage = false;
        for (let i = idx + 1; i < stack.length; i++) {
          const layer = stack[i];
          const lcs = ownerWin.getComputedStyle(layer);
          const lop = parseFloat(lcs.opacity);
          const op = Number.isFinite(lop) ? clamp(lop, 0, 1) : 1;
          const ltag = (layer.localName || layer.tagName || '').toLowerCase();
          if (
            op > 0 &&
            (GRAPHIC_BACKDROP_TAGS.has(ltag) ||
              layer.namespaceURI === SVG_NAMESPACE)
          ) {
            computed.has_background_image = true;
            return;
          }
          const isPropagatedBody =
              bodyPropagatesToCanvas && layer === ownerDoc.body;
          if (
            lcs.backgroundImage &&
            lcs.backgroundImage !== 'none' &&
            !(!/url\(/i.test(lcs.backgroundImage) &&
              isHairlineBackgroundSize(lcs.backgroundSize))
          ) {
            if (isPropagatedBody) {
              deferredCanvasBgImage = /url\(/i.test(lcs.backgroundImage);
              continue;
            }
            if (/url\(/i.test(lcs.backgroundImage) || !layer.contains(el)) {
              computed.has_background_image = true;
            }
            return;
          }
          const lbg = parseRgba(lcs.backgroundColor);
          if (lbg && lbg.a * op > 0) {
            const effA = clamp(lbg.a * op, 0, 1);
            const entry = {r: lbg.r, g: lbg.g, b: lbg.b, a: effA};
            if (isPropagatedBody) {
              deferredCanvasLayer = entry;
              continue;
            }
            layers.push(entry);
            if (effA >= 0.99) {
              break;
            }
          }
        }

        const hitOpaqueLayer =
            layers.length > 0 && layers[layers.length - 1].a >= 0.99;
        if (!hitOpaqueLayer) {
          if (deferredCanvasBgImage) {
            computed.has_background_image = true;
            return;
          }
          if (deferredCanvasLayer) {
            layers.push(deferredCanvasLayer);
          }
        }

        let result = {
          r: TRANSPARENT_WHITE.r,
          g: TRANSPARENT_WHITE.g,
          b: TRANSPARENT_WHITE.b,
          a: TRANSPARENT_WHITE.a,
        };
        for (let i = layers.length - 1; i >= 0; i--) {
          result = compositeOver(layers[i], result);
        }
        if (computed.background_color && computed.background_color.a > 0) {
          const effA = clamp(computed.background_color.a * ownOp, 0, 1);
          if (effA > 0) {
            result = compositeOver(
                {
                  r: computed.background_color.r,
                  g: computed.background_color.g,
                  b: computed.background_color.b,
                  a: effA,
                },
                result,
            );
          }
        }
        computed.visual_background = result;
      }

      /**
       * Records base (non-media-query) CSS rotations per selector.
       * @param {?CSSRuleList} rules
       * @param {!Map<string, {rotate: ?number, transform: ?number}>} baseMap
       */
      function collectBaseRotations(rules, baseMap) {
        if (!rules) {
          return;
        }
        for (let i = 0; i < rules.length; i++) {
          const rule = rules[i];
          if (rule.type === 1 && rule.style && rule.selectorText) {
            const rot = parseRotatePropDeg(rule.style.rotate);
            const tf = parseTransformRotationDeg(rule.style.transform);
            if (rot !== null || tf !== null) {
              const selectors = rule.selectorText
                  .split(',')
                  .map((s) => s.trim().toLowerCase());
              for (let s = 0; s < selectors.length; s++) {
                const key = selectors[s];
                const prev = baseMap.get(key) || {
                  rotate: null,
                  transform: null,
                };
                baseMap.set(key, {
                  rotate: rot !== null ? rot : prev.rotate,
                  transform: tf !== null ? tf : prev.transform,
                });
              }
            }
          }
        }
      }

      /**
       * Returns `true` if any `@media (orientation: ...)` rule rotates an
       * element by a quarter turn relative to its base rotation.
       * @param {?CSSRuleList} rules
       * @param {!Map<string, {rotate: ?number, transform: ?number}>} baseMap
       * @return {boolean}
       */
      function hasOrientationRotation(rules, baseMap) {
        if (!rules) {
          return false;
        }
        for (let i = 0; i < rules.length; i++) {
          const rule = rules[i];
          if (rule.type === 4) {
            const cond = (
              rule.conditionText ||
              (rule.media && rule.media.mediaText) ||
              ''
            ).toLowerCase();
            if (/orientation\s*:\s*(portrait|landscape)/.test(cond)) {
              const inner = rule.cssRules || [];
              for (let j = 0; j < inner.length; j++) {
                const styleRule = inner[j];
                if (!styleRule || !styleRule.style) {
                  continue;
                }
                const rot = parseRotatePropDeg(styleRule.style.rotate);
                const tf = parseTransformRotationDeg(styleRule.style.transform);
                if (rot === null && tf === null) {
                  continue;
                }
                const selectors = (styleRule.selectorText || '')
                    .split(',')
                    .map((s) => s.trim().toLowerCase());
                for (let s = 0; s < selectors.length; s++) {
                  const base = baseMap.get(selectors[s]) || {
                    rotate: null,
                    transform: null,
                  };
                  const baseAngle = (base.rotate ?? 0) + (base.transform ?? 0);
                  const effRot = rot !== null ? rot : (base.rotate ?? 0);
                  const effTf = tf !== null ? tf : (base.transform ?? 0);
                  const effAngle = effRot + effTf;
                  const delta = effAngle - baseAngle;
                  if (
                    isQuarterTurn(delta) ||
                    (effAngle !== baseAngle && isQuarterTurn(effAngle))
                  ) {
                    return true;
                  }
                }
              }
            }
          }
          if (rule.cssRules && hasOrientationRotation(rule.cssRules, baseMap)) {
            return true;
          }
        }
        return false;
      }

      /**
       * Detects whether same-origin stylesheets lock orientation via CSS
       * rotation inside an orientation media query.
       * @param {?Document} targetDoc
       * @return {boolean}
       */
      function detectCssOrientationLock(targetDoc) {
        if (!targetDoc || !targetDoc.styleSheets) {
          return false;
        }
        const baseMap = new Map();
        for (let i = 0; i < targetDoc.styleSheets.length; i++) {
          try {
            collectBaseRotations(targetDoc.styleSheets[i].cssRules, baseMap);
          } catch {
            // Cross-origin stylesheet; skip cleanly.
          }
        }
        for (let i = 0; i < targetDoc.styleSheets.length; i++) {
          try {
            if (
              hasOrientationRotation(targetDoc.styleSheets[i].cssRules, baseMap)
            ) {
              return true;
            }
          } catch {
            // Cross-origin stylesheet; skip cleanly.
          }
        }
        return false;
      }

      // =======================================================================
      // 4. Frame/Media Helpers & DOM Tree Walker
      // =======================================================================

      /**
       * Reads same-origin accessibility metadata for `<iframe>` and `<frame>`
       * elements.
       * @param {!Element} el
       * @param {string} tag
       * @return {{frame_tested: ?boolean, frame_has_focusable: ?boolean}}
       */
      function readFrameMetadata(el, tag) {
        if (tag !== 'iframe' && tag !== 'frame') {
          return {
            frame_tested: null,
            frame_has_focusable: null,
          };
        }
        try {
          const frameDoc = el.contentDocument;
          if (!frameDoc) {
            return {
              frame_tested: false,
              frame_has_focusable: null,
            };
          }
          return {
            frame_tested: true,
            frame_has_focusable: Boolean(
                frameDoc.querySelector(FOCUSABLE_SELECTOR),
            ),
          };
        } catch {
          return {
            frame_tested: false,
            frame_has_focusable: null,
          };
        }
      }

      /**
       * Reads finite media duration in seconds for `<audio>` and `<video>`.
       * @param {!Element} el
       * @param {string} tag
       * @return {?number}
       */
      function readMediaDuration(el, tag) {
        if (tag !== 'audio' && tag !== 'video') {
          return null;
        }
        return finiteNumber(el.duration);
      }

      /**
       * Reads the element's bounding client rectangle.
       * @param {!Element} el
       * @return {?{x: number, y: number, width: number, height: number}}
       */
      function readRect(el) {
        if (typeof el.getBoundingClientRect !== 'function') {
          return null;
        }
        const rect = el.getBoundingClientRect();
        return {
          x: rect.x ?? rect.left ?? 0,
          y: rect.y ?? rect.top ?? 0,
          width: rect.width ?? 0,
          height: rect.height ?? 0,
        };
      }

      /**
       * Appends `childId` to the `children` list of `nodes[parentId]`.
       * @param {!Array<!Object>} nodes
       * @param {number} parentId
       * @param {number} childId
       */
      function appendChild(nodes, parentId, childId) {
        const parent = nodes[parentId];
        if (parent.children) {
          parent.children.push(childId);
        } else {
          parent.children = [childId];
        }
      }

      /**
       * Recursively serializes a DOM node into `state.nodes`, traversing open
       * Shadow DOM, `<slot>` assigned nodes, and visible same-origin iframes.
       * @param {?Node} domNode
       * @param {number} parentId
       * @param {{
       *   nodes: !Array<!Object>,
       *   visitedDocs: !Set<!Document>,
       *   doc: ?Document,
       *   win: ?Window,
       * }} state
       */
      function walkDomTree(domNode, parentId, state) {
        if (!domNode) {
          return;
        }
        if (domNode.nodeType === 9 || domNode.nodeType === 11) {
          for (let i = 0; i < domNode.childNodes.length; i++) {
            walkDomTree(domNode.childNodes[i], parentId, state);
          }
          return;
        }
        if (domNode.nodeType === 3) {
          const text = domNode.nodeValue || '';
          if (!text) {
            return;
          }
          const id = state.nodes.length;
          state.nodes.push({
            id,
            parent: parentId,
            tag: '#text',
            text,
          });
          appendChild(state.nodes, parentId, id);
          return;
        }
        if (domNode.nodeType !== 1) {
          return;
        }

        const attrs = {};
        let hasAttrs = false;
        for (let i = 0; i < domNode.attributes.length; i++) {
          const attr = domNode.attributes[i];
          attrs[attr.name.toLowerCase()] = attr.value;
          hasAttrs = true;
        }
        const tag = (domNode.localName || domNode.tagName || '').toLowerCase();
        const frameMeta = readFrameMetadata(domNode, tag);
        const computed = readComputed(domNode, state.win);
        const rect = readRect(domNode);
        populateVisualBackground(domNode, computed, rect, state.doc, state.win);
        const mediaDuration = readMediaDuration(domNode, tag);
        const id = state.nodes.length;
        const node = {
          id,
          parent: parentId,
          tag,
        };
        if (hasAttrs) {
          node.attrs = attrs;
        }
        if (computed !== null) {
          node.computed = computed;
        }
        if (rect !== null) {
          node.rect = rect;
        }
        if (frameMeta.frame_tested !== null) {
          node.frame_tested = frameMeta.frame_tested;
        }
        if (frameMeta.frame_has_focusable !== null) {
          node.frame_has_focusable = frameMeta.frame_has_focusable;
        }
        if (mediaDuration !== null) {
          node.media_duration_secs = mediaDuration;
        }
        state.nodes.push(node);
        appendChild(state.nodes, parentId, id);

        if (tag === 'iframe') {
          const ariaHidden =
              (attrs['aria-hidden'] || '').trim().toLowerCase() === 'true';
          const selfHidden =
              'hidden' in attrs ||
              (computed &&
                (computed.display === 'none' ||
                  computed.visibility === 'hidden' ||
                  computed.visibility === 'collapse'));
          if (!ariaHidden && !selfHidden) {
            try {
              const frameDoc = domNode.contentDocument;
              if (
                frameDoc &&
                !state.visitedDocs.has(frameDoc) &&
                frameDoc.body
              ) {
                state.visitedDocs.add(frameDoc);
                for (let i = 0; i < frameDoc.body.childNodes.length; i++) {
                  walkDomTree(frameDoc.body.childNodes[i], id, state);
                }
              }
            } catch {
              // Cross-origin iframe; skip recursive walk cleanly.
            }
          }
        } else if (domNode.shadowRoot) {
          walkDomTree(domNode.shadowRoot, id, state);
        } else if (
          tag === 'slot' &&
          typeof domNode.assignedNodes === 'function'
        ) {
          const assigned = domNode.assignedNodes({flatten: true});
          const children = assigned.length > 0 ? assigned : domNode.childNodes;
          for (let i = 0; i < children.length; i++) {
            walkDomTree(children[i], id, state);
          }
        } else if (SKIP_CHILDREN_TAGS.has(tag)) {
          // Accessibility rules never inspect script, template, or noscript.
        } else {
          for (let i = 0; i < domNode.childNodes.length; i++) {
            walkDomTree(domNode.childNodes[i], id, state);
          }
        }
      }

      // =======================================================================
      // 5. Public Entry Point
      // =======================================================================

      /**
       * Collects a Clearway IR document snapshot starting from `root`.
       * @param {!Node=} root Root DOM node to snapshot; defaults to `document`.
       * @return {{
       *   nodes: !Array<!Object>,
       *   root: number,
       *   css_orientation_locked: (boolean|undefined),
       * }}
       */
      function collectClearwayDocument(root = document) {
        const doc = root && root.nodeType === 9 ?
          root :
          (root && root.ownerDocument) || document;
        const win = (doc && doc.defaultView) ||
            (typeof window !== 'undefined' ? window : null);
        const nodes = [
          {
            id: 0,
            tag: '#document',
          },
        ];
        const state = {
          nodes,
          visitedDocs: new Set(doc ? [doc] : []),
          doc,
          win,
        };

        walkDomTree(root, 0, state);
        const result = {
          nodes,
          root: 0,
        };
        if (detectCssOrientationLock(doc)) {
          result.css_orientation_locked = true;
        }
        return result;
      }

      return {collectClearwayDocument};
    },
);
