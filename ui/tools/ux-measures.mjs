/**
 * The rules `just ux-measure` counts by (#629), as
 * `docs/research/ux-reading-crowding-and-finding.md` section 3 defines them.
 *
 * `valuesOf` gathers what a screen was answered, in Node. Every other rule
 * lives in `measureInPage`, which runs in the browser and can reach nothing
 * of this file but itself and what it is handed, so the rules are written
 * once, there, and tested there, against pages a test draws.
 */

/**
 * Every string and number in a JSON answer, whitespace made single: the
 * result, and the words of a refusal, which the server writes and so are
 * content too.
 * @param {unknown} answer
 * @param {Set<string>} [into]
 */
export function valuesOf(answer, into = new Set()) {
	if (typeof answer === 'string') {
		const text = answer.replace(/\s+/g, ' ').trim();
		if (text) into.add(text);
	} else if (typeof answer === 'number') {
		into.add(String(answer));
	} else if (Array.isArray(answer)) {
		for (const item of answer) valuesOf(item, into);
	} else if (answer && typeof answer === 'object') {
		for (const item of Object.values(answer)) valuesOf(item, into);
	}
	return into;
}

/**
 * @typedef {object} MeasureInput
 * @property {string[]} values every string and number the page was answered, whitespace made single
 * @property {string[]} accents the accent colours, as CSS colours
 * @property {boolean} probe whether to walk each control's tapped area (Tap size and Gaps)
 * @property {boolean} [breakage] count only what breaks, for the 360 px pass: text cut off,
 *   boxes drawn over one another, a sideways scroll
 */

/**
 * Count one screen, in the browser, with the window at the top. Measures 1
 * to 9 of the research note's section 3; Depth is counted by hand. With
 * `breakage`, the 360 px pass instead (decision 3 of the map), which judges
 * nothing else at that width.
 *
 * Self-contained: it is handed to `page.evaluate`, so it reaches nothing
 * outside itself. It leaves the window scrolled where the last probe put it.
 * @param {MeasureInput} input
 */
