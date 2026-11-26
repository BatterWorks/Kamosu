<!--
	The cooking screen: ONE Step filling the phone, with the amounts for that
	step above it (ADR 0011).

	It is used at arm's length, with wet hands, on a phone propped against
	something — which is the whole of why it is shaped like this:

	  · the Step's text is the LARGEST TYPE IN THE APP (`text-step`, 26px), and
	    it is the only thing on screen that scrolls;
	  · above it, ONLY the Ingredients this step uses. Not the list, not a
	    filter over the list — the two or three lines that matter now, on the
	    room's own ground with a hairline under them. A step that adds nothing
	    says so in words rather than showing an empty box;
	  · the ground turns indigo. Standing at the stove is another room, and it
	    is the one place in Kamosu where that is true;
	  · THERE IS NO TAB BAR HERE. The layout hides it on this route, so no tap
	    near the bottom of a wet phone can land you in Shopping. The foot of
	    the screen is Back and Next and NOTHING ELSE (#88), so the only thing a
	    wet thumb can reach there is a way through the recipe. The way out is
	    at the top: a back arrow and the recipe's name (#651).

	HOW MUCH ROOM ANY OF IT GETS WAS SETTLED BY MEASUREMENT, NOT BY EYE (#88).
	The screen is used in Safari with its URL bar on show, which leaves about
	700px on the phone this was fitted to — not the 844px a headless browser
	reports. Everything about this room's spacing was re-fitted against the real
	579 Steps of the Crouton export at that height: 53% of Steps naming two
	amounts used to fit without scrolling, and 92% do now. Before moving a
	number here, measure it at 700px against real Steps.

	EVERY CONTROL A WET THUMB MUST HIT IS 48px OF TOUCH. The small ones are
	`quiet-button`s, 40px of ink tapped at 48 (ADR 0044), and every button here
	is one of the shared pieces in the room's colours (`cook-room`, #651).

	WHICH INGREDIENTS THIS STEP USES IS NOT DECIDED HERE. The Core works it out
	from their Readings and serves it as `version.cooking.steps[i].uses` — a
	list of indices into the recipe's own Ingredient Lines — so an agent at the
	MCP door asked to read out the next step names the same amounts this screen
	is showing (ADR 0001). The same is true of the duration a timer is offered
	for and of the Step's conversions, its oven and its amounts: all are read out
	of the Step's own text, server-side, at display time, and none is stored.

	THE AMOUNTS ARE THE WRITTEN LINES, at full size, with the one subordinate
	line beneath them (#49, ADR 0016) — already converted to this cook's
	measures, because `get_recipe` works `measured` out for this reader. A line
	Kamosu could not read carries no subordinate line and is shown whole rather
	than guessed at (ADR 0002); at the Yield as written nothing marks it as
	unread, because a badge that fires sometimes teaches people it fires always.

	HOW MUCH IS BEING COOKED IS ASKED BEFORE A FRESH COOKING STARTS (#109, option
	B, the choice of 23 September 2026). The question stands where the
	amounts and the Step will, and the foot's one button starts the cooking —
	there is no second button for it, and it is no step (#651). Afterwards the Yield on the row
	above the hairline brings the question back, and the foot then reads *Back
	to step N*. Once the amounts are scaled THE SCALED AMOUNT LEADS: the Core's
	subordinate line at full size, the recipe's own line small beneath it. At
	the stove the number to measure is the one that matters, and the written
	line is still there to say what the recipe asked for. A line that could not be
	scaled keeps its own words and says it was not scaled — once a cook has
	asked for twice the recipe, a line left alone is exactly what they need
	telling. A line with no amount (olive oil, salt to taste) had nothing to
	scale and says nothing (#680). The scaled line is the whole line at the new
	amount, `1½ heads of garlic`, so it still says what it is an amount of.
	Kamosu scaled nothing here: every figure came from the Core.
-->
<script lang="ts">
	import { onDestroy, tick as settledDom, untrack } from 'svelte';
	import { demo } from '#lib/demo.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import SheetFrame from '#lib/SheetFrame.svelte';
	import { useRoom } from '#lib/room.svelte.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import type { GetRecipeOutput, StartAttemptOutput } from '#lib/api/catalogue.js';
	import { timer, clock, keepTimer, keptTimer } from './timer.svelte';
	import { chime } from './chime';
	import { wakeLock } from './wake-lock.svelte';
	import {
		seed,
		serialise,
		differs,
		keepDraft,
		readDraft,
		forgetDraft,
		type AsCooked,
		type Line,
	} from './as-cooked.svelte';
	import { useKeeping } from '#lib/offline/outbox.js';
	import { photographCooking, pickedFile } from '#lib/offline/photograph.js';
	import { reach, rereads } from '#lib/offline/device.svelte.js';
	import { navigating, page } from '$app/state';
	import { goto } from '$app/navigation';
	import HowMuch from '#lib/HowMuch.svelte';
	import StepPhoto from '#lib/StepPhoto.svelte';
	import StepWords from '#lib/StepWords.svelte';
	import { fresh } from './fresh';
	import { mistake, putMistakeAway } from '#lib/mistake.svelte.js';
	import {
		caughtUp,
		counts,
		fromSearch,
		isMultiplier,
		said,
		same,
		wholeRecipe,
		type Wanted,
	} from '#lib/how-much.js';
	import { parseAmount } from '#lib/offline/shopping.js';

	interface Props {
		branchId: string;
	}

	let { branchId }: Props = $props();

	/** The first wait before the recipe is read again, and the longest (#674). */
	const RETRY_FIRST_MS = 1_000;
	const RETRY_MOST_MS = 30_000;

	const kamosu = useKamosu();
	const keeping = useKeeping();
	const bell = chime();
	const countdown = timer(Date.now, ringOn);

	/**
	 * A timer that has run out rings again every 20 seconds until it is
	 * tapped, for two minutes: three tones once were missed by a cook at the
	 * sink (round 1 of #681's climb). A kitchen timer keeps on until told.
	 */
	let ringing: ReturnType<typeof setInterval> | undefined;
	function ringOn() {
		bell.ring();
		clearInterval(ringing);
		let again = 0;
		ringing = setInterval(() => {
			again += 1;
			if (!countdown.rung || again > 6) clearInterval(ringing);
			else bell.ring();
		}, 20_000);
	}
	const awake = wakeLock();

	let attempt = $state<StartAttemptOutput | undefined>(undefined);
	let recipe = $state<GetRecipeOutput | undefined>(undefined);
	let failed = $state(false);

	/**
	 * The recipe is read from four places: the cooking opening, a change of
	 * how much, a refresh behind, and a read tried again. Two can be on their
	 * way at once, and the older can land last (#674). Each read is numbered
	 * as it is asked, and an answer older than the one on screen is dropped,
	 * so amounts for an earlier choice never come back over the ones for the
	 * later.
	 */
	let asked = 0;
	/**
	 * The number of the read whose answer is on screen. An answer lands when
	 * it is newer than that, so a newer read that failed never costs the
	 * screen an older one that came back (found in review): the cooking would
	 * otherwise stay on loading when a refresh's read failed as it opened.
	 */
	let shown = 0;
	/** Reads of the recipe on their way now. */
	let reading = $state(0);
	/** Goes up as each read comes back or fails, so the retry looks again. */
	let settled = $state(0);

	/** Read the recipe and show it, unless a newer answer is already shown. */
	async function readRecipe(): Promise<void> {
		const mine = ++asked;
		reading += 1;
		try {
			const read = await kamosu.getRecipe({ branch_id: branchId });
			if (mine > shown) {
				shown = mine;
				recipe = read;
			}
		} finally {
			reading -= 1;
			settled += 1;
		}
	}

	/** A read that failed is tried again below, so its refusal is dropped. */
	function dropRefused(error: unknown) {
		if (!(error instanceof OperationError)) throw error;
	}
	let finished = $state(false);

	// ---- saying you did it differently (#58, ADR 0005) --------------------
	//
	// The recipe as THIS cooking has it, which is the recipe itself until the
	// cook writes on it. It is held here rather than sent on every keystroke:
	// a whole recipe crossing the wire per character typed at a stove would be
	// absurd, and `Done` is a moment the cook already understands. What is sent
	// is the whole state, and the Core decides by fingerprint whether it is a
	// deviation at all.
	let asCooked = $state<AsCooked | undefined>(undefined);
	/** The step in front of the cook has become fields, in place. */
	let writing = $state(false);
	/** The rest of the Ingredient Lines are open beneath this step's own. */
	let wholeList = $state(false);
	/** Written since the last save. */
	let unsaved = $state(false);
	let writeFailed = $state(false);
	let discarding = $state(false);
	/** The two quiet controls of the top row are open under the ⋯ (#222). */
	let more = $state(false);
	/** The cook tapped *Went wrong* and is reading what it means (#119). */
	let readingMistake = $state(false);
	/** Kamosu went wrong and the cook has not put it away yet (#98). */
	const wentWrong = $derived(mistake.made !== undefined);
	let discarded = $state(false);
	/** A photograph that could not be kept, even on the phone. */
	let photoFailed = $state(false);
	/** A photograph taken in a Demo, which will not be kept (#403). */
	let photoInDemo = $state(false);

	/** How much is being cooked is being asked (#109). */
	let asking = $state(false);
	/**
	 * It was asked as the cooking opened, so the foot starts the cooking rather
	 * than going back to a step.
	 */
	let atStart = $state(false);
	/** The last change of how much did not reach even the phone's own keeping. */
	let howMuchFailed = $state(false);

	/**
	 * THE BODY: this step's amounts and then the Step, in ONE scrolling box
	 * (#681). Until #681 the amounts were capped at 45% of the screen and the
	 * Step scrolled in a box of its own, so a busy Step hid ingredient lines
	 * with no sign of them while room stood empty under a short Step, and the
	 * Step itself was cut after four lines (the tomato soup at six servings).
	 * One box takes the room the two need, and one swipe reads the rest.
	 *
	 * `moreBelow` is whether anything is still below its bottom edge, read from
	 * the DOM rather than predicted, because how many lines a sentence takes
	 * depends on the font, the width and the phone (#88). It follows the scroll,
	 * so the fade lifts once the cook has read to the end.
	 */
	let bodyEl = $state<HTMLElement | undefined>(undefined);
	/** The amounts' and the Step's own columns, which scroll apart on a phone turned sideways. */
	let amountsEl = $state<HTMLElement | undefined>(undefined);
	let stepBoxEl = $state<HTMLElement | undefined>(undefined);
	/** The Step's own field while the cook writes. */
	let stepFieldEl = $state<HTMLTextAreaElement | undefined>(undefined);
	let moreBelow = $state(false);
	/**
	 * The next Step goes under this one, not beside it, on this Step. Beside it,
	 * on a tablet on its side, it takes a third of the width, and a long Step
	 * squeezed into the rest ran past the foot: the soup's second step needed a
	 * scroll to reach its own timing (round 1 of #681's climb). Under it, the
	 * Step has the whole width and only the preview runs on below. Set for the
	 * Step it was measured on, and only ever one way within it, so the layout
	 * cannot chase itself.
	 */
	let nextBelow = $state(false);
	/** This Step's amounts did not leave it room in one column. */
	let crowded = $state(false);

	function measureBody() {
		const element = bodyEl;
		// A few pixels of slack: sub-pixel line heights make a body that fits
		// report a pixel of overflow, and a fade over one that fits greys out its
		// last line for nothing.
		const over = element
			? element.scrollHeight - element.scrollTop - element.clientHeight > 8
			: false;
		moreBelow = over;
		// One change a measure, each kept for the rest of the Step, so the
		// layout settles in at most two moves and never chases itself.
		if (over)
			untrack(() => {
				if (room.roomy && nextBeside) nextBelow = true;
				else if (!crowded && !writing && amounts.length >= 3) crowded = true;
			});
	}

	/**
	 * Any tap opens the sound, so a timer taken back after a reload can still
	 * ring: iOS lets a page sound only from inside a tap.
	 */
	function readySound() {
		bell.ready();
	}

	/**
	 * A running timer stops on a SECOND tap (#681). One stray touch from a wet
	 * thumb used to throw away a 25-minute countdown with no way back. The first
	 * tap says what the next one does, for three seconds.
	 */
	let stopArmed = $state(false);
	let disarm: ReturnType<typeof setTimeout> | undefined;

	function tapTimer() {
		clearTimeout(disarm);
		if (countdown.rung || stopArmed) {
			stopArmed = false;
			countdown.clear();
			clearInterval(ringing);
			if (attempt) keepTimer(attempt.id, null);
			return;
		}
		stopArmed = true;
		disarm = setTimeout(() => (stopArmed = false), 3000);
	}

	function startTimer(seconds: number) {
		bell.ready();
		countdown.start(seconds);
		if (attempt) keepTimer(attempt.id, countdown.endsAt);
	}

	/**
	 * The timer this cooking had running when the cook last left the screen,
	 * taken back once its cooking is known.
	 */
	let timerTakenBack = false;
	$effect(() => {
		const id = attempt?.id;
		if (!id || timerTakenBack) return;
		timerTakenBack = true;
		untrack(() => {
			const endsAt = keptTimer(id);
			if (endsAt !== null && countdown.remaining === null) countdown.resume(endsAt);
		});
	});

	/**
	 * When the cook last moved between steps. A second tap that lands just after
	 * Next has turned into Finish cooking is the first tap again, not a decision
	 * to finish (#681: two taps 250 ms apart on the second-last step ended the
	 * cooking).
	 */
	let movedAt = 0;

	function finishOnPurpose() {
		if (Date.now() - movedAt < 500) return;
		void finish();
	}

	/**
	 * Back and Next, once a tap. A thumb that bounces lands two taps on the
	 * same spot: 120 and 300 ms apart each moved the cook two Steps, and only
	 * the small count said so (round 1 of #681's climb). 600 ms also swallowed
	 * a cook's deliberate second tap at 382 ms, with nothing to say it had
	 * (round 2), so the window is a bounce's and no longer.
	 */
	function moveOnPurpose(move: () => void) {
		if (Date.now() - movedAt < 350) return;
		move();
	}

	const room = useRoom();
	const nextId = $props.id();
	/**
	 * The next Step's own element, and how many whole lines of it are shown
	 * (#197). Three under the Step on an upright tablet. Beside the Step it has
	 * a column as tall as the Step's box, and how many lines that is depends on
	 * the window, so it is measured: a line cut through the middle by the edge
	 * of its column reads as a fault.
	 */
	let nextEl = $state<HTMLElement | undefined>(undefined);
	let nextLines = $state(3);

	/**
	 * *Got it*. The card closes with the mistake rather than staying open for
	 * the next one: a later mistake raises the word again, and never a card over
	 * the Step that the cook did not tap for.
	 */
	function mistakeRead() {
		readingMistake = false;
		putMistakeAway();
	}

	/** Tries at starting: Try again on the screen that says it failed counts one more. */
	let tries = $state(0);

	$effect(() => {
		void tries;
		let current = true;
		void (async () => {
			try {
				// Starting IS the Attempt (ADR 0010): opening this screen twice
				// hands back the one already In Progress rather than making a
				// second, so there is no "resume" branch to write here.
				const started = await kamosu.startAttempt({ branch_id: branchId });
				if (!current) return;
				attempt = started;
				await readRecipe();
				if (!current) return;
				// A cooking nothing has happened to yet opens on the question
				// (#109) — already set to what the recipe page was scaled to,
				// where the cook came from one that was (the errands).
				const carried = untrack(() => fromSearch(page.url.searchParams));
				if (fresh(started)) {
					asking = true;
					atStart = true;
					if (carried !== undefined && !same(carried, started.cooking_yield)) await choose(carried);
				} else if (carried !== undefined && !same(carried, started.cooking_yield)) {
					// Sent from a recipe page scaled to another amount, into a
					// cooking already under way at its own. The cooking's amount
					// is a fact about this afternoon and is not overwritten from
					// another screen — but the cook is shown the question rather
					// than left to find the difference in the amounts.
					asking = true;
				}
				// Read once and let go of: left in the address, a reload later in
				// the cooking would offer the recipe page's amount again over the
				// one the cook has since chosen (found in live acceptance, #109).
				// Shallow: a navigation here would run this start a second time,
				// racing the amount just sent. Not once the cook has gone on: in
				// SvelteKit 3 a shallow `goto` cancels a navigation under way, and
				// the address it would tidy is no longer this screen's (#525).
				if (carried !== undefined && current && navigating.type === null)
					void goto(`/cook/${branchId}`, { shallow: true, replace: true, state: page.state });
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			}
		})();
		return () => {
			current = false;
		};
	});

	$effect(() => {
		awake.start();
		return () => awake.stop();
	});

	/**
	 * Whether anything is below the body's bottom edge.
	 *
	 * What it holds is read as a dependency and not just the element: Svelte
	 * reuses the same box from one step to the next, so an effect watching only
	 * the element would measure the first Step and then never look again. What
	 * changes the answer is the sentence, writing mode, how many amounts there
	 * are, a notice, and turning the phone, which `resize` covers. Scrolling is
	 * the box's own `onscroll`.
	 *
	 * Deliberately NOT a ResizeObserver. The screen tests run in jsdom, whose
	 * setup stubs nothing a screen could come to rely on by accident, and a fade
	 * is an enhancement: it must not be the reason this component cannot mount.
	 */
	$effect(() => {
		// read, so the effect re-runs when any of them moves
		void bodyEl;
		void here?.row.text;
		void writing;
		void amounts.length;
		void writeFailed;
		// The next Step moving under this one, which gives the Step its width,
		// and the amounts going to two columns.
		void nextBelow;
		void crowded;
		// A Demo's strip, which takes the foot of the screen once the shell
		// has heard it is one (#407).
		void demo.is;
		measureBody();
		// Once more after the frame, by when the strip's room has been taken.
		const settled = requestAnimationFrame(measureBody);
		// And once the type has arrived: the first Step of a cooking opened cold
		// is laid out in a stand-in face, and the real one is taller.
		let live = true;
		void document.fonts?.ready.then(() => {
			if (live) measureBody();
		});
		window.addEventListener('resize', measureBody);
		return () => {
			live = false;
			cancelAnimationFrame(settled);
			window.removeEventListener('resize', measureBody);
		};
	});

	/**
	 * Every Step opens at its top (#681). The body is one box reused from step
	 * to step, so it kept the last step's scroll, and a long Step arrived with
	 * its first lines already scrolled away.
	 */
	$effect(() => {
		void here?.index;
		untrack(() => {
			for (const box of [bodyEl, amountsEl, stepBoxEl]) if (box) box.scrollTop = 0;
			nextBelow = false;
			crowded = false;
			measureBody();
		});
	});

	$effect(() => {
		const element = nextEl;
		if (!element || !room.roomy || !nextBeside) {
			nextLines = 3;
			return;
		}
		const measure = () => {
			const words = element.querySelector('p');
			if (!words) return;
			const line = parseFloat(getComputedStyle(words).lineHeight);
			const tall = element.clientHeight - (words.offsetTop - element.offsetTop);
			// Where nothing can be measured the three lines stand.
			nextLines = line > 0 && tall > 0 ? Math.max(1, Math.floor(tall / line)) : 3;
		};
		measure();
		// Watched rather than worked out from what might move it. The column is
		// as tall as the Step's box, and that changes with the window, with how
		// many lines the amounts above take, and with a scaled amount gaining a
		// line beneath it. The words inside never change its height, so this
		// cannot chase itself.
		if (typeof ResizeObserver === 'undefined') return;
		const watching = new ResizeObserver(measure);
		watching.observe(element);
		return () => watching.disconnect();
	});

	onDestroy(() => {
		countdown.dispose();
		clearInterval(ringing);
		clearTimeout(disarm);
		clearTimeout(keptFor);
	});

	/**
	 * The recipe again, whenever what the phone showed has been overtaken: the
	 * service worker's refresh behind a kept answer, or the outbox having sent
	 * a change of how much it held while the network was gone (#77). The
	 * scaled amounts are the Core's, so they arrive by reading, never by being
	 * worked out here.
	 */
	const reread = rereads('get_recipe');
	$effect(() => {
		if (reread.count === 0) return;
		// Untracked: the read counts itself in `reading`, which this must not
		// come to depend on and run again for.
		untrack(() => readRecipe().catch(dropRefused));
	});

	// ---- where the cook is standing --------------------------------------

	/**
	 * The Version this Attempt is pinned to, which is not necessarily the head:
	 * an Attempt started from the Thread cooks an older Version, and a recipe
	 * edited mid-cook must not change under the cook's hands (ADR 0005).
	 */
	const version = $derived(
		recipe?.versions.find((each) => each.version_id === attempt?.version_id) ??
			recipe?.versions.at(-1),
	);
	const content = $derived(version?.content);

	/**
	 * The recipe this cooking is actually walking through — the Version, until
	 * the cook writes on it, and their own words from then on. Seeded once the
	 * Attempt and its Version are both in hand, and never re-seeded underneath
	 * somebody who is typing.
	 */
	$effect(() => {
		if (asCooked || !content || !attempt) return;
		// Words written with no network are the cook's until the server has
		// them (#77): the phone keeps them here, because what the server makes
		// of them — which line changed, which was added — is the server's to
		// read (ADR 0019), and it has not read them yet.
		const waiting = keeping.holds(attempt.id);
		if (!waiting) forgetDraft(attempt.id);
		asCooked = (waiting ? readDraft(attempt.id) : undefined) ?? seed(content, attempt.as_cooked);
	});

	/**
	 * The rows a cook actually stands on, in order — Sections are headings over
	 * the list, not places, and a line the cook dropped is not a place either.
	 *
	 * `at` indexes the AS COOKED's steps rather than the Version's, and that is what
	 * `current_step_index` means from #58 onwards: the cooking screen is the As
	 * Cooked once there is one, so a step the cook inserted is a step they stand
	 * on, time and all. Where the cook has written nothing this is the Version line for
	 * line, so nothing about an ordinary cooking moved.
	 */
	const stops = $derived(
		(asCooked?.steps ?? [])
			.map((row, index) => ({ row, index }))
			.filter((each) => each.row.kind === 'step' && !each.row.dropped),
	);

	/**
	 * Where in `stops` the cook is. `current_step_index` is an index into the
	 * whole list and may land on a Section — the stored 0 of a recipe that opens
	 * with one — so the first stop at or after it is where they are standing.
	 * Read rather than corrected: writing on load would make merely opening the
	 * screen an action, and two devices are meant to agree without either
	 * nudging the other (ADR 0010).
	 */
	const position = $derived.by(() => {
		const at = attempt?.current_step_index ?? 0;
		const found = stops.findIndex((stop) => stop.index >= at);
		return found === -1 ? Math.max(0, stops.length - 1) : found;
	});
	const here = $derived(stops[position]);

	/**
	 * A Step is in front of the cook. Only then does the mark stand in for the
	 * card: loading, a failure, a cooking thrown away or finished have no Step
	 * to protect and no top row to carry the mark, so there the card is drawn in
	 * place rather than said only to a screen reader (#119).
	 */
	const onStep = $derived(
		!failed &&
			content !== undefined &&
			attempt !== undefined &&
			here !== undefined &&
			!discarded &&
			!finished,
	);
	const last = $derived(position >= stops.length - 1);

	/**
	 * The Step after this one, shown small where the window has room (#197).
	 * Not on the phone, where the Step needs every line it has. Not while the
	 * cook is writing on this Step either, when the room belongs to the field.
	 * The last Step has none.
	 */
	const comingNext = $derived(room.wide && !writing ? stops[position + 1] : undefined);
	/** It stands beside the Step, where the window is roomy and the Step leaves it room. */
	const nextBeside = $derived(comingNext !== undefined && !nextBelow);

	/** The Section this step falls under, where the recipe has any. */
	const section = $derived.by(() => {
		if (!here) return null;
		const above = (asCooked?.steps ?? []).slice(0, here.index).reverse();
		return above.find((row) => row.kind === 'section' && !row.dropped)?.text ?? null;
	});

	/**
	 * What the Core read out of this Step's own text: the Ingredient Lines it
	 * uses, and the duration it offers as a timer. Deliberately not called a
	 * Reading — that word is taken, and means what Kamosu understood of an
	 * INGREDIENT LINE (GLOSSARY.md).
	 */
	/**
	 * Looked up by where the step CAME FROM in the Version, never by where it
	 * sits now — inserting a step shifts every position after it, and `uses` and
	 * `measured` are the Core's arrays over the Version. A step the cook wrote
	 * has no such reading, and says nothing rather than borrowing its
	 * neighbour's: Kamosu has not read that sentence.
	 */
	const outOfText = $derived(
		here?.row.from === null || here === undefined
			? null
			: (version?.cooking.steps[here.row.from] ?? null),
	);
	/** The recipe's photograph of this Step, if it has one (#110). */
	const stepPhoto = $derived(
		here?.row.from === null || here === undefined
			? null
			: (content?.steps[here.row.from]?.photo ?? null),
	);
	/**
	 * What the Core added to this Step for this cook: its oven and its amounts,
	 * converted and scaled, each drawn straight after what it converts (#150).
	 * A Step the cook rewrote loses them, as a changed Ingredient Line loses
	 * its subordinate line: those figures describe the words the recipe wrote,
	 * not the ones now on screen.
	 */
	const stepConversions = $derived(
		here === undefined || here.row.from === null || wroteStep(here.row)
			? []
			: (version?.measured.steps[here.row.from] ?? []),
	);

	/**
	 * The amounts for this step: the written Ingredient Line at full size and
	 * the one subordinate line beneath it, in the recipe's own order — as this
	 * cooking has them, which is as the recipe has them until somebody writes.
	 *
	 * A line whose words the cook changed loses its subordinate line, because
	 * that reading describes the amount the recipe asked for and not the one
	 * that went in. A line they dropped stays on screen, struck: it was in front
	 * of them a moment ago and vanishing furniture is how a screen loses a cook.
	 */
	/** Where the lines the cook wrote straight after the line at `at` sit. */
	function followers(rows: Line[], at: number): number[] {
		const found = [];
		for (let next = at + 1; next < rows.length; next++) {
			if (rows[next].from !== null || rows[next].kind !== 'ingredient') break;
			found.push(next);
		}
		return found;
	}

	/**
	 * The lines the cook wrote that stand under a line some Step uses, and so
	 * are drawn on that Step and not offered again on every other one.
	 */
	const onAStep = $derived.by(() => {
		const rows = asCooked?.ingredients ?? [];
		const used = new Set((version?.cooking.steps ?? []).flatMap((step) => step?.uses ?? []));
		return new Set(
			rows.flatMap((row, at) =>
				row.from !== null && used.has(row.from) ? followers(rows, at) : [],
			),
		);
	});

	const amounts = $derived(
		(outOfText?.uses ?? []).flatMap((from) => {
			const rows = asCooked?.ingredients ?? [];
			const at = rows.findIndex((row) => row.from === from);
			const row = rows[at];
			if (!row || row.kind !== 'ingredient') return [];
			const written: string | null = content?.ingredients[from]?.text ?? row.text;
			const changed = row.text.trim() !== written.trim();
			const own = {
				at,
				row,
				written,
				changed,
				beneath: changed || row.dropped ? null : (version?.measured.ingredients[from] ?? null),
				/** Whether Kamosu read an amount on the line, so there was one to scale (#680). */
				hasAmount: Boolean(version?.readings?.[from]?.amount),
			};
			// The lines the cook wrote straight after this one are drawn under it.
			// A line rewritten past recognition comes back from the Core as this
			// one dropped and a new one added after it (ADR 0019: a removal and
			// an addition, the old struck beside the new), and Add a line puts
			// its line after the Step's last one, so the Core's own order keeps
			// each on its Step. Both were gone from the Step once the cooking was
			// opened again (rounds 2 and 3 of #681's climb).
			const after = followers(rows, at)
				.filter((next) => writing || rows[next].text.trim() !== '')
				.map((next) => ({
					...own,
					at: next,
					row: rows[next],
					written: null,
					changed: true,
					beneath: null,
					hasAmount: false,
				}));
			return [own, ...after];
		}),
	);

	/**
	 * Whether this cooking's words for a step are not the recipe's — rewritten,
	 * or written from nothing. What the noren stripes, and what puts the bar
	 * down the side of the Step.
	 */
	/**
	 * Whether a line is ticked. Ticking is keyed on the line's index in the
	 * Version, which is what the server stores and what two devices agree on, so
	 * a line the cook added has no index and is never ticked.
	 */
	function isTicked(line: Line): boolean {
		return line.from !== null && ticked.has(line.from);
	}

	function wroteStep(line: Line): boolean {
		if (line.dropped) return false;
		if (line.from === null) return true;
		return line.text.trim() !== (content?.steps[line.from]?.text ?? '').trim();
	}

	/** Every other Ingredient Line, for the steps — most of them — that name none. */
	const otherLines = $derived(
		(asCooked?.ingredients ?? []).flatMap((row, at) => {
			if (row.kind !== 'ingredient') return [];
			if (amounts.some((amount) => amount.at === at)) return [];
			const written = row.from === null ? null : (content?.ingredients[row.from]?.text ?? null);
			return [
				{ at, row, written, changed: written !== null && row.text.trim() !== written.trim() },
			];
		}),
	); /**
	 * The lines the cook added while cooking, drawn under this Step's own with
	 * Add a line, where they were added, rather than at the foot of a list
	 * opened first (round 1 of #681's climb: a grader found Add a line only after
	 * opening The other ingredients and scrolling past twelve rows).
	 */
	const addedLines = $derived(
		otherLines.filter((line) => line.row.from === null && !onAStep.has(line.at)),
	);
	/**
	 * The recipe's own lines this Step does not use, behind The other
	 * ingredients, and the lines the cook wrote on another Step.
	 */
	const restLines = $derived(
		otherLines.filter((line) => line.row.from !== null || onAStep.has(line.at)),
	);

	const ticked = $derived(new Set(attempt?.ticked_ingredients ?? []));

	/**
	 * A busy Step's amounts go in two columns, and on a phone at the ingredient
	 * line's size, so they leave the Step its room (#681). Only where one
	 * column does not leave it: chosen by four lines or more, they squeezed the
	 * bourguignon's four into half columns of small type above a short Step and
	 * 400px of nothing (round 1 of #681's climb). So the Step is drawn in one
	 * column first, and goes to two once it is found running past the foot.
	 * Three lines is the least that two columns can shorten.
	 *
	 * In two columns the lines are 24px apart, not the line size's 23.8: rows
	 * that touch at a fraction of a pixel claimed the same pixel row twice,
	 * and measured as taps overlapping (round 3 of #681's climb).
	 */
	const columns = $derived(crowded && amounts.length >= 3);
	const figure = $derived(
		columns
			? 'text-line leading-[calc(24*var(--type-unit))] sideways:text-panel-figure wide:text-panel-figure wide:leading-figure-far'
			: 'text-panel-figure wide:leading-figure-far',
	);

	/**
	 * The first stop whose Step uses any Ingredient Line. What the tick box is
	 * for is said there and not again: a line repeated over every step of a
	 * cooking was read as filler by every grader of #681's first round.
	 */
	const firstWithAmounts = $derived(
		stops.findIndex(
			(stop) =>
				stop.row.from !== null && (version?.cooking.steps[stop.row.from]?.uses.length ?? 0) > 0,
		),
	);

	// ---- moving, ticking, finishing --------------------------------------

	/**
	 * Every write on this screen is the same one Operation, and every one of
	 * them shows on screen before it is sent: at the stove a tap that waits for
	 * a round trip reads as a tap that did not register, and the cook taps
	 * again. The answer replaces what was laid down optimistically; a refusal
	 * puts the Attempt back exactly as the server has it.
	 */
	async function move(
		optimistic: StartAttemptOutput,
		input: { current_step_index?: number; ticked_ingredients?: number[] },
	) {
		const before = attempt;
		attempt = optimistic;
		try {
			attempt = await kamosu.advanceAttempt({ attempt_id: optimistic.id, ...input });
			moveFailed = false;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			attempt = before;
			// Said, where the tap was: the Step went back, or the tick came
			// off, and nothing told the cook why (round 2 of #681's climb). With
			// no network the outbox holds the move, so this is Kamosu refusing.
			moveFailed = true;
		}
	}
	/** The last move or tick was refused, and put back. */
	let moveFailed = $state(false);

	/**
	 * Back a step, and from the first one back to how much (#651): how much is
	 * no step, but it is where the cooking began, so Back is never greyed out.
	 */
	function back() {
		if (position > 0) return step(position - 1);
		writing = false;
		wholeList = false;
		void save();
		movedAt = Date.now();
		atStart = true;
		asking = true;
		more = false;
	}

	function step(to: number) {
		const stop = stops[to];
		if (!attempt || !stop) return;
		movedAt = Date.now();
		writing = false;
		wholeList = false;
		more = false;
		void save();
		void move({ ...attempt, current_step_index: stop.index }, { current_step_index: stop.index });
	}

	// ---- writing what you did --------------------------------------------
	//
	// Every one of these changes what is held here and nothing else. Nothing reaches the
	// server until `save`, which runs when the cook leaves writing mode or the
	// step — the two moments they have already decided they are done.

	/** Rewrite one line, in either list. */
	function write(list: 'ingredients' | 'steps', at: number, text: string) {
		const row = asCooked?.[list][at];
		if (!row) return;
		row.text = text;
		unsaved = true;
	}

	/**
	 * Take a line out, or put it back. It stays here either way, marked
	 * — so the tap that dropped it is the tap that undoes it, and nothing
	 * disappears from under a wet finger. `serialise` is what leaves it out.
	 */
	function drop(at: number) {
		const row = asCooked?.ingredients[at];
		if (!row) return;
		row.dropped = !row.dropped;
		unsaved = true;
	}

	/** A line the cook added: theirs, from nowhere in the recipe. */
	async function addLine() {
		if (!asCooked) return;
		// After this Step's last line, so it is this Step's when the cooking is
		// opened again; at the foot of the list on a Step that uses none.
		const last = Math.max(-1, ...amounts.map((amount) => amount.at));
		const at = last >= 0 ? last + 1 : asCooked.ingredients.length;
		asCooked.ingredients.splice(at, 0, {
			kind: 'ingredient',
			text: '',
			from: null,
			dropped: false,
		});
		unsaved = true;
		// The new field takes the caret, so the cook types rather than aims again.
		await settledDom();
		document.querySelector<HTMLInputElement>(`input[data-line-at="${at}"]`)?.focus();
	}

	/**
	 * Insert a step before the one the cook is standing on, and leave them
	 * standing on the new one — they are writing down what they have just done,
	 * so that is where they want to be. Inserting before shifts every later step
	 * up by one, and `current_step_index` therefore names the new step without
	 * anything being written: the stored number did not move, the list did.
	 */
	async function insertStep() {
		if (!asCooked || !here) return;
		asCooked.steps.splice(here.index, 0, { kind: 'step', text: '', from: null, dropped: false });
		writing = true;
		unsaved = true;
		// The new Step's own words are what the cook came to write: the whole
		// list closes and the field takes the caret, where it opened far below
		// the list before (round 1 of #681's climb).
		wholeList = false;
		await settledDom();
		stepFieldEl?.focus();
	}

	/**
	 * Send the whole recipe as this cooking has it — or nothing at all, where
	 * the cook has ended up back where they started. The Core decides for real
	 * by fingerprint; this only saves a round trip on the common case.
	 */
	async function save() {
		if (!attempt || !asCooked || !content || !unsaved) return;
		const as_cooked = differs(content, asCooked) ? serialise(content, asCooked) : null;
		try {
			const answer = await kamosu.setAsCooked({ attempt_id: attempt.id, as_cooked });
			// ONLY the As Cooked is taken from the answer. It carries a whole
			// Attempt, including the `current_step_index` as it stood when the
			// request left — and this runs alongside `advance_attempt`, so
			// replacing the Attempt wholesale would throw the cook back a step
			// whenever this one answered second.
			if (attempt) attempt = { ...attempt, as_cooked: answer.as_cooked };
			if (attempt && keeping.holds(attempt.id)) keepDraft(attempt.id, asCooked);
			else if (attempt) forgetDraft(attempt.id);
			unsaved = false;
			writeFailed = false;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			// What they wrote stays on screen. Losing a cook's own words because
			// a network blinked is the one failure this screen must not have.
			writeFailed = true;
		}
	}

	/**
	 * Ticking is keyed on the line's index in the VERSION, which is what the
	 * server stores and what two devices agree on. A line the cook added has no
	 * such index and cannot be ticked — it is a note to themselves about what
	 * went in, not a thing to check off.
	 */
	function tick(at: number | null) {
		if (!attempt || at === null) return;
		const next = new Set(attempt.ticked_ingredients);
		if (next.has(at)) next.delete(at);
		else next.add(at);
		const ticked_ingredients = [...next].sort((a, b) => a - b);
		void move({ ...attempt, ticked_ingredients }, { ticked_ingredients });
	}

	/**
	 * Finishing stops the Attempt being In Progress and nothing more. The
	 * rating and the note are optional and live in the diary (#59, ADR 0010):
	 * a cook who says nothing still cooked, and being asked to file paperwork
	 * with a hot pan in hand is how a screen makes people lie to it.
	 *
	 * It does not navigate anywhere by itself. A screen that jumps somewhere
	 * else the moment a tap lands is a tap the cook did not intend spent in
	 * reverse — they look up and are somewhere they did not ask to be. Both
	 * ways out are offered instead, and neither is pressed for them.
	 */
	/**
	 * Undoing a false start (ADR 0010). Because starting a cook is what makes
	 * the cooking real, opening these steps out of curiosity would otherwise
	 * register as one — so the counterweight is here, at the stove, where the
	 * false start happened, rather than only in the diary a week later.
	 *
	 * It is behind a confirmation because it is the one control on this screen
	 * that destroys something, and a wet thumb must not be able to spend it.
	 */
	async function discard() {
		// Once: Enter confirms this sheet as well as the button (#196), and a
		// second press must not ask for an Attempt that is already going.
		if (!attempt || throwingAway) return;
		throwingAway = true;
		try {
			await kamosu.deleteAttempt({ attempt_id: attempt.id });
			forgetDraft(attempt.id);
			keepTimer(attempt.id, null);
			discarding = false;
			discarded = true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			// The cooking is still here and still the cook's, so the screen stays
			// and says the throwing away did not happen. It said "Could not start
			// cooking" over everything (round 2 of #681's climb).
			discarding = false;
			moveFailed = true;
		} finally {
			throwingAway = false;
		}
	}
	let throwingAway = false;

	/**
	 * A picture of the cooking, taken at the stove (#77, option A). It is kept
	 * on the phone before anything else, so it is never lost to a network that
	 * blinked, and put on the cooking beside the others. Only the pictures are
	 * taken from the answer: the rest of the Attempt it carries left before the
	 * step the cook may have moved to since.
	 */
	/** A photo was kept a moment ago, and the screen says so for three seconds. */
	let photoKept = $state(false);
	let keptFor: ReturnType<typeof setTimeout> | undefined;

	async function takePhoto(event: Event) {
		const picture = pickedFile(event);
		if (!attempt || !picture) return;
		const id = attempt.id;
		try {
			const taken = await photographCooking(kamosu, keeping, id, picture);
			if (attempt?.id === id) attempt = { ...attempt, photographs: taken.photographs };
			photoFailed = false;
			photoInDemo = taken.inADemo;
			// Said where the cook is looking, for a moment (round 1 of #681's
			// climb): More closed and the screen looked as before, so a photo
			// that worked and one that never left the camera looked the same.
			if (!taken.inADemo) {
				photoKept = true;
				clearTimeout(keptFor);
				keptFor = setTimeout(() => (photoKept = false), 3_000);
			}
		} catch (error) {
			if (!(error instanceof OperationError) && !(error instanceof DOMException)) throw error;
			photoFailed = true;
		}
	}

	async function finish() {
		if (!attempt) return;
		writing = false;
		await save();
		try {
			await kamosu.finishAttempt({ attempt_id: attempt.id });
			keepTimer(attempt.id, null);
			finished = true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			moveFailed = true;
		}
	}

	/**
	 * HOW MUCH is being cooked — the Attempt's own Yield or multiplier where one
	 * was set, and otherwise the recipe as written: its Yield, or *As written*
	 * for the third of the library that never said what it makes (#109). A
	 * multiple of the whole recipe is said in words, *Twice the recipe*, where
	 * the audit found `×2` needed decoding (#651, C3).
	 *
	 * The Yield stands beside the amounts because every amount on this screen
	 * was scaled to it: dropping it would leave `800 ml` on screen with nothing
	 * saying 800 ml of what quantity of dish. The recipe's name is at the top
	 * since #651. Built as one string so the spacing cannot depend on how a
	 * formatter happened to break the lines.
	 */
	const written = $derived(content?.yield ?? null);
	const wanted = $derived(attempt?.cooking_yield ?? null);
	const howMuch = $derived(
		wanted && isMultiplier(wanted)
			? (wholeRecipe(parseAmount(wanted.amount)) ?? `×${wanted.amount}`)
			: (said(wanted, version?.yield_nouns) ??
					said(written, version?.yield_nouns) ??
					m.how_much_as_written()),
	);

	/**
	 * Whether the amounts on screen were scaled to what is being cooked. They
	 * are not while a change made with no network waits to be sent (#77): the
	 * phone holds the choice, and only the Core can scale.
	 */
	// `?? null`: a recipe the phone kept before #109 has no `scaled_to` at all,
	// and was as written.
	const amountsCaughtUp = $derived(
		version ? caughtUp(version.scaled_to ?? null, wanted, written) : true,
	);
	/** The amounts on screen are not the recipe's as written. */
	const scaled = $derived(amountsCaughtUp && (version?.scaled_to ?? null) !== null);

	/**
	 * The cook chose an amount other than the recipe's own. Only then does the
	 * amount stand above the Step: a choice the cook made stays in sight while it
	 * holds and comes off with one tap (U12), and the recipe as written is no
	 * choice to show. How much is otherwise asked once, before the first Step
	 * (#109), and Back from the first Step still reaches it (#681).
	 */
	const choseAmount = $derived(
		wanted !== null &&
			!same(wanted, written) &&
			!(isMultiplier(wanted) && parseAmount(wanted.amount) === 1),
	);

	/**
	 * Say how much is being cooked. It is shown at once and sent like every
	 * other move on this screen; the amounts follow when the recipe is read
	 * again, scaled by the Core. It stores nothing on the recipe and mints no
	 * Version (ADR 0002): it is a fact about this afternoon.
	 */
	async function choose(chosen: Wanted) {
		if (!attempt) return;
		const before = attempt;
		attempt = { ...attempt, cooking_yield: chosen };
		try {
			const answer = await kamosu.advanceAttempt({ attempt_id: before.id, cooking_yield: chosen });
			// Only the Yield: the rest of the answer left before any step the
			// cook may have taken since (`save` has the same rule).
			if (attempt) attempt = { ...attempt, cooking_yield: answer.cooking_yield };
			howMuchFailed = false;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			if (attempt) attempt = { ...attempt, cooking_yield: before.cooking_yield };
			howMuchFailed = true;
			return;
		}
		await readRecipe().catch(dropRefused);
	}

	/**
	 * Read the recipe again while the amounts on screen are not scaled to what
	 * is being cooked and Kamosu can be reached (#674): the read after a change
	 * failed, or came back before the change was applied, or the outbox held
	 * the change and sent it with no service worker to tell the page. Each try
	 * waits twice as long as the one before, up to half a minute, and none
	 * starts while a read is on its way. Out of reach, nothing is asked: the
	 * line says the amounts wait, and the outbox tells the page once it sends.
	 */
	let missed = 0;
	$effect(() => {
		void settled;
		if (!version || amountsCaughtUp || reading > 0 || !reach.server) return;
		const wait = Math.min(RETRY_FIRST_MS * 2 ** missed, RETRY_MOST_MS);
		const retry = setTimeout(() => {
			missed += 1;
			readRecipe().catch(dropRefused);
		}, wait);
		return () => clearTimeout(retry);
	});
	$effect(() => {
		if (amountsCaughtUp) missed = 0;
	});

	// ---- the timer --------------------------------------------------------

	const duration = $derived(outOfText?.timer_seconds ?? null);
	/**
	 * The duration as a cook reads it before starting it — `45 min`, `1 h 30 min`,
	 * `30 s`. Whole units only: an offer of `7 min 0 s` is noise, and the number
	 * on the button is there to be recognised at arm's length.
	 */
	const offer = $derived.by(() => {
		if (duration === null) return null;
		if (duration < 60) return m.cook_seconds({ seconds: duration });
		const wholeMinutes = Math.round(duration / 60);
		if (wholeMinutes < 60) return m.cook_minutes({ minutes: wholeMinutes });
		const hours = Math.floor(wholeMinutes / 60);
		const minutes = wholeMinutes % 60;
		return minutes === 0 ? m.cook_hours({ hours }) : m.cook_hours_minutes({ hours, minutes });
	});
</script>

<svelte:window
	onpointerdown={readySound}
	onkeydown={(event) => {
		if (event.key === 'Escape' && more) more = false;
	}}
/>

<svelte:head>
	<title>{content ? m.cook_tab_title({ title: content.title }) : m.app_name()}</title>
</svelte:head>

<!-- Stops above a Demo's strip, which stands under Back and Start cooking (#407). -->
<div
	class="fixed inset-x-0 top-0 above-demo-strip z-30 flex cook-room flex-col bg-cook-ground px-gutter pt-safe pb-safe text-cook-ink cook-far wide:px-[var(--cook-inset)]"
>
	<!--
		Kamosu went wrong, said aloud (#119). The root layout keeps #98's card off
		this screen, so this is how a screen reader hears it, whatever the screen is
		showing. It is always here and only its words come and go, because an
		alert region added along with its words is one a screen reader may miss.
	-->
	<p class="sr-only" role="alert">{wentWrong ? m.wrong_title() : ''}</p>

	{#snippet wrongCard()}
		<h2 class="font-display text-line font-semibold text-cook-ink">{m.wrong_title()}</h2>
		<p class="pt-1 text-body text-cook-ink">{m.wrong_body()}</p>
	{/snippet}

	{#if wentWrong && !onStep}
		<section
			class="mt-4 shrink-0 border-t-3 border-t-support bg-cook-panel p-4"
			aria-label={m.wrong_title()}
		>
			{@render wrongCard()}
			<!--
				Thrown away and finished carry a way back of their own. Loading,
				failing and a recipe with no steps carry none, and after a mistake
				the loading may never end, so there the card's one action is the way
				out rather than *Got it*, which would leave the cook with nothing.
			-->
			{#if discarded || finished}
				<button type="button" class="mt-4 second-button w-full" onclick={mistakeRead}>
					{m.notice_got_it()}
				</button>
			{:else}
				<a href="/recipes/{branchId}" class="mt-4 main-button w-full">
					{m.cook_back_to_recipe()}
				</a>
			{/if}
		</section>
	{/if}
	{#if failed}
		<!-- With a way on from here: again, or back to the recipe (round 2 of
		     #681's climb found the bare sentence a dead end). -->
		<div class="flex min-h-0 flex-1 flex-col justify-center gap-4">
			<p class="text-body text-cook-soft" role="alert">{m.cook_failed()}</p>
			<button
				type="button"
				class="main-button w-full"
				onclick={() => {
					failed = false;
					tries += 1;
				}}
			>
				{m.cook_failed_again()}
			</button>
			<a href="/recipes/{branchId}" class="second-button w-full">{m.cook_back_to_recipe()}</a>
		</div>
	{:else if !content || !attempt}
		<p class="pt-8 text-body text-cook-soft">{m.loading()}</p>
	{:else if !here}
		<p class="pt-8 text-body text-cook-soft">{m.cook_no_steps()}</p>
	{:else if discarded}
		<div class="flex min-h-0 flex-1 flex-col justify-center gap-4">
			<p class="font-display text-title font-semibold" role="status">{m.cook_thrown_away()}</p>
			<a href="/recipes/{branchId}" class="main-button w-full">{m.cook_back_to_recipe()}</a>
		</div>
	{:else if finished}
		<!--
			Cooked. The cooking is already real and was from the moment it
			started (ADR 0010); this only stopped it being In Progress. Both
			ways out, and no rating asked for at the stove.
		-->
		<div class="flex min-h-0 flex-1 flex-col justify-center gap-4">
			<p class="font-display text-title font-semibold" role="status">{m.cook_finished()}</p>
			<a href="/cooked" class="main-button w-full">{m.cook_see_diary()}</a>
			<a href="/recipes/{branchId}" class="second-button w-full">{m.cook_back_to_recipe()}</a>
		</div>
	{:else}
		<!--
			THE TOP LINE: the way out, and the recipe's name (#651, option A,
			chosen on the prototype on 20 November 2026). A back arrow and the
			name on one line, the two one link to the recipe page, as a phone's
			own screens leave. The audit of 19 November 2026 found the screen had
			no visible way out and did not say what was cooking (C1): the way out
			was a block reading *Pause*, which none of its three evaluators read as
			one. Leaving still keeps the cooking In Progress (ADR 0010): the recipe
			page's Start cooking hands back this one, on the step it was on.

			#88 took the name off this screen when it shared the session's row and
			cut to `Katsu Curry (Japanese Curry with Chicken C…` at 430px. Here it
			has the row to itself but for More, and a long name is still cut with
			"…", as the chosen page drew it: the name says which recipe, and the
			Step says the rest.
		-->
		<header class="relative flex shrink-0 items-center gap-2 pt-1 sideways:pt-0">
			<a
				href="/recipes/{branchId}"
				class="-ml-2 flex min-h-12 min-w-0 flex-1 items-center gap-[6px] rounded-sm pr-2 text-cook-ink"
				aria-label={m.cook_leave_for_now({ title: content.title })}
			>
				<svg
					viewBox="0 0 24 24"
					aria-hidden="true"
					class="h-[26px] w-[26px] shrink-0"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"><path d="M15 5l-7 7 7 7" /></svg
				>
				<!-- A photo kept is said here for three seconds, in the recipe's name's
				     place: over nothing the cook reads at that moment, and moving
				     nothing. Above Back and Next it covered the Step's last lines, and
				     at the top right the step count (rounds 2 and 3 of #681's climb). -->
				{#if photoKept}
					<span class="truncate font-display text-line font-semibold" aria-hidden="true">
						✓ {m.cook_photo_kept()}
					</span>
				{:else}
					<span class="truncate font-display text-line font-semibold">{content.title}</span>
				{/if}
			</a>
			<p class="sr-only" role="status">{photoKept ? m.cook_photo_kept() : ''}</p>
			<!--
				KAMOSU WENT WRONG, in two words (#119, option B, chosen 23
				September 2026). Everywhere else #98's card says it at the top of the
				page. Here the card would push the Step down the screen and put *Got
				it* under a wet thumb, so this row carries a beni mark and two words
				instead, and the card is one deliberate tap away. It stands before
				More, which keeps the row's right end (#222).

				A thumb that lands on it by accident opens a card and nothing else.
			-->
			{#if wentWrong}
				<button
					type="button"
					class="quiet-button shrink-0"
					aria-label={m.wrong_title()}
					aria-haspopup="dialog"
					onclick={() => {
						discarding = false;
						readingMistake = true;
					}}
				>
					<i class="h-2 w-2 shrink-0 rounded-sm bg-support" aria-hidden="true"></i>
					{m.cook_went_wrong()}
				</button>
			{/if}
			<!--
				MORE, the rarely needed controls (#222, #681): a photograph of the
				cooking, noting a change, the screen's sleep and the false start, a row
				each. A ⋯ with no word was found unlabelled by the audit (U8), so it
				carries *More* since #651. It opens OVER the Step, from the top row,
				rather than pushing the Step down, and a tap anywhere else closes it
				(#681).
			-->
			<button
				type="button"
				class="quiet-button shrink-0"
				aria-expanded={more}
				onclick={() => (more = !more)}
			>
				<svg
					viewBox="0 0 24 24"
					aria-hidden="true"
					class="h-[18px] w-[18px]"
					fill="none"
					stroke="currentColor"
					stroke-width="2.5"
					stroke-linecap="round"><path d="M5 12h.01M12 12h.01M19 12h.01" /></svg
				>
				{m.cook_more()}
			</button>
			{#if more}
				<!--
					The rest of the screen, while More is open: a tap there closes More
					and does nothing else. Closing on the tap and letting it carry on
					ticked the line under the thumb, or would have moved the Step
					(round 2 of #681's climb). Escape closes it from a keyboard.
				-->
				<button
					type="button"
					class="fixed inset-0 z-10 cursor-default bg-transparent"
					tabindex="-1"
					aria-hidden="true"
					onclick={() => (more = false)}
				></button>
				<div
					class="absolute top-full right-0 z-20 mt-1 grid w-[min(340px,100%)] divide-y divide-cook-line rounded-sm border border-cook-line bg-cook-panel px-3"
				>
					<!--
						A PICTURE OF THE COOKING, at the stove where the dish is (#77). It
						opens the phone's camera, the cook comes back to the step they
						were on, and the words count what has been taken.

						A label around the file picker rather than a button that clicks
						one: a label is the one way iOS Safari always opens a picker, and
						the picker inside stays reachable for a keyboard.
					-->
					<label class="flex min-h-12 cursor-pointer items-center gap-3 text-body text-cook-ink">
						<svg
							viewBox="0 0 24 24"
							aria-hidden="true"
							class="h-[18px] w-[18px] shrink-0"
							fill="none"
							stroke="currentColor"
							stroke-width="1.8"
							stroke-linecap="round"
							stroke-linejoin="round"
							><path d="M4 8h3l2-3h6l2 3h3v11H4z" /><circle cx="12" cy="13" r="3.5" /></svg
						>
						{attempt.photographs.length > 0
							? m.cook_photo_count({ count: attempt.photographs.length })
							: m.cook_photo()}
						<input
							type="file"
							accept="image/*"
							capture="environment"
							class="sr-only"
							aria-label={m.cook_photo_take()}
							onchange={(event) => {
								void takePhoto(event);
								more = false;
							}}
						/>
					</label>
					<!--
						THE WHOLE of how a cook says *I did it differently* (#58). It
						opens no sheet and covers nothing: the amounts and the Step become
						fields where they already stand, so the cook never leaves the step
						they are on. This was chosen over a sheet on 4 September 2026; the
						reasoning is on #58.
					-->
					<button
						type="button"
						class="flex min-h-12 items-center gap-3 text-left text-body text-cook-ink"
						onclick={() => {
							more = false;
							writing = true;
						}}
					>
						<svg
							viewBox="0 0 24 24"
							aria-hidden="true"
							class="h-[18px] w-[18px] shrink-0"
							fill="none"
							stroke="currentColor"
							stroke-width="1.8"
							stroke-linecap="round"
							stroke-linejoin="round"><path d="M5 4h10l4 4v12H5zM9 12h6M9 16h4" /></svg
						>
						{m.cook_note_a_change()}
					</button>
					<!--
						HOW MUCH, asked again from any Step (round 1 of #681's climb). A
						cooking as written shows no amount above the Step, so the one way
						back to the question was Back to before the first Step: six taps
						from the sixth.
					-->
					<button
						type="button"
						class="flex min-h-12 items-center gap-3 text-left text-body text-cook-ink"
						onclick={() => {
							more = false;
							atStart = false;
							asking = true;
						}}
					>
						<svg
							viewBox="0 0 24 24"
							aria-hidden="true"
							class="h-[18px] w-[18px] shrink-0"
							fill="none"
							stroke="currentColor"
							stroke-width="1.8"
							stroke-linecap="round"
							stroke-linejoin="round"><path d="M4 20h4L19 9l-4-4L4 16v4zM14 6l4 4" /></svg
						>
						{m.cook_change_how_much()}
					</button>
					<!--
						The Wake Lock, and its off switch. The real API and nothing else — a
						browser without it says the screen may sleep, which is true, rather
						than a hidden video pretending otherwise.
					-->
					<button
						type="button"
						class="flex min-h-12 items-center justify-between gap-3 text-left text-body text-cook-ink disabled:opacity-60"
						disabled={!awake.available}
						aria-pressed={awake.wanted}
						onclick={() => awake.toggle()}
					>
						{awake.held ? m.cook_awake_on() : m.cook_awake_off()}
						<span class="cook-switch" class:cook-switch-on={awake.held} aria-hidden="true"></span>
					</button>
					<!-- The one control here that loses something: last, apart, in beni (U11). -->
					<button
						type="button"
						class="min-h-12 text-left text-body text-support"
						onclick={() => {
							more = false;
							readingMistake = false;
							discarding = true;
						}}
					>
						{m.cook_false_start()}
					</button>
				</div>
			{/if}
		</header>

		{#if !asking}
			<!--
				The noren: one hanging strip per step, the one you are on lit. It is
				a progress bar that never needs a number read off it, which is what
				a glance from two steps away buys. Where you are is said in words at
				its end since #651, where it was `1/5` on the row above: the audit
				read the number as a fraction (C2). Neither shows while how much is
				asked, which is no step.
			-->
			<div
				class="flex shrink-0 items-center gap-3 pt-2 pb-3 wide:pb-4 sideways:pt-1 sideways:pb-[6px]"
			>
				<div class="flex flex-1 gap-1" aria-hidden="true">
					{#each stops as stop, index (stop.index)}
						<!--
							A strip is STRIPED where this cooking's words are not the
							recipe's — a step rewritten, or one the cook inserted. It costs
							the noren no height and no number to say so, which is the whole
							of why the noren is shaped the way it is.
						-->
						<i
							style="height: var(--noren-h){wroteStep(stop.row)
								? '; background-image: repeating-linear-gradient(135deg, var(--color-cook-accent) 0 3px, transparent 3px 6px)'
								: ''}"
							class="flex-1 rounded-sm
								{index < position ? 'bg-cook-accent opacity-45' : ''}
								{index === position ? 'bg-cook-accent' : ''}
								{index > position ? 'bg-cook-rule' : ''}"
						></i>
					{/each}
				</div>
				<span class="shrink-0 text-step-reading text-cook-ink">
					{m.cook_step_of({ n: position + 1, total: stops.length })}
				</span>
			</div>
		{/if}

		{#if section && !asking}
			<p class="shrink-0 pb-2 font-display text-body font-semibold text-cook-accent">{section}</p>
		{/if}

		{#if asking}
			<!--
				HOW MUCH ARE YOU MAKING (#109), in the place the amounts and the
				Step will stand. Not a step (#651, C2): no noren, no count, and its
				heading is the question itself. The foot's one button is what leaves
				it — Start cooking as the cooking opens, Back to step N afterwards —
				so there is no button in here to confuse with it.
			-->
			<div class="flex min-h-0 flex-1 flex-col justify-center overflow-y-auto pt-3">
				<HowMuch
					{written}
					{wanted}
					nouns={version?.yield_nouns}
					room="cook"
					onchoose={(chosen) => void choose(chosen)}
				/>
				{#if counts(written)}
					<p class="max-w-[480px] pt-3 text-step-reading text-cook-soft">
						{m.how_much_amounts_follow()}
					</p>
				{/if}
				{#if howMuchFailed}
					<p class="pt-2 text-step-reading text-cook-ink" role="alert">{m.how_much_failed()}</p>
				{/if}
			</div>
		{:else}
			<!--
				THE ROW ABOVE THE BODY, and only when it has something on it (#681).
				It holds what a cook watches or set themselves: the timer, the amount
				they chose where it is not the recipe's own, and Done while they
				write. Photo and Note a change went under More, which they are rarely
				reached for at the stove (U5), and the row no longer wraps to two
				lines between the amounts and the Step.

				THE TIMER LIVES HERE, outside the scrolling body: a RUNNING timer
				that scrolls out of sight is the one thing a timer must never do
				(#88). It keeps its width while it runs, so nothing beside it jumps.
				Once the time is up it fills its row, filled and in the body's size:
				the same small chip was the one sign across a kitchen (round 2 of
				#681's climb).
			-->
			{#if writing || countdown.remaining !== null || (duration !== null && offer) || choseAmount}
				<div class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-2 pb-3 sideways:pb-2">
					<!--
						The timer. One tap, nothing typed, nothing stored — and it is the
						cook's rather than the step's, so it follows them forward. While
						one runs it stands where the offer was, filled: two timers at once
						is a thing to keep track of, and this screen exists to stop the
						cook keeping track of things. A running one stops on a second tap
						(#681); one that has rung goes on the first.
					-->
					{#if countdown.remaining !== null}
						<button
							type="button"
							class="quiet-button min-w-[128px] shrink-0 justify-center border-cook-accent bg-cook-accent font-semibold text-cook-on-accent tabular-nums
								{countdown.rung ? 'flex-1 text-body' : ''}"
							onclick={tapTimer}
						>
							{@render clockIcon()}
							<!-- The digits at the amounts' figure size, read across the
							     kitchen; they were the chip's 15px (round 1 of #681's
							     climb). Words stay the chip's size, so the chip does not
							     outgrow its row. -->
							<span
								aria-hidden="true"
								class={countdown.rung || stopArmed ? '' : 'text-panel-figure leading-none'}
							>
								{countdown.rung
									? m.cook_timer_done()
									: stopArmed
										? m.cook_timer_tap_again()
										: clock(countdown.remaining)}
							</span>
							<span class="sr-only">
								{countdown.rung
									? m.cook_timer_done()
									: m.cook_timer_running({ clock: clock(countdown.remaining) })}
								— {stopArmed ? m.cook_timer_tap_again() : m.cook_timer_stop()}
							</span>
						</button>
					{:else if !writing && duration !== null && offer}
						<button
							type="button"
							class="quiet-button min-w-[128px] shrink-0 justify-center border-cook-accent font-semibold"
							aria-label={m.cook_timer_start({ duration: offer })}
							onclick={() => startTimer(duration)}
						>
							{@render clockIcon()}
							{m.cook_timer_offer({ duration: offer })}
						</button>
					{/if}
					<!--
						HOW MUCH is being cooked, where the cook chose it: every figure
						below was scaled to this number, and it is also what asks again
						(#109).
					-->
					{#if choseAmount && !writing}
						<!-- At the far end, where a thumb aiming at the timer cannot catch it,
						     and where it stands whether a Step has a timer or not (round 1 of
						     #681's climb). -->
						<button
							type="button"
							class="quiet-button ms-auto shrink-0"
							aria-label={m.cook_how_much_is({ amount: howMuch })}
							onclick={() => {
								atStart = false;
								asking = true;
								more = false;
							}}
						>
							<svg
								viewBox="0 0 24 24"
								aria-hidden="true"
								class="h-[18px] w-[18px] shrink-0"
								fill="none"
								stroke="currentColor"
								stroke-width="1.8"
								stroke-linecap="round"
								stroke-linejoin="round"><path d="M4 20h4L19 9l-4-4L4 16v4zM14 6l4 4" /></svg
							>
							{howMuch}
						</button>
					{/if}
					{#if writing}
						<button
							type="button"
							class="ms-auto main-button min-w-[120px]"
							onclick={async () => {
								void save();
								writing = false;
								// The Step again from its top, as on arriving at it: it came
								// back scrolled down the fields' length (round 2 of #681's
								// climb).
								await settledDom();
								for (const box of [bodyEl, amountsEl, stepBoxEl]) if (box) box.scrollTop = 0;
							}}
						>
							{m.cook_writing_done()}
						</button>
					{/if}
				</div>
			{/if}

			<!--
				THE BODY: the amounts for this step, then the Step, in one box that
				scrolls as one (#681). The Step never moves above its amounts (#681,
				the maintainer's rule). A phone turned sideways (#222, option 2 of the
				audit of 9 October 2026) puts the amounts on the left and the Step on
				the right, each scrolling in its own column, because at 390 tall there
				is no room for one under the other.
			-->
			<div class="relative flex min-h-0 flex-1 flex-col">
				<div
					bind:this={bodyEl}
					onscroll={measureBody}
					class="min-h-0 flex-1 overflow-y-auto sideways:overflow-hidden"
				>
					<div
						class="flex min-h-full flex-col sideways:grid sideways:h-full sideways:grid-cols-[2fr_3fr] sideways:grid-rows-[minmax(0,1fr)] sideways:gap-x-4"
					>
						<div bind:this={amountsEl} class="shrink-0 sideways:min-h-0 sideways:overflow-y-auto">
							<!--
								A save that did not land. What the cook wrote is still on screen and
								still held here, and the next `Done` sends it again — losing
								somebody's own words because a network blinked is the one failure
								this screen must not have.
							-->
							{#if writeFailed}
								<p class="pb-2 text-step-reading text-cook-soft" role="alert">
									{m.cook_write_failed()}
								</p>
							{/if}
							{#if photoFailed}
								<p class="pb-2 text-step-reading text-cook-soft" role="alert">
									{room.wide ? m.cook_photo_failed_device() : m.cook_photo_failed()}
								</p>
							{:else if photoInDemo}
								<p class="pb-2 text-step-reading text-cook-soft" role="status">
									{m.cook_photo_demo()}
								</p>
							{/if}
							<!--
								A change of how much made with no network (#77): the phone holds
								it and the Core has not scaled it yet, so the amounts below are the
								ones the phone had, and say nothing beneath them rather than a
								figure for the wrong amount. They are drawn soft until the new
								ones come, so the old figures do not pass for the chosen amount's.
								With Kamosu in reach the wait is the moment the Core takes to
								answer, and the line says so rather than blaming the network
								(round 1 of #681's climb).
							-->
							{#if !amountsCaughtUp}
								<p class="pb-2 text-step-reading text-cook-soft" role="status">
									{reach.server
										? m.cook_scaling_working({ amount: howMuch })
										: m.cook_scaling_waits({ amount: howMuch })}
								</p>
							{/if}
							{#if writing}
								<!--
									The amounts, as fields, in the place they already occupied and at
									the size they already had. The tick boxes go while writing: a wet
									thumb aiming at a field must not be able to tick a line by
									missing it. What writing is for is said once above them (#651).
								-->
								<p class="pb-[6px] text-step-reading text-cook-soft">{m.cook_writing()}</p>
								<!--
									Cooking another amount, the fields hold the recipe's own lines,
									which is what an As Cooked changes. Said, so the cook knows which
									amount they are writing over (round 1 of #681's climb).
								-->
								{#if scaled}
									<p class="pb-[6px] text-step-reading text-cook-soft">
										{said(written)
											? m.cook_writing_recipe_for({ amount: said(written) ?? '' })
											: m.cook_writing_recipe_own()}
									</p>
								{/if}
								{#if amounts.length > 0}
									<ul class="flex flex-col gap-2">
										{#each amounts as amount (amount.at)}
											<li class="flex items-center gap-2">
												<input
													data-line-at={amount.at}
													value={amount.row.text}
													disabled={amount.row.dropped}
													placeholder={amount.row.from === null ? m.cook_added_line() : undefined}
													oninput={(event) =>
														write('ingredients', amount.at, event.currentTarget.value)}
													class="min-h-12 min-w-0 flex-1 rounded-sm border border-cook-accent bg-transparent px-2 py-1 font-display text-panel-figure font-semibold text-cook-ink disabled:line-through disabled:opacity-45"
												/>
												<button
													type="button"
													class="quiet-button shrink-0"
													onclick={() => drop(amount.at)}
												>
													{amount.row.dropped ? m.cook_keep_line() : m.cook_drop_line()}
												</button>
											</li>
											<!--
												The recipe's own line, offered back on a tap. It lives here,
												inside writing mode, where *what did it say?* is the question
												being asked — and not on the screen a cook reads at the stove.
											-->
											{#if amount.changed && !amount.row.dropped && amount.written !== null}
												{@const recipeLine = amount.written}
												<li>
													<button
														type="button"
														class="min-h-12 text-left text-step-reading text-cook-soft line-through"
														onclick={() => write('ingredients', amount.at, recipeLine)}
													>
														{amount.written}
														<span class="sr-only">— {m.cook_as_written()}</span>
													</button>
												</li>
											{/if}
										{/each}
									</ul>
								{/if}

								<!--
									A LINE THE COOK ADDS, where they are: under this Step's own lines,
									from the moment Note a change opens (round 1 of #681's climb).
								-->
								{#if addedLines.length > 0}
									<ul class="mt-2 flex flex-col gap-2">
										{#each addedLines as line (line.at)}
											<li class="flex items-center gap-2">
												<input
													data-line-at={line.at}
													value={line.row.text}
													disabled={line.row.dropped}
													placeholder={m.cook_added_line()}
													oninput={(event) =>
														write('ingredients', line.at, event.currentTarget.value)}
													class="min-h-12 min-w-0 flex-1 rounded-sm border border-cook-accent bg-transparent px-2 py-1 font-display text-panel-figure font-semibold text-cook-ink disabled:line-through disabled:opacity-45"
												/>
												<button
													type="button"
													class="quiet-button shrink-0"
													onclick={() => drop(line.at)}
												>
													{line.row.dropped ? m.cook_keep_line() : m.cook_drop_line()}
												</button>
											</li>
										{/each}
									</ul>
								{/if}
								<button type="button" class="quiet-button mt-3" onclick={addLine}>
									{m.cook_add_a_line()}
								</button>

								<!--
									THE REST OF THE LIST. Across the real corpus a step names an
									Ingredient Line only 42% of the time — 2 steps of 11 on Dan Dan
									Noodles — so on most steps the panel above is empty and the salt
									this cook actually used less of is not in it. A cook found that
									standing in the room, and both this and adding a line answer it:
									the whole list opens in the same body, which scrolls, so the
									screen still never hands the cook a second surface to be in.
								-->
								<div class="mt-2 border-t border-cook-rule">
									<button
										type="button"
										class="flex min-h-12 w-full items-center justify-between text-body text-cook-ink"
										aria-expanded={wholeList}
										onclick={() => (wholeList = !wholeList)}
									>
										<span>{wholeList ? m.cook_every_line() : m.cook_other_lines()}</span>
										<span>{wholeList ? '−' : `+${restLines.length}`}</span>
									</button>
								</div>

								{#if wholeList}
									<ul class="flex flex-col gap-2">
										{#each restLines as line (line.at)}
											<li class="flex items-center gap-2">
												<input
													value={line.row.text}
													disabled={line.row.dropped}
													oninput={(event) =>
														write('ingredients', line.at, event.currentTarget.value)}
													class="min-h-12 min-w-0 flex-1 rounded-sm border-b border-cook-rule bg-transparent py-2 font-display text-body text-cook-ink disabled:line-through disabled:opacity-45"
												/>
												<button
													type="button"
													class="quiet-button shrink-0"
													onclick={() => drop(line.at)}
												>
													{line.row.dropped ? m.cook_keep_line() : m.cook_drop_line()}
												</button>
											</li>
										{/each}
									</ul>
								{/if}
							{:else if amounts.length > 0}
								<!--
									WHAT THE BOX IS FOR is said above it (#651, C6): the audit found
									a small faint square beside a line, gathered, prepared or used.
									It is said on the first step that has amounts and not again
									(#681). A step that uses no line shows no panel at all: the
									Step is the whole of it.
								-->
								{#if position === firstWithAmounts}
									<p class="pb-[6px] text-step-reading text-cook-soft">{m.cook_tick_each()}</p>
								{/if}
								<!--
									Two columns where the window is roomy (#197). A tablet on its
									side is short and wide, and five amounts in one column took
									almost half of it from the Step. One amount keeps the whole
									width rather than wrapping in half of it.

									TWO COLUMNS ON A BUSY STEP, on every window but a phone on its
									side (#681). Eight amounts one under the other took the phone's
									whole screen and left the Step three lines: the tomato soup at
									six servings, the very case #681 was filed for. In two columns
									they take half the height, each line still a whole 48px tap, and
									on a phone in the ingredient line's own size, which fits two
									lines of words in that height.

									The columns run down like a newspaper's rather than in rows, so
									a long line opposite a short one leaves no hole beside the short
									one: rows sized to the longer of each pair left the eye a ragged
									grid and took the height of the longest line twice (round 1 of
									#681's climb). On a tablet the lines keep the small gap the
									single column has: lines that touched there gave their 48px taps
									away to each other by a fraction of a pixel, which measured 45.
									The phone's lines touch and measure 48, and the phone has no
									pixels to spare.
								-->
								<ul
									class={columns
										? 'columns-2 gap-x-3 wide:gap-x-8 sideways:columns-1'
										: 'flex flex-col gap-1 roomy:grid roomy:grid-cols-2 roomy:gap-x-8'}
								>
									{#each amounts as amount (amount.at)}
										<li
											class={columns
												? 'break-inside-avoid wide:pb-1 sideways:pb-1'
												: 'only:col-span-2'}
										>
											{#if amount.row.from === null}
												<!-- A line the cook wrote: no box, since it has no place in
												     the recipe for a tick to keep, and the bar a changed
												     line wears. -->
												<p class="flex min-h-12 w-full items-start gap-3">
													<span
														class="h-[26px] w-[26px] shrink-0 wide:h-[30px] wide:w-[30px]"
														aria-hidden="true"
													></span>
													<span
														class="block min-w-0 flex-1 border-l-2 border-cook-accent pl-2 font-display font-semibold {figure}"
													>
														{amount.row.text}
													</span>
												</p>
											{:else}
												<button
													type="button"
													class="flex min-h-12 w-full items-start gap-3 text-left"
													aria-pressed={isTicked(amount.row)}
													onclick={() => tick(amount.row.from)}
												>
													<span
														class="mt-[2px] flex h-[26px] w-[26px] shrink-0 items-center justify-center rounded-sm border-2 wide:h-[30px] wide:w-[30px]
														{isTicked(amount.row)
															? 'border-cook-accent bg-cook-accent text-cook-on-accent'
															: 'border-cook-soft'}"
														aria-hidden="true"
													>
														{#if isTicked(amount.row)}
															<svg
																viewBox="0 0 24 24"
																class="h-[18px] w-[18px]"
																fill="none"
																stroke="currentColor"
																stroke-width="3"
																stroke-linecap="round"
																stroke-linejoin="round"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg
															>
														{/if}
													</span>
													<span
														class="min-w-0 flex-1 {isTicked(amount.row) ? 'opacity-50' : ''}
														{amount.changed || amount.row.dropped ? 'border-l-2 border-cook-accent pl-2' : ''}"
													>
														{#if scaled && amount.beneath && !amount.changed && !amount.row.dropped}
															<!--
															Scaled, so the scaled line stands alone: since #680 it is
															the whole line at the new amount, `1½ heads of garlic`, and
															says what it is an amount of by itself. The recipe's own
															line no longer doubles every row beneath it (#681): it is
															one tap away in Note a change, and on the recipe page.
														-->
															<span class="block font-display font-semibold {figure}">
																{amount.beneath}
															</span>
														{:else}
															<span
																class="block font-display font-semibold {figure}
																{amount.row.dropped ? 'line-through opacity-45' : ''}
																{amountsCaughtUp ? '' : 'text-cook-soft'}"
															>
																{amount.row.text}
															</span>
															{#if scaled && amount.hasAmount && !amount.changed && !amount.row.dropped}
																<!-- Only a line with an amount Kamosu could not scale says so (#680). -->
																<span class="block text-step-reading text-cook-soft"
																	>{m.how_much_not_scaled()}</span
																>
															{:else if amount.beneath && amountsCaughtUp}
																<!-- In the cook's own measures, so often the figure they
																     measure by: at the ingredient line's size, where 15px
																     muted was the hardest thing on the screen to read
																     (round 2 of #681's climb). -->
																<span
																	class="block text-line leading-[calc(24*var(--type-unit))] text-cook-soft wide:text-step-reading wide:leading-[1.4]"
																	>{amount.beneath}</span
																>
															{/if}
														{/if}
														<!--
														What the recipe asked for, kept under what went in. One
														short line, and *what did it say?* is a real question with
														a hot pan in hand — unlike a whole struck paragraph under
														the Step, which would be a difference view (ADR 0014).
													-->
														{#if amount.changed && !amount.row.dropped && amount.written !== null}
															<span class="block text-step-reading text-cook-soft line-through">
																{amount.written}
															</span>
														{/if}
													</span>
													<span class="sr-only">{m.cook_tick({ line: amount.row.text })}</span>
												</button>
											{/if}
										</li>
									{/each}
								</ul>
							{/if}
						</div>

						<!--
							The Step. The largest type in the app. A hairline sets it off
							from the amounts above it, where there are any.
						-->
						<div
							bind:this={stepBoxEl}
							class="flex flex-1 flex-col items-start sideways:mt-0 sideways:min-h-0 sideways:overflow-y-auto sideways:border-t-0 sideways:border-l sideways:border-cook-rule sideways:pt-0 sideways:pl-[20px]
								{amounts.length > 0 || writing ? 'mt-3 border-t border-cook-rule pt-3 wide:mt-4 wide:pt-6' : ''}
								{nextBeside ? 'roomy:flex-row roomy:items-stretch roomy:gap-8' : ''}"
						>
							{#if writing}
								<!--
									The Step, still the largest type in the app, still in the same
									place — now a field. The cook never left the step, because there
									was never anywhere else to be. It keeps room to type in however
									much the recipe's own words take beneath it (#681).

									A method that grew a stage. Inserting BEFORE and staying put is
									the wording chosen and the right shape: a cook writes the step
									down having just done it, so the new one is where they now are.
									The button stands above the field, before the Step it goes
									before: under it, it was below the fold of a phone (round 2 of
									#681's climb).
								-->
								<button type="button" class="quiet-button mb-2 shrink-0" onclick={insertStep}>
									{m.cook_insert_step()}
								</button>
								<textarea
									bind:this={stepFieldEl}
									value={here.row.text}
									placeholder={here.row.from === null ? m.cook_added_step() : undefined}
									oninput={(event) => write('steps', here.index, event.currentTarget.value)}
									class="min-h-[200px] w-full flex-1 resize-none rounded-sm border border-cook-accent bg-transparent p-2 font-display text-step font-semibold text-cook-ink"
								></textarea>
								{#if wroteStep(here.row) && here.row.from !== null}
									{@const written = content.steps[here.row.from]?.text ?? ''}
									<button
										type="button"
										class="min-h-12 shrink-0 text-left text-step-reading text-cook-soft line-through"
										onclick={() => write('steps', here.index, written)}
									>
										{written}
										<span class="sr-only">— {m.cook_as_written()}</span>
									</button>
								{/if}
							{:else}
								<!--
									A rewritten Step wears the bar and nothing else. An amount keeps
									its old line under it, but a whole struck paragraph under the
									largest type in the app is a difference view, and Kamosu does not
									draw one (ADR 0014). The recipe's words are one tap away inside
									writing mode.

									The Step's photograph, where the recipe gives it one (#110): a
									small square at the top right that the words wrap around, and a
									tap shows it across the screen. By where the step CAME FROM, like
									everything else read off the Version: a step the cook wrote has
									no photograph, and a rewritten one keeps the recipe's.
								-->
								<div
									class="w-full {nextBeside
										? 'mb-4 roomy:mb-0 roomy:min-w-0 roomy:flex-1'
										: comingNext
											? 'mb-4'
											: ''}"
								>
									<div
										class="font-display text-step font-semibold
											{wroteStep(here.row) ? 'border-l-[3px] border-cook-accent pl-3' : ''}"
									>
										{#if stepPhoto}
											<StepPhoto
												photograph={stepPhoto}
												number={position + 1}
												shapeClass="float-right mt-1 mb-2 ml-3 h-[var(--step-photo)] w-[var(--step-photo)]"
											/>
										{/if}
										<StepWords
											text={here.row.text}
											conversions={stepConversions}
											lead={scaled}
											readingClass="font-sans text-step-reading font-normal text-cook-soft"
										/>
									</div>
								</div>
							{/if}
							<!--
								THE NEXT STEP, small (#197). It was chosen on 5 October 2026 over
								the same screen without it. A tablet on the counter has the room,
								and reading ahead is what a cook does while something simmers. It
								is the words and nothing else: the amounts above are still only
								this Step's (ADR 0011), and the next one's arrive when the cook does.

								Under the Step on an upright tablet, at the foot of the body when
								the Step is short and after it when the Step is long, so it takes
								only the room the Step leaves (#681), held to three lines. Beside
								the Step where the window is roomy, where it takes as many whole
								lines as its column is tall, unless beside it the Step would run
								past the foot: then under it, as upright.
							-->
							{#if comingNext}
								<aside
									bind:this={nextEl}
									aria-labelledby={nextId}
									class="mt-auto min-h-0 w-full shrink-0 border-t border-cook-rule pt-4 text-cook-soft
										{nextBeside
										? 'roomy:mt-0 roomy:w-[30%] roomy:overflow-hidden roomy:border-t-0 roomy:border-l roomy:pt-0 roomy:pl-6'
										: ''}"
								>
									<h2 id={nextId} class="pb-2 text-label font-semibold uppercase">
										{m.cook_up_next({ n: position + 2, total: stops.length })}
									</h2>
									<p
										class="line-clamp-3 font-display text-next-step"
										style:-webkit-line-clamp={nextLines}
									>
										{comingNext.row.text}
									</p>
								</aside>
							{/if}
						</div>
					</div>
				</div>
				<!--
					More below the bottom edge. The last 28px fade into the room's own
					ground, and only while there is more to scroll to: a line sliced by
					the box's edge reads as a rendering fault rather than as an
					invitation to scroll, and a fade at the end greys out the last line
					for nothing (#681).
				-->
				{#if moreBelow}
					<div
						class="pointer-events-none absolute inset-x-0 bottom-0 step-fade"
						aria-hidden="true"
					></div>
				{/if}
			</div>
		{/if}

		<!--
			THE WAYS THROUGH THE RECIPE, and the only things at the foot of the
			phone (#88): no tap near the bottom of a wet phone can land anywhere
			else. Back and Next are halves of one row, Back a `second-button` and
			Next the screen's `main-button` (#651).

			WHILE HOW MUCH IS ASKED there is one button, and no Back: how much is
			not a step, so there is nothing before it (#651, C2). Back on the first
			step goes to it, so Back is never greyed out on a step.

			A PHONE ON ITS SIDE draws them 40 tall, since it is 390 tall in all,
			and since round 2 of #681's climb they take a tap 8px past their foot,
			48 in all, as a quiet button does: drawn smaller than 48, never tapped
			smaller (ADR 0044). Below, into the screen's own margin: above, they
			reached into the last amount's tap.
		-->
		{#if moveFailed && !asking}
			<p class="shrink-0 pt-2 text-step-reading text-cook-ink" role="alert">
				{m.cook_move_failed()}
			</p>
		{/if}
		<div
			class="relative flex shrink-0 gap-3 pt-3 wide:gap-4 wide:pt-6 wide:pb-3 sideways:pt-2 sideways:[&>button]:relative sideways:[&>button]:min-h-[40px] sideways:[&>button]:after:absolute sideways:[&>button]:after:inset-x-0 sideways:[&>button]:after:top-0 sideways:[&>button]:after:bottom-[-8px] sideways:[&>button]:after:content-['']"
		>
			{#if asking}
				<button
					type="button"
					class="main-button flex-1 wide:min-h-[var(--cook-foot-far)] wide:text-foot-far"
					onclick={() => {
						// Next takes this button's place: a double tap's second half
						// must not skip the first Step (round 2 of #681's climb).
						movedAt = Date.now();
						asking = false;
						atStart = false;
					}}
				>
					{atStart ? m.cook_start_cooking() : m.cook_back_to_step({ n: position + 1 })}
				</button>
			{:else}
				<button
					type="button"
					class="second-button flex-1 wide:min-h-[var(--cook-foot-far)] wide:text-foot-far"
					onclick={() => moveOnPurpose(back)}
				>
					{m.cook_back()}
				</button>
				<button
					type="button"
					class="main-button flex-1 wide:min-h-[var(--cook-foot-far)] wide:text-foot-far"
					onclick={last ? finishOnPurpose : () => moveOnPurpose(() => step(position + 1))}
				>
					{last ? m.cook_finish() : m.cook_next()}
				</button>
			{/if}
		</div>

		<!--
			Opening either sheet below moves the caret into it, which `SheetFrame`
			does. It has to: the control that opens the false start is at the top
			of the screen, while the sheet is the last thing in the document.
			Without it a keyboard or a screen reader would leave the trigger, walk
			the amounts, the Step, Back and Next, and only then arrive at a
			confirmation it had already asked for.

			KEEP COOKING IS THE MAIN BUTTON, and Enter is it too (#651, U11):
			throwing a cooking away is the one act here that destroys something,
			so it is outlined in beni and apart, never the filled one a wet thumb
			or a key reaches first.
		-->
		{#if discarding}
			<SheetFrame
				dim="cook"
				label={m.cook_false_start()}
				class="bg-cook-panel px-gutter pt-6"
				onclose={() => (discarding = false)}
				onconfirm={() => (discarding = false)}
			>
				<p class="text-body text-cook-ink">{m.cook_false_start_confirm()}</p>
				<!-- Keep cooking at the foot, where Next was a moment ago and the
				     thumb already is; throwing away above it, a reach apart. The
				     other way round put the losing one under the thumb (round 2 of
				     #681's climb, U11). -->
				<button type="button" class="mt-4 second-button w-full loses" onclick={discard}>
					{m.cook_false_start_yes()}
				</button>
				<button type="button" class="mt-6 main-button w-full" onclick={() => (discarding = false)}>
					{m.cook_false_start_no()}
				</button>
			</SheetFrame>
		{/if}

		<!--
			#98's card, raised from the foot the way the false start's question is,
			because that is the one sheet a cook already knows here. It opens only
			on a tap, so it never covers a Step the cook did not ask it to.
		-->
		{#if readingMistake && wentWrong}
			<SheetFrame
				dim="cook"
				label={m.wrong_title()}
				class="border-t-3 border-t-support bg-cook-panel px-gutter pt-6"
				onclose={mistakeRead}
				onconfirm={mistakeRead}
			>
				{@render wrongCard()}
				<button type="button" class="mt-4 main-button w-full" onclick={mistakeRead}>
					{m.notice_got_it()}
				</button>
			</SheetFrame>
		{/if}
	{/if}
</div>

{#snippet clockIcon()}
	<svg
		viewBox="0 0 24 24"
		aria-hidden="true"
		class="h-[18px] w-[18px] shrink-0"
		fill="none"
		stroke="currentColor"
		stroke-width="1.8"
		stroke-linecap="round"><circle cx="12" cy="13" r="8" /><path d="M12 9v4l3 2M9 3h6" /></svg
	>
{/snippet}