export function measureInPage({ values, accents, probe, breakage = false }) {
	const W = innerWidth;
	const H = innerHeight;
	scrollTo({ top: 0, left: 0, behavior: 'instant' });
	const valueSet = new Set(values);

	// ── words, and what the page was answered ──
	const letter = /[\p{L}\p{N}]/u;
	const norm = (/** @type {string} */ t) => t.replace(/\s+/g, ' ').trim();
	const wordsOf = (/** @type {string} */ t) => t.split(/\s+/).filter((r) => letter.test(r)).length;
	const content = (/** @type {string} */ text) => {
		if (valueSet.has(text)) return true;
		if (wordsOf(text) < 3) return false;
		const touches = (/** @type {string | undefined} */ c) => c !== undefined && letter.test(c);
		for (const value of valueSet) {
			for (let at = value.indexOf(text); at >= 0; at = value.indexOf(text, at + 1)) {
				if (!touches(value[at - 1]) && !touches(value[at + text.length])) return true;
			}
		}
		return false;
	};

	// ── colours ──
	const canvas = document.createElement('canvas');
	canvas.width = canvas.height = 1;
	const ctx = /** @type {CanvasRenderingContext2D} */ (
		canvas.getContext('2d', { willReadFrequently: true })
	);
	/** @type {Map<string, number[]>} */
	const parsed = new Map();
	/** A CSS colour as [r, g, b, alpha 0–1]. */
	const rgba = (/** @type {string} */ css) => {
		let found = parsed.get(css);
		if (!found) {
			ctx.clearRect(0, 0, 1, 1);
			ctx.fillStyle = 'rgba(0,0,0,0)';
			ctx.fillStyle = css;
			ctx.fillRect(0, 0, 1, 1);
			const d = ctx.getImageData(0, 0, 1, 1).data;
			found = [d[0], d[1], d[2], d[3] / 255];
			parsed.set(css, found);
		}
		return found;
	};
	const over = (/** @type {number[]} */ top, /** @type {number[]} */ under) => {
		const a = top[3];
		return [0, 1, 2].map((i) => top[i] * a + under[i] * (1 - a)).concat(1);
	};
	const same = (/** @type {number[]} */ x, /** @type {number[]} */ y) =>
		[0, 1, 2].every((i) => Math.abs(x[i] - y[i]) <= 3);
	const luminance = (/** @type {number[]} */ c) => {
		const [r, g, b] = c.slice(0, 3).map((v) => {
			const s = v / 255;
			return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
		});
		return 0.2126 * r + 0.7152 * g + 0.0722 * b;
	};
	const contrast = (/** @type {number[]} */ x, /** @type {number[]} */ y) => {
		const [hi, lo] = [luminance(x), luminance(y)].sort((a, b) => b - a);
		return (hi + 0.05) / (lo + 0.05);
	};
	const accentColours = accents.map(rgba);
	const isAccent = (/** @type {string} */ css) => {
		const c = rgba(css);
		return c[3] > 0.5 && accentColours.some((a) => same(a, c));
	};
	/** @type {Map<Element, number[]>} */
	const behindCache = new Map();
	/** The colour behind an element: its ancestors' backgrounds laid over one another. */
	const behind = (/** @type {Element} */ el) => {
		let found = behindCache.get(el);
		if (found) return found;
		const layers = [];
		for (let a = el.parentElement; a; a = a.parentElement) {
			const c = rgba(getComputedStyle(a).backgroundColor);
			if (c[3] > 0) {
				layers.push(c);
				if (c[3] >= 0.999) break;
			}
		}
		found = [255, 255, 255, 1];
		for (const layer of layers.reverse()) found = over(layer, found);
		behindCache.set(el, found);
		return found;
	};
	const fillOf = (/** @type {Element} */ el) => rgba(getComputedStyle(el).backgroundColor);
	const filled = (/** @type {Element} */ el, least = 0.1) => {
		const bg = fillOf(el);
		return bg[3] > least && !same(over(bg, behind(el)), behind(el));
	};
	const bordered = (/** @type {Element} */ el) => {
		const cs = getComputedStyle(el);
		return ['Top', 'Right', 'Bottom', 'Left'].some((side) => {
			const width = parseFloat(cs.getPropertyValue(`border-${side.toLowerCase()}-width`));
			const style = cs.getPropertyValue(`border-${side.toLowerCase()}-style`);
			const colour = rgba(cs.getPropertyValue(`border-${side.toLowerCase()}-color`));
			return (
				width > 0 &&
				style !== 'none' &&
				colour[3] > 0 &&
				!same(over(colour, behind(el)), behind(el))
			);
		});
	};

	// ── what is visible ──
	/** @type {Map<Element, boolean>} */
	const shownCache = new Map();
	const shown = (/** @type {Element} */ el) => {
		let found = shownCache.get(el);
		if (found !== undefined) return found;
		// Hidden from sight is `display: none`, invisible or see-through, or a
		// box of one pixel or less, as `.sr-only` is. An ancestor that small
		// hides its children only when it clips them.
		const own = el.getBoundingClientRect();
		found =
			el.checkVisibility({ visibilityProperty: true, opacityProperty: true }) &&
			own.width > 1 &&
			own.height > 1;
		for (
			let a = el.parentElement;
			found && a && a !== document.documentElement;
			a = a.parentElement
		) {
			const r = a.getBoundingClientRect();
			const cs = getComputedStyle(a);
			const clips = cs.overflow !== 'visible' || cs.clip !== 'auto' || cs.clipPath !== 'none';
			if ((r.width <= 1 || r.height <= 1) && clips) found = false;
		}
		shownCache.set(el, found);
		return found;
	};
	const inView = (/** @type {DOMRect} */ r) =>
		r.bottom > 0 && r.top < H && r.right > 0 && r.left < W;

	// ── bars and notice cards ──
	const everything = [...document.body.querySelectorAll('*')];
	const bars = everything.filter((el) => {
		if (!shown(el)) return false;
		if ((el.tagName === 'HEADER' && !el.closest('main')) || el.getAttribute('role') === 'banner')
			return true;
		const { position } = getComputedStyle(el);
		const r = el.getBoundingClientRect();
		return (
			(position === 'fixed' || position === 'sticky') && r.width >= W / 2 && r.height < 0.9 * H
		);
	});
	const pinned = bars.filter((b) => ['fixed', 'sticky'].includes(getComputedStyle(b).position));
	const inBar = (/** @type {Element} */ el) => bars.some((b) => b.contains(el));
	const pinnedBar = (/** @type {Element} */ el) => pinned.find((b) => b.contains(el));
	const notices = [...document.querySelectorAll('[role=status], [role=alert]')].filter(
		(el) =>
			shown(el) && el.getBoundingClientRect().width >= W / 2 && (bordered(el) || filled(el, 0)),
	);
	const bands = [...bars, ...notices].map((el) => {
		const r = el.getBoundingClientRect();
		return [r.top, r.bottom];
	});
	const room = (() => {
		const clipped = bands
			.map(([t, b]) => [Math.max(0, t), Math.min(H, b)])
			.filter(([t, b]) => b > t)
			.sort((a, b) => a[0] - b[0]);
		let covered = 0;
		let reach = 0;
		for (const [t, b] of clipped) {
			const from = Math.max(t, reach);
			if (b > from) covered += b - from;
			reach = Math.max(reach, b);
		}
		return covered / H;
	})();

	// ── words ──
	/** @type {{ text: string, node: Node, first: boolean, block: Element, content: boolean }[]} */
	const segments = [];
	const blockOf = (/** @type {Element} */ el) => {
		for (let a = /** @type {Element | null} */ (el); a; a = a.parentElement) {
			const { display } = getComputedStyle(a);
			if (display !== 'inline' && display !== 'contents') return a;
		}
		return document.body;
	};
	const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
	const range = document.createRange();
	for (let node = walker.nextNode(); node; node = walker.nextNode()) {
		const parent = node.parentElement;
		if (!parent || parent.closest('script, style, noscript, template')) continue;
		const text = norm(node.textContent ?? '');
		if (!text || !shown(parent)) continue;
		range.selectNodeContents(node);
		const rects = [...range.getClientRects()].filter((r) => r.width > 0 && r.height > 0);
		if (rects.length === 0) continue;
		segments.push({
			text,
			node,
			first: rects.some(inView),
			block: blockOf(parent),
			content: content(text),
		});
	}
	for (const field of document.querySelectorAll('input[placeholder], textarea[placeholder]')) {
		const input = /** @type {HTMLInputElement} */ (field);
		const text = norm(input.placeholder);
		if (!text || input.value || !shown(input)) continue;
		segments.push({
			text,
			node: input,
			first: inView(input.getBoundingClientRect()),
			block: input,
			content: false,
		});
	}
	const interfaceSegments = segments.filter((s) => !s.content);
	const sum = (/** @type {typeof segments} */ list) =>
		list.reduce((n, s) => n + wordsOf(s.text), 0);
	/** @type {{ text: string, first: boolean }[]} */
	const sentences = [];
	// Interface text is joined within its nearest block, and content between
	// two stretches of it ends the first, so a recipe's title does not run
	// the words either side of it into one sentence.
	/** @type {Map<Element, (typeof segments)[]>} */
	const byBlock = new Map();
	for (const s of segments) {
		const chunks = byBlock.get(s.block) ?? [[]];
		byBlock.set(s.block, chunks);
		if (s.content) chunks.push([]);
		else chunks[chunks.length - 1].push(s);
	}
	for (const list of [...byBlock.values()].flat().filter((chunk) => chunk.length > 0)) {
		let text = '';
		/** @type {{ at: number, first: boolean }[]} */
		const starts = [];
		for (const s of list) {
			if (text) text += ' ';
			starts.push({ at: text.length, first: s.first });
			text += s.text;
		}
		const firstAt = (/** @type {number} */ at) => {
			let found = starts[0].first;
			for (const start of starts) if (start.at <= at) found = start.first;
			return found;
		};
		const pattern = /[^]*?(?:[.?!…](?=\s|$)|$)/gy;
		for (let m = pattern.exec(text); m && m[0]; m = pattern.exec(text)) {
			const lead = m[0].length - m[0].trimStart().length;
			const sentence = m[0].trim();
			if (sentence) sentences.push({ text: sentence, first: firstAt(m.index + lead) });
		}
	}
	const explains = (/** @type {string} */ t) => /[.?!…]$/.test(t) && wordsOf(t) >= 6;
	const words = {
		interfaceFirst: sum(interfaceSegments.filter((s) => s.first)),
		interfacePage: sum(interfaceSegments),
		contentFirst: sum(segments.filter((s) => s.content && s.first)),
		contentPage: sum(segments.filter((s) => s.content)),
		explanationsFirst: sentences.filter((s) => s.first && explains(s.text)).map((s) => s.text),
		explanationsPage: sentences.filter((s) => explains(s.text)).map((s) => s.text),
		long: sentences.filter((s) => wordsOf(s.text) > 25).map((s) => s.text),
		sentences: sentences.map((s) => s.text),
		interfaceTexts: interfaceSegments.map((s) => s.text),
	};

	// ── controls ──
	const CONTROL =
		'a[href], button, input:not([type=hidden]), select, textarea, summary, [role=button], [role=link], [role=tab], [role=checkbox], [role=switch], [role=menuitem], [role=option]';
	const controls = [...document.querySelectorAll(CONTROL)].filter(
		(el) => shown(el) && !el.parentElement?.closest(CONTROL),
	);
	const label = (/** @type {Element} */ el) => {
		const html = /** @type {HTMLInputElement} */ (el);
		const text =
			el.getAttribute('aria-label') ||
			norm(html.innerText ?? '') ||
			html.placeholder ||
			html.value ||
			'';
		return norm(text).slice(0, 48) || el.tagName.toLowerCase();
	};
	const drawn = new Map(controls.map((c) => [c, c.getBoundingClientRect()]));
	if (breakage) {
		const clipped = (/** @type {string} */ overflow) =>
			overflow === 'hidden' || overflow === 'clip';
		const cutOff = everything
			.filter((el) => {
				if (el.matches('input, textarea, select') || !shown(el)) return false;
				if (!letter.test(/** @type {HTMLElement} */ (el).innerText ?? '')) return false;
				const cs = getComputedStyle(el);
				return (
					(clipped(cs.overflowX) && el.scrollWidth > el.clientWidth + 1) ||
					(clipped(cs.overflowY) && el.scrollHeight > el.clientHeight + 1)
				);
			})
			.map((el) => norm(/** @type {HTMLElement} */ (el).innerText ?? '').slice(0, 48));
		// Every text and control, each with its boxes, compared only with what
		// shares its bar or with the page: the tab bar is meant to lie over
		// the page, but its labels must not run into one another.
		/** @type {{ node: Node, label: string, boxes: DOMRect[], group: Element | null }[]} */
		const items = segments
			.filter((s) => s.node instanceof Text && !s.node.parentElement?.closest(CONTROL))
			.map((s) => {
				range.selectNodeContents(s.node);
				const boxes = [...range.getClientRects()].filter((r) => r.width > 0 && r.height > 0);
				const group = pinnedBar(/** @type {Element} */ (s.node.parentElement)) ?? null;
				return { node: s.node, label: s.text.slice(0, 48), boxes, group };
			});
		for (const c of controls) {
			items.push({
				node: c,
				label: label(c),
				boxes: [/** @type {DOMRect} */ (drawn.get(c))],
				group: pinnedBar(c) ?? null,
			});
		}
		items.sort((a, b) =>
			a.node.compareDocumentPosition(b.node) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1,
		);
		/** @type {string[][]} */
		const overlaps = [];
		for (let i = 0; i < items.length; i++) {
			for (let j = i + 1; j < items.length; j++) {
				const [a, b] = [items[i], items[j]];
				if (a.group !== b.group || a.node.contains(b.node) || b.node.contains(a.node)) continue;
				const crossed = a.boxes.some((x) =>
					b.boxes.some(
						(y) =>
							Math.min(x.right, y.right) - Math.max(x.left, y.left) > 2 &&
							Math.min(x.bottom, y.bottom) - Math.max(x.top, y.top) > 2,
					),
				);
				if (crossed) overlaps.push([a.label, b.label]);
			}
		}
		const root = document.documentElement;
		return { cutOff, overlaps, sideways: Math.max(0, root.scrollWidth - root.clientWidth) };
	}
	const isField = (/** @type {Element} */ el) => el.matches('input, select, textarea');
	const held = (/** @type {Element} */ el) =>
		['aria-pressed', 'aria-selected', 'aria-checked'].some((a) => el.getAttribute(a) === 'true') ||
		(el.hasAttribute('aria-current') && el.getAttribute('aria-current') !== 'false');
	/** The element that draws a control's fill: itself, or a child as large as it. */
	const face = (/** @type {Element} */ el) => {
		if (fillOf(el)[3] > 0) return el;
		const r = /** @type {DOMRect} */ (drawn.get(el));
		for (const child of el.querySelectorAll('*')) {
			const cr = child.getBoundingClientRect();
			if (
				Math.abs(cr.width - r.width) <= 2 &&
				Math.abs(cr.height - r.height) <= 2 &&
				fillOf(child)[3] > 0
			)
				return child;
		}
		return el;
	};
	const isMain = (/** @type {Element} */ el) => {
		const r = /** @type {DOMRect} */ (drawn.get(el));
		if (r.height < 40 || held(el)) return false;
		const f = face(el);
		const bg = fillOf(f);
		return bg[3] > 0.5 && contrast(over(bg, behind(f)), behind(f)) >= 3;
	};
	const firstControls = controls.filter((c) => inView(/** @type {DOMRect} */ (drawn.get(c))));
	const mains = controls.filter(isMain);

	// ── unmarked actions ──
	const underlined = (/** @type {Element} */ el) =>
		[el, ...el.querySelectorAll('*')].some((e) =>
			getComputedStyle(e).textDecorationLine.includes('underline'),
		);
	const accented = (/** @type {Element} */ el) =>
		[el, ...el.querySelectorAll('*')].some((e) => {
			const cs = getComputedStyle(e);
			const ownText = [...e.childNodes].some(
				(n) => n.nodeType === Node.TEXT_NODE && letter.test(n.textContent ?? ''),
			);
			const drawing = e instanceof SVGElement;
			return (
				((ownText || drawing) && isAccent(cs.color)) ||
				(drawing && (isAccent(cs.fill) || isAccent(cs.stroke)))
			);
		});
	const pictured = (/** @type {Element} */ el) =>
		!!el.querySelector('img, picture') ||
		[el, ...el.querySelectorAll('*')].some((e) =>
			getComputedStyle(e).backgroundImage.includes('url('),
		);
	const listRow = (/** @type {Element} */ el) => {
		const row = el.closest('li') ?? el;
		const parent = row.parentElement;
		if (!parent) return false;
		const r = row.getBoundingClientRect();
		const alike = [...parent.children].filter((s) => {
			if (s.tagName !== row.tagName || s.className !== row.className || !shown(s)) return false;
			if (!(s.matches(CONTROL) || s.querySelector(CONTROL))) return false;
			return Math.abs(s.getBoundingClientRect().left - r.left) <= 2;
		});
		return new Set(alike.map((s) => Math.round(s.getBoundingClientRect().top))).size >= 3;
	};
	const unmarked = firstControls
		.filter(
			(c) =>
				!isField(c) &&
				!inBar(c) &&
				!pictured(c) &&
				!listRow(c) &&
				!filled(face(c)) &&
				!bordered(c) &&
				!underlined(c) &&
				!accented(c),
		)
		.map(label);

	// ── type and colour ──
	const sizes = new Set();
	const colours = new Set();
	for (const s of segments) {
		// A text is drawn in its element's type; a placeholder in its field's
		// size and its own colour.
		const field = s.node instanceof Element ? s.node : null;
		const owner = field ?? s.node.parentElement;
		if (!owner) continue;
		const cs = getComputedStyle(owner);
		const colour = field ? getComputedStyle(field, '::placeholder').color : cs.color;
		sizes.add(Math.round(parseFloat(cs.fontSize)));
		colours.add(
			rgba(colour)
				.map((v, i) => (i === 3 ? Math.round(v * 100) / 100 : v))
				.join(','),
		);
	}

	// ── length ──
	const pageHeight = document.documentElement.scrollHeight;
	const firstMain = mains[0] && /** @type {DOMRect} */ (drawn.get(mains[0]));
	const length = {
		screenfuls: Math.round((pageHeight / H) * 100) / 100,
		mainActionAt: firstMain ? Math.round((firstMain.top / H) * 100) / 100 : null,
	};

	// ── tap size and gaps ──
	/** @type {{ label: string, width: number, height: number }[]} */
	const small = [];
	/** @type {string[]} */
	const cutShort = [];
	/** @type {{ label: string, by: string }[]} */
	const covered = [];
	/** @type {string[]} */
	const inSentence = [];
	/** @type {{ el: Element, group: Element | null, box: number[] }[]} */
	const tapped = [];
	if (probe) {
		const topLimit = Math.max(
			0,
			...pinned
				.map((b) => b.getBoundingClientRect())
				.filter((r) => r.top <= 1)
				.map((r) => r.bottom),
		);
		const bottomLimit = Math.min(
			H,
			...pinned
				.map((b) => b.getBoundingClientRect())
				.filter((r) => r.bottom >= H - 1)
				.map((r) => r.top),
		);
		const scrollers = everything.filter(
			(e) => e.scrollWidth > e.clientWidth + 1 || e.scrollHeight > e.clientHeight + 1,
		);
		const kept = scrollers.map((e) => [e.scrollLeft, e.scrollTop]);
		const putShelvesBack = () =>
			scrollers.forEach((e, i) => {
				e.scrollLeft = kept[i][0];
				e.scrollTop = kept[i][1];
			});
		// A point answers one element, so where two tapped areas overlap the
		// walk gives the shared strip to whichever lies on top, and no overlap
		// shows. Each control is walked a second time with every other control
		// set aside, which gives the whole area it claims: Gaps compares those.
		const aside = document.createElement('style');
		aside.textContent =
			'body[data-ux-alone] [data-ux-control]:not([data-ux-probing]), body[data-ux-alone] [data-ux-control]:not([data-ux-probing]) * { pointer-events: none !important; } [data-ux-far] { visibility: hidden !important; }';
		// Chrome hit-tests the whole page for every point asked: on the
		// recipe list, 24,000px tall, a point cost a quarter of a millisecond
		// and the walk 20 seconds a screen (#638). Whatever lies, with all it
		// holds, more than a window's width or height outside the window is
		// under no point the walk asks, so it is hidden while the walks run,
		// which makes each point about ten times cheaper. Finding it reads
		// every box, so it is found again only once the window has moved half
		// a window, or a shelf has scrolled, since it was last found: what
		// was hidden is then still half a window away. It is all shown again
		// before any text is read, since hidden text is no part of `innerText`.
		/** @type {Element[]} */
		let farAside = [];
		/** @type {{ x: number, y: number, shelves: string } | null} */
		let farFoundAt = null;
		const setFarAside = () => {
			const now = {
				x: scrollX,
				y: scrollY,
				shelves: scrollers.map((e) => `${e.scrollLeft} ${e.scrollTop}`).join(),
			};
			if (
				farFoundAt &&
				Math.abs(now.x - farFoundAt.x) <= W / 2 &&
				Math.abs(now.y - farFoundAt.y) <= H / 2 &&
				now.shelves === farFoundAt.shelves
			)
				return;
			farFoundAt = now;
			/** @type {Set<Element>} */
			const far = new Set();
			for (let i = everything.length - 1; i >= 0; i--) {
				const el = everything[i];
				const r = el.getBoundingClientRect();
				const out = r.bottom < -H || r.top > 2 * H || r.right < -W || r.left > 2 * W;
				if (out && [...el.children].every((child) => far.has(child))) far.add(el);
			}
			const roots = new Set(
				[...far].filter((el) => !el.parentElement || !far.has(el.parentElement)),
			);
			for (const el of farAside) if (!roots.has(el)) el.removeAttribute('data-ux-far');
			for (const el of roots) el.setAttribute('data-ux-far', '');
			farAside = [...roots];
		};
		document.head.append(aside);
		for (const c of controls) c.setAttribute('data-ux-control', '');
		const insideSentence = (/** @type {Element} */ c) => {
			if (c.tagName !== 'A' || getComputedStyle(c).display !== 'inline') return false;
			const block = blockOf(c);
			const all = norm(/** @type {HTMLElement} */ (block).innerText ?? '');
			const own = norm(/** @type {HTMLElement} */ (c).innerText ?? '');
			return letter.test(all.replace(own, ''));
		};
		/** Whether a finger at a point lands on the control. */
		const isOn = (/** @type {Element} */ c, /** @type {Element | null} */ e) =>
			!!e && (e === c || c.contains(e));
		// The screen's edge ends a tapped area as surely as a finger's
		// reach does, so it is no reason to call the area cut short.
		const onScreen = (/** @type {number} */ x, /** @type {number} */ y) =>
			x >= 0 && y >= 0 && x < W && y < H;
		/**
		 * Bring a control to the middle of the room the bars leave, and say
		 * where its middle is. Run over the controls in the same order from
		 * the same start, it leaves the page as it left it the time before.
		 * @param {Element} c
		 */
		const bringUp = (c) => {
			c.scrollIntoView({ block: 'nearest', inline: 'center', behavior: 'instant' });
			if (inBar(c)) scrollTo({ top: 0, behavior: 'instant' });
			else {
				const r = c.getBoundingClientRect();
				scrollBy({ top: r.top + r.height / 2 - (topLimit + bottomLimit) / 2, behavior: 'instant' });
			}
			setFarAside();
			const r = c.getBoundingClientRect();
			return { r, cx: Math.floor(r.left + r.width / 2), cy: Math.floor(r.top + r.height / 2) };
		};
		/** How far a walk jumps while it stays on the control, in pixels. */
		const WALK_STRIDE = 8;
		/**
		 * How far a finger can move from a control's middle in each of four
		 * directions and still tap it, and whether a bar's control stopped
		 * it. A stride at a time while the far point is still the control's,
		 * then a pixel at a time across the last stride, where the edge lies
		 * (#638). A point jumped over is taken to be the control's when the
		 * points either side of it are: something narrower than a stride lying
		 * across the middle of a control is all this can miss.
		 * @param {Element} c
		 * @param {number} cx
		 * @param {number} cy
		 */
		const walkFrom = (c, cx, cy) => {
			const bar = inBar(c);
			let short = false;
			const hit = (/** @type {number} */ x, /** @type {number} */ y) =>
				isOn(c, document.elementFromPoint(x, y));
			const walk = (/** @type {number} */ dx, /** @type {number} */ dy) => {
				let d = 0;
				for (;;) {
					const far = d + WALK_STRIDE;
					if (onScreen(cx + dx * far, cy + dy * far) && hit(cx + dx * far, cy + dy * far)) {
						d = far;
						continue;
					}
					for (;;) {
						const x = cx + dx * (d + 1);
						const y = cy + dy * (d + 1);
						if (!onScreen(x, y)) return d;
						if (!hit(x, y)) {
							const e = document.elementFromPoint(x, y);
							if (!bar && e && inBar(e)) short = true;
							return d;
						}
						d++;
					}
				}
			};
			const reach = [walk(-1, 0), walk(1, 0), walk(0, -1), walk(0, 1)];
			return { reach, short };
		};
		const startedAt = [scrollX, scrollY];
		/** @type {Map<Element, { r: DOMRect, width: number, height: number, short: boolean }>} */
		const walked = new Map();
		/** @type {{ c: Element, by: Element | null }[]} */
		const coveredBy = [];
		for (const c of controls) {
			const { r, cx, cy } = bringUp(c);
			const centre = document.elementFromPoint(cx, cy);
			if (!isOn(c, centre)) {
				coveredBy.push({ c, by: centre });
				continue;
			}
			const { reach, short } = walkFrom(c, cx, cy);
			const [left, right, up, down] = reach;
			walked.set(c, { r, width: left + right + 1, height: up + down + 1, short });
		}
		// The second walk, each control alone, from the same start, so each
		// control comes up where it did the first time. The page is set to
		// "alone" once for all of them, since every change to the page's
		// own marks has Chrome work out the whole page's styles again.
		putShelvesBack();
		scrollTo({ left: startedAt[0], top: startedAt[1], behavior: 'instant' });
		document.body.setAttribute('data-ux-alone', '');
		for (const c of controls) {
			const { cx, cy } = bringUp(c);
			const first = walked.get(c);
			if (!first) continue;
			c.setAttribute('data-ux-probing', '');
			const claimed = walkFrom(c, cx, cy).reach;
			c.removeAttribute('data-ux-probing');
			// The area in the page's own layout, with the window at the top and
			// every shelf where it started, so two controls can be compared.
			const { r } = first;
			const r0 = /** @type {DOMRect} */ (drawn.get(c));
			tapped.push({
				el: c,
				group: pinnedBar(c) ?? null,
				box: [
					r0.left - (r.left - (cx - claimed[0])),
					r0.top - (r.top - (cy - claimed[2])),
					r0.right + (cx + claimed[1] + 1 - r.right),
					r0.bottom + (cy + claimed[3] + 1 - r.bottom),
				],
			});
		}
		document.body.removeAttribute('data-ux-alone');
		for (const el of farAside) el.removeAttribute('data-ux-far');
		for (const { c, by } of coveredBy)
			covered.push({ label: label(c), by: by ? label(by) : 'nothing' });
		for (const [c, { width, height, short }] of walked) {
			if (insideSentence(c)) inSentence.push(label(c));
			else if (width < 48 || height < 48) {
				if (short) cutShort.push(label(c));
				else small.push({ label: label(c), width, height });
			}
		}
		putShelvesBack();
		aside.remove();
		for (const c of controls) c.removeAttribute('data-ux-control');
	}
	/** @type {string[][]} */
	const overlapping = [];
	for (let i = 0; i < tapped.length; i++) {
		for (let j = i + 1; j < tapped.length; j++) {
			const [a, b] = [tapped[i], tapped[j]];
			if (a.group !== b.group) continue;
			const across = Math.min(a.box[2], b.box[2]) - Math.max(a.box[0], b.box[0]);
			const down = Math.min(a.box[3], b.box[3]) - Math.max(a.box[1], b.box[1]);
			if (across > 1 && down > 1) overlapping.push([label(a.el), label(b.el)]);
		}
	}
	/** @type {{ px: number, between: string[] } | null} */
	let smallest = null;
	for (let i = 0; i < controls.length; i++) {
		for (let j = i + 1; j < controls.length; j++) {
			const [a, b] = [controls[i], controls[j]];
			if ((pinnedBar(a) ?? null) !== (pinnedBar(b) ?? null)) continue;
			const [ra, rb] = [
				/** @type {DOMRect} */ (drawn.get(a)),
				/** @type {DOMRect} */ (drawn.get(b)),
			];
			const isSmall = (/** @type {DOMRect} */ r) => r.width < 48 || r.height < 48;
			if (!isSmall(ra) && !isSmall(rb)) continue;
			const dx = Math.max(0, rb.left - ra.right, ra.left - rb.right);
			const dy = Math.max(0, rb.top - ra.bottom, ra.top - rb.bottom);
			const px = Math.round(Math.hypot(dx, dy) * 10) / 10;
			if (!smallest || px < smallest.px) smallest = { px, between: [label(a), label(b)] };
		}
	}

	return {
		words,
		room: Math.round(room * 1000) / 1000,
		controls: { first: firstControls.length, mainButtons: firstControls.filter(isMain).map(label) },
		unmarked,
		tap: { small, cutShort, covered, inSentence },
		gaps: { overlapping, smallest },
		type: { sizes: [...sizes].sort((a, b) => a - b), colours: [...colours] },
		length,
		notices: notices.length,
	};
}
