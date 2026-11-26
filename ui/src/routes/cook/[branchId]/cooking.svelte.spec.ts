/**
 * The screen-seam test for the cooking screen (#61, ADR 0011), against a
 * stand-in Kamosu. Every answer below is checked against the Catalogue before
 * the screen sees it — a field renamed in `src/catalogue.rs` fails this test in
 * the same commit.
 *
 * What is being tested is ADR 0011's shape: **one Step, with the amounts for
 * that step above it**. If any assertion here starts describing a screen that
 * shows the whole ingredient list, or a step that is not the largest thing on
 * it, the screen has gone wrong.
 */

import { afterEach, beforeEach, describe, expect, it, vi, type MockInstance } from 'vitest';
import { cleanup, render, screen, fireEvent, within } from '@testing-library/svelte';
import { standIn, type Answers, type Refusal } from '#lib/api/stand-in.js';
import { reached, refreshed } from '#lib/offline/device.svelte.js';
import { page } from '$app/state';
import type { GetRecipeOutput, StartAttemptOutput } from '#lib/api/catalogue.js';
import type { Room } from '#lib/room.svelte.js';
import CookingTestHarness from './CookingTestHarness.svelte';
import { fresh } from './fresh';
import { keepingForTests } from '../../../testing/render';
import { putMistakeAway } from '#lib/mistake.svelte.js';

// A running timer is kept on the device under its cooking's id, and every
// test here cooks `at_1`: one test's timer must not still run in the next.
afterEach(() => {
	for (const key of Object.keys(localStorage))
		if (key.startsWith('kamosu.timer.')) localStorage.removeItem(key);
});

const INGREDIENTS = [
	{ kind: 'section' as const, text: 'For the cutlets' },
	{ kind: 'ingredient' as const, text: '2 chicken breasts' },
	{ kind: 'ingredient' as const, text: '1 cup panko' },
	{ kind: 'ingredient' as const, text: '800 ml water' },
	{ kind: 'ingredient' as const, text: 'a pinch of salt' },
];

const STEPS = [
	{ kind: 'section' as const, text: 'Assemble', photo: null },
	{ kind: 'step' as const, text: 'Coat the chicken in panko.', photo: null },
	{ kind: 'step' as const, text: 'Pour in the water and simmer for about 7 minutes.', photo: null },
	{ kind: 'step' as const, text: 'Season with salt and leave it alone.', photo: null },
];

/**
 * The Core's own answer, as `get_recipe` serves it. Note what the screen is
 * NOT given: no mapping it could compute itself, no filtered ingredient list —
 * only `uses`, a list of indices, and `timer_seconds`, both worked out
 * server-side so the MCP door has them too.
 */
const COOKING = {
	steps: [
		null,
		{ uses: [1, 2], by_hand: false, timer_seconds: null },
		{ uses: [3], by_hand: false, timer_seconds: 420 },
		{ uses: [4], by_hand: false, timer_seconds: null },
	],
};

const attempt = (over: Partial<StartAttemptOutput> = {}): StartAttemptOutput => ({
	id: 'at_1',
	lineage_id: 'l_1',
	person_id: 'p_1',
	version_id: 'v_1',
	current_step_index: 0,
	ticked_ingredients: [],
	cooking_yield: null,
	note: null,
	rating: null,
	finished_at: null,
	resumable: true,
	created_at: '2026-08-30T10:00:00Z',
	last_action_at: '2026-08-30T10:00:00Z',
	as_cooked: null,
	photographs: [],
	...over,
});

function answers(over: Answers = {}): Answers {
	return {
		instance_status: {
			version: '0.1.0',
			setup_complete: true,
			password_minimum: 15,
			passkey_origin: null,
			demo: false,
		},
		start_attempt: attempt(),
		// Every move on this screen is the same one Operation, sent whole
		// (ADR 0010). The stand-in echoes an Attempt back so the screen has a
		// real answer to replace what it laid down optimistically.
		advance_attempt: attempt(),
		finish_attempt: attempt({ finished_at: '2026-08-30T11:00:00Z', resumable: false }),
		// What the cook actually cooked, sent whole (#58). The stand-in echoes an
		// Attempt back, exactly as `advance_attempt` above does.
		set_as_cooked: attempt(),
		delete_attempt: { deleted: true },
		get_recipe: {
			branch_id: 'b_1',
			lineage_id: 'l_1',
			cookbook: { id: 'c_1', name: null, authors: [{ person_id: 'p_1', name: 'Stéphane' }] },
			name: null,
			writes: true,
			mine: true,
			arrived: false,
			hand_id: 'h_1',
			language: 'en',
			origin_address: null,
			head_version_id: 'v_1',
			translation: null,
			tags: [],
			related_recipes: [],
			cooked: { count: 0, last_cooked_at: null, ratings: [] },
			versions: [
				{
					sequence: 1,
					version_id: 'v_1',
					parent_version_id: null,
					hand_id: 'h_1',
					name: null,
					change_note: null,
					created_at: '2026-08-30T09:00:00Z',
					translates_version_id: null,
					scaled_to: null,
					yield_nouns: { one: 'serving', many: 'servings' },
					language: 'en',
					// A recipe that composes nothing, which is nearly all of them (#50).
					components: [],
					content: {
						title: 'Chicken Katsu Curry',
						yield: { amount: '4', noun: 'servings' },
						prep_time_minutes: 20,
						cook_time_minutes: 30,
						note: null,
						main_photo: null,
						nutrition: null,
						source: null,
						ingredients: INGREDIENTS,
						steps: STEPS,
					},
					readings: [
						null,
						{ amount: '2', unit: null, target: 'chicken', lineage_id: null },
						{ amount: '1', unit: 'cup', target: 'panko', lineage_id: null },
						{ amount: '800', unit: 'ml', target: 'water', lineage_id: null },
						// `a pinch of salt`: Kamosu knows what the line is ABOUT and
						// could not read a quantity out of it. 28% of real Ingredient
						// Lines carry no quantity, and such a line is a working line.
						{ amount: null, unit: null, target: 'salt', lineage_id: null },
					],
					measured: {
						// Nothing beneath the salt, because there is nothing Kamosu
						// could honestly put there.
						ingredients: [null, null, 'about 240 g', null, null],
						// Nothing for any Step: none of these writes an oven or an
						// amount (#150 puts one in its own test).
						steps: [null, null, null, null],
					},
					cooking: COOKING,
				},
			],
		},
		...over,
	};
}

/** The same recipe answer with a different `cooking` derivation laid over it. */
function withCooking(cooking: GetRecipeOutput['versions'][number]['cooking']): GetRecipeOutput {
	const recipe = answers().get_recipe as GetRecipeOutput;
	return { ...recipe, versions: [{ ...recipe.versions[0], cooking }] };
}

/**
 * Open the cooking screen. A cooking nothing has happened to yet opens on
 * *how much are you making?* (#109), so for the tests about the step itself
 * this answers it the way most cooks will — Start cooking, as written — and
 * hands over the screen standing on the first Step. `asked: true` leaves the
 * question open for the tests about it.
 */
const cook = async (over: Answers = {}, { asked = false, room = 'phone' as Room } = {}) => {
	const kamosu = standIn(answers(over));
	render(CookingTestHarness, { props: { client: kamosu.client, branchId: 'b_1', room } });
	const start = over.start_attempt ?? attempt();
	const opensOnTheQuestion =
		typeof start === 'object' && 'current_step_index' in start && fresh(start);
	if (opensOnTheQuestion && !asked) {
		await fireEvent.click(await screen.findByRole('button', { name: 'Start cooking' }));
		// A moment passes before the cook's next tap, as at the stove: Back and
		// Next take no tap for a bounce's length after a move, Start included.
		if (vi.isFakeTimers()) await vi.advanceTimersByTimeAsync(BOUNCE_PAST);
		else {
			const real = Date.now.bind(Date);
			shifted = vi.spyOn(Date, 'now').mockImplementation(() => real() + BOUNCE_PAST);
		}
	}
	return kamosu;
};

/** Longer than the screen's bounce guard on Back, Next and Finish. */
const BOUNCE_PAST = 600;
let shifted: MockInstance<() => number> | undefined;
afterEach(() => {
	shifted?.mockRestore();
	shifted = undefined;
});

/**
 * The innermost element whose whole text is exactly this — the step, the
 * amount, the button. `children.length === 0` keeps it off the wrappers a
 * matching element is nested inside.
 */
const exactly = (text: string) =>
	screen.findByText((_, node) => node?.textContent?.trim() === text && node.children.length === 0);

/** Open More, where the rarely needed controls live since #681. */
const openMore = async () =>
	fireEvent.click(await screen.findByRole('button', { name: /^more$/i }));

describe('the cooking screen', () => {
	it('opens on the first Step even where the stored index lands on a Section', async () => {
		await cook();
		// `current_step_index` is 0, which is the Section this recipe opens
		// with. A Section is a heading, not somewhere a cook stands.
		expect(await exactly('Coat the chicken in panko.')).toBeInTheDocument();
		expect(await screen.findByText('Assemble')).toBeInTheDocument();
	});

	it('shows only the Ingredients this Step uses, and none of the others', async () => {
		await cook();
		expect(await screen.findByText('2 chicken breasts')).toBeInTheDocument();
		expect(await screen.findByText('1 cup panko')).toBeInTheDocument();
		// The panel is scoped to the step. The water belongs to the next one and
		// the salt to no step at all, and neither is on screen — which is the
		// whole of ADR 0011: not the list, not a filter over the list.
		expect(screen.queryByText('800 ml water')).not.toBeInTheDocument();
		expect(screen.queryByText('a pinch of salt')).not.toBeInTheDocument();
	});

	it('sets the Step in the largest type in the app, above every amount', async () => {
		await cook();
		const step = await exactly('Coat the chicken in panko.');
		expect(step).toHaveClass('text-step');
		// The amounts panel uses the panel figure, which is smaller by
		// construction — the type scale is spec (ADR 0011), so the ranking is
		// asserted rather than the pixel value, which lives in app.css.
		const amount = await exactly('2 chicken breasts');
		expect(amount).toHaveClass('text-panel-figure');
		expect(amount).not.toHaveClass('text-step');
	});

	it('never lets the amounts push the Step and both ways out off the screen', async () => {
		// `Best Steak Marinade In Existence` names TEN Ingredient Lines on one
		// Step, and it is a real recipe in the corpus. Before #88 the amounts
		// were `shrink-0` against a `flex-1` Step, so on that recipe they pushed
		// the Step, the timer and both buttons clean off the bottom of the phone.
		// #88 capped them at 45% and scrolled the Step apart, which hid lines with
		// no sign of them and cut the Step after four lines (#681). Since #681 the
		// amounts and the Step are one body that scrolls as one.
		//
		// jsdom does no layout, so a height cannot be measured here. What is
		// asserted is the MECHANISM: one scrolling box holds both, and both ways
		// through the recipe stand outside it.
		await cook();
		const body = (await exactly('2 chicken breasts')).closest('.overflow-y-auto');
		expect(body).toBeTruthy();
		expect(body).toContainElement(await exactly('Coat the chicken in panko.'));
		expect(body).not.toContainElement(screen.getByRole('button', { name: 'Back' }));
		expect(body).not.toContainElement(screen.getByRole('button', { name: 'Next step' }));
		// Nothing caps the amounts any more.
		expect(body?.innerHTML).not.toMatch(/max-h-\[\d+%\]/);
	});

	it('shows a quantity Kamosu could not read whole, and never guesses at one', async () => {
		await cook({ start_attempt: attempt({ current_step_index: 3 }) });
		// `a pinch of salt` has a Reading — Kamosu knows the line is about salt,
		// which is how it reaches this step's panel at all — but no quantity it
		// could read. So the written line stands whole, with nothing beneath it
		// and no mark saying Kamosu failed at anything (ADR 0002).
		const salt = await exactly('a pinch of salt');
		expect(salt.parentElement?.textContent?.trim()).toBe('a pinch of salt');
	});

	it('puts each conversion straight after what it converts in the Step, and changes no word', async () => {
		// Option A (#150): "1 cup (about 240 g) panko" and "350° (about 180 °C)",
		// each figure beside what it converts, the oven included, rather than on
		// a line of figures beneath.
		const recipe = answers().get_recipe as GetRecipeOutput;
		const version = recipe.versions[0];
		const written = 'Coat the chicken in 1 cup panko, then 1 cup more, and bake at 350°.';
		await cook({
			start_attempt: attempt({ current_step_index: 1 }),
			get_recipe: {
				...recipe,
				versions: [
					{
						...version,
						content: {
							...version.content,
							steps: version.content.steps.map((row, at) =>
								at === 1 ? { ...row, text: written } : row,
							),
						},
						measured: {
							...version.measured,
							steps: [
								null,
								[
									{ written: '1 cup', measured: 'about 240 g' },
									{ written: '1 cup', measured: 'about 240 g' },
									{ written: '350°', measured: 'about 180 °C' },
								],
								null,
								null,
							],
						},
					},
				],
			},
		});
		const addition = (await screen.findAllByText('(about 240 g)'))[0];
		const step = addition.closest('.text-step') as HTMLElement;
		expect(step).toHaveClass('text-step');
		expect(addition).toHaveClass('text-step-reading', 'text-cook-soft');
		expect(step.textContent?.replace(/\s+/g, ' ').trim()).toBe(
			'Coat the chicken in 1 cup (about 240 g) panko, then 1 cup (about 240 g) more, and bake at 350° (about 180 °C).',
		);
		// Take the additions away and the Step is exactly as written.
		const bare = step.cloneNode(true) as HTMLElement;
		bare.querySelectorAll('.text-step-reading').forEach((span) => span.remove());
		expect(
			bare.textContent
				?.replace(/\u00a0/g, '')
				.replace(/\s+/g, ' ')
				.trim(),
		).toBe(written);
		// The oven has no line of its own beneath the Step any more.
		expect(screen.queryByText('about 180 °C')).not.toBeInTheDocument();
		expect(screen.getByText('(about 180 °C)')).toHaveClass('text-step-reading');
	});

	it('carries the one subordinate line the Core worked out, and invents no arithmetic', async () => {
		await cook();
		// `about 240 g` is the Core's answer for `1 cup panko` at this cook's
		// measures and Yield (#49). The screen displays it and computes nothing.
		expect(await screen.findByText('about 240 g')).toBeInTheDocument();
	});

	/** Three lines on the first Step. */
	const busy = () =>
		withCooking({
			steps: [null, { uses: [1, 2, 3], by_hand: false, timer_seconds: null }, null, null],
		});
	const busyList = async () =>
		(await screen.findByRole('button', { name: /chicken/i })).closest('ul');

	it("keeps a busy Step's amounts in one column where they fit (#681)", async () => {
		// jsdom lays nothing out, so the body fits.
		await cook({ get_recipe: busy() });
		expect(await busyList()).not.toHaveClass('columns-2');
		expect(await exactly('2 chicken breasts')).toHaveClass('text-panel-figure');
	});

	it('first tries one column at the middle size (round 4 of #681)', async () => {
		// The body runs past its box only while a figure is at its full size.
		const body = (element: HTMLElement) => element.classList.contains('overflow-y-auto');
		Object.defineProperty(HTMLElement.prototype, 'scrollHeight', {
			configurable: true,
			get(this: HTMLElement) {
				if (!body(this)) return 0;
				return this.querySelector('.text-panel-figure') ? 1000 : 400;
			},
		});
		Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
			configurable: true,
			get(this: HTMLElement) {
				return body(this) ? 500 : 0;
			},
		});
		try {
			await cook({ get_recipe: busy() });
			await vi.waitFor(async () =>
				expect(await exactly('2 chicken breasts')).toHaveClass('text-panel-figure-mid'),
			);
			expect(await busyList()).not.toHaveClass('columns-2');
		} finally {
			Reflect.deleteProperty(HTMLElement.prototype, 'scrollHeight');
			Reflect.deleteProperty(HTMLElement.prototype, 'clientHeight');
		}
	});

	it('puts them in two columns once one runs the Step past the foot (#681)', async () => {
		// Chosen by count, two columns squeezed four lines into small type above
		// a short Step (round 1 of #681's climb). Here the body is told it runs
		// twice past its box.
		const body = (element: HTMLElement) => element.classList.contains('overflow-y-auto');
		Object.defineProperty(HTMLElement.prototype, 'scrollHeight', {
			configurable: true,
			get(this: HTMLElement) {
				return body(this) ? 1000 : 0;
			},
		});
		Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
			configurable: true,
			get(this: HTMLElement) {
				return body(this) ? 500 : 0;
			},
		});
		try {
			await cook({ get_recipe: busy() });
			await vi.waitFor(async () => expect(await busyList()).toHaveClass('columns-2'));
			expect(await exactly('2 chicken breasts')).toHaveClass('text-line');
		} finally {
			Reflect.deleteProperty(HTMLElement.prototype, 'scrollHeight');
			Reflect.deleteProperty(HTMLElement.prototype, 'clientHeight');
		}
	});

	it('shows the Step alone on a step that uses no line (#681)', async () => {
		// Step index 1 uses the chicken and the panko; index 2 uses the water.
		// This fixture's `uses: []` case is reached by answering with one.
		await cook({
			start_attempt: attempt({ current_step_index: 2 }),
			get_recipe: withCooking({
				steps: [
					null,
					{ uses: [], by_hand: false, timer_seconds: null },
					{ uses: [], by_hand: false, timer_seconds: null },
					null,
				],
			}),
		});
		// The Step alone: no panel, no filler line (#681).
		expect(await screen.findByText(/^Step \d of \d$/)).toBeInTheDocument();
		expect(screen.queryByText(/nothing new to add/i)).toBeNull();
		expect(screen.queryByText('Tick each one as it goes in')).toBeNull();
	});

	it('offers a timer only where the Step names a duration, and never types one', async () => {
		const kamosu = await cook({ start_attempt: attempt({ current_step_index: 2 }) });
		const start = await screen.findByRole('button', { name: /7 min/i });
		expect(start).toBeInTheDocument();

		vi.useFakeTimers();
		try {
			await fireEvent.click(start);
			// Started with one tap. Nothing was typed and — the point of ADR 0011
			// — nothing was sent: a timer is no part of what a cooking is.
			expect(kamosu.calls.map((call) => call.operation)).not.toContain('advance_attempt');
		} finally {
			vi.useRealTimers();
		}
	});

	it('carries a running timer to the next Step rather than cancelling it', async () => {
		vi.useFakeTimers();
		try {
			await cook({ start_attempt: attempt({ current_step_index: 2 }) });
			await vi.waitFor(() => screen.getByRole('button', { name: /7 min/i }));
			await fireEvent.click(screen.getByRole('button', { name: /7 min/i }));
			await vi.advanceTimersByTimeAsync(60_000);
			// Move on. The timer is the COOK's, not the step's — it is why one is
			// set at all: you start the rice and get on with the sauce.
			await fireEvent.click(screen.getByRole('button', { name: /next step/i }));
			await vi.waitFor(() =>
				expect(screen.getByRole('button', { name: /timer/i })).toBeInTheDocument(),
			);
		} finally {
			vi.useRealTimers();
		}
	});

	it('takes a running timer back when the cook leaves and comes back', async () => {
		vi.useFakeTimers();
		try {
			await cook({ start_attempt: attempt({ current_step_index: 2 }) });
			await vi.waitFor(() => screen.getByRole('button', { name: 'Set a timer for 7 min' }));
			await fireEvent.click(screen.getByRole('button', { name: 'Set a timer for 7 min' }));
			await vi.advanceTimersByTimeAsync(60_000);
			// Off to the recipe, or the phone reloads the page: the screen goes.
			cleanup();
			await vi.advanceTimersByTimeAsync(60_000);
			await cook({ start_attempt: attempt({ current_step_index: 2 }) });
			await vi.waitFor(() =>
				expect(screen.getByRole('button', { name: /timer: 5:00/i })).toBeInTheDocument(),
			);
			// And one that ran out meanwhile says so.
			cleanup();
			await vi.advanceTimersByTimeAsync(10 * 60_000);
			await cook({ start_attempt: attempt({ current_step_index: 2 }) });
			await vi.waitFor(() =>
				expect(screen.getByRole('button', { name: /time is up/i })).toBeInTheDocument(),
			);
			// Put away, it stays away.
			await fireEvent.click(screen.getByRole('button', { name: /time is up/i }));
			cleanup();
			await cook({ start_attempt: attempt({ current_step_index: 2 }) });
			await vi.waitFor(() => screen.getByRole('button', { name: 'Set a timer for 7 min' }));
		} finally {
			vi.useRealTimers();
		}
	});

	it('stops a running timer on a second tap, never on one stray touch (#681)', async () => {
		vi.useFakeTimers();
		try {
			await cook({ start_attempt: attempt({ current_step_index: 2 }) });
			await vi.waitFor(() => screen.getByRole('button', { name: 'Set a timer for 7 min' }));
			await fireEvent.click(screen.getByRole('button', { name: 'Set a timer for 7 min' }));
			await vi.advanceTimersByTimeAsync(5_000);
			const running = screen.getByRole('button', { name: /timer: 6:55/i });
			// One touch says what the next one does, and stops nothing.
			await fireEvent.click(running);
			expect(running).toHaveTextContent('Tap again to stop');
			expect(screen.queryByRole('button', { name: 'Set a timer for 7 min' })).toBeNull();
			// Left alone, it goes back to counting.
			await vi.advanceTimersByTimeAsync(3_500);
			expect(running).not.toHaveTextContent('Tap again to stop');
			// Two touches stop it, and the Step offers its timer again.
			await fireEvent.click(running);
			await fireEvent.click(running);
			expect(screen.getByRole('button', { name: 'Set a timer for 7 min' })).toBeInTheDocument();
		} finally {
			vi.useRealTimers();
		}
	});

	it('does not finish on the second tap of a double tap on Next (#681)', async () => {
		vi.useFakeTimers();
		try {
			const kamosu = await cook({
				start_attempt: attempt({ current_step_index: 2 }),
				advance_attempt: attempt({ current_step_index: 3 }),
			});
			await vi.waitFor(() => screen.getByRole('button', { name: 'Next step' }));
			await fireEvent.click(screen.getByRole('button', { name: 'Next step' }));
			// The same place now reads Finish cooking; a tap landing at once is the
			// double tap's second half, not a decision.
			await fireEvent.click(await screen.findByRole('button', { name: /finish cooking/i }));
			expect(kamosu.calls.map((call) => call.operation)).not.toContain('finish_attempt');
			await vi.advanceTimersByTimeAsync(1_000);
			await fireEvent.click(screen.getByRole('button', { name: /finish cooking/i }));
			await vi.waitFor(() =>
				expect(kamosu.calls.map((call) => call.operation)).toContain('finish_attempt'),
			);
		} finally {
			vi.useRealTimers();
		}
	});

	it('moves one Step for a bounced double tap on Next, and the next on a later tap', async () => {
		vi.useFakeTimers();
		try {
			// The Core echoes the Step it was sent.
			await cook({
				advance_attempt: (input) => attempt({ current_step_index: input.current_step_index ?? 1 }),
			});
			await vi.waitFor(() => screen.getByRole('button', { name: 'Next step' }));
			// The Step on screen, asked now: `exactly` waits on real timers.
			const on = (text: string) =>
				screen.queryAllByText(
					(_, node) => node?.textContent?.trim() === text && node.children.length === 0,
				).length > 0;
			await fireEvent.click(screen.getByRole('button', { name: 'Next step' }));
			await fireEvent.click(screen.getByRole('button', { name: 'Next step' }));
			await vi.waitFor(() =>
				expect(on('Pour in the water and simmer for about 7 minutes.')).toBe(true),
			);
			await vi.advanceTimersByTimeAsync(1_000);
			await fireEvent.click(screen.getByRole('button', { name: 'Next step' }));
			await vi.waitFor(() => expect(on('Season with salt and leave it alone.')).toBe(true));
		} finally {
			vi.useRealTimers();
		}
	});

	it('says what the tick box is for on the first step with amounts, and not again (#681)', async () => {
		await cook({ start_attempt: attempt({ current_step_index: 2 }) });
		expect(await exactly('800 ml water')).toBeInTheDocument();
		expect(screen.queryByText('Tick each one as it goes in')).toBeNull();
	});

	it('opens More over the Step and closes it on a tap anywhere else, which presses nothing', async () => {
		const kamosu = await cook();
		await openMore();
		expect(screen.getByRole('button', { name: 'Throw this cooking away' })).toBeInTheDocument();
		// A layer over the rest of the screen takes the tap: the line beneath the
		// thumb is not ticked (round 2 of #681's climb).
		const layer = document.querySelector('button.fixed.inset-0');
		expect(layer).not.toBeNull();
		await fireEvent.click(layer as HTMLElement);
		expect(screen.queryByRole('button', { name: 'Throw this cooking away' })).toBeNull();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('advance_attempt');
	});

	it('does not skip the first Step on a double tap of Start cooking', async () => {
		vi.useFakeTimers();
		try {
			await cook({}, { asked: true });
			await vi.waitFor(() => screen.getByRole('button', { name: 'Start cooking' }));
			await fireEvent.click(screen.getByRole('button', { name: 'Start cooking' }));
			// Next stands where Start cooking was; the second half of the tap lands on it.
			await vi.waitFor(() => screen.getByRole('button', { name: 'Next step' }));
			await fireEvent.click(screen.getByRole('button', { name: 'Next step' }));
			expect(screen.getByText('Coat the chicken in panko.')).toBeInTheDocument();
		} finally {
			vi.useRealTimers();
		}
	});

	it('offers no timer on a Step that names no duration', async () => {
		await cook();
		expect(await exactly('Coat the chicken in panko.')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /timer/i })).not.toBeInTheDocument();
	});

	it('writes an advance through to the server, sending the whole step index', async () => {
		const kamosu = await cook();
		await fireEvent.click(await screen.findByRole('button', { name: /next step/i }));
		const advance = kamosu.calls.find((call) => call.operation === 'advance_attempt');
		expect(advance?.input).toEqual({ attempt_id: 'at_1', current_step_index: 2 });
	});

	it('offers to try again, and the way back, when the cooking could not start', async () => {
		let refused = true;
		const kamosu = await cook({
			start_attempt: () =>
				refused
					? { refuse: 'internal', message: 'The start went wrong.' }
					: attempt({ current_step_index: 1 }),
		});
		expect(await screen.findByText('Could not start cooking this recipe.')).toBeInTheDocument();
		expect(screen.getByRole('link', { name: 'Back to the recipe' })).toHaveAttribute(
			'href',
			'/recipes/b_1',
		);
		refused = false;
		await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
		expect(await exactly('Coat the chicken in panko.')).toBeInTheDocument();
		expect(kamosu.calls.filter((call) => call.operation === 'start_attempt')).toHaveLength(2);
	});

	it('says so when Kamosu refuses a move, rather than going back without a word', async () => {
		let refused = true;
		await cook({
			advance_attempt: (input) =>
				refused
					? { refuse: 'internal', message: 'The write went wrong.' }
					: attempt({ current_step_index: input.current_step_index ?? 1 }),
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'Next step' }));
		// Back on the Step the cook was on, and told why (round 2 of #681's climb).
		expect(await screen.findByText('Kamosu could not save that. Try again.')).toHaveAttribute(
			'role',
			'alert',
		);
		expect(await exactly('Coat the chicken in panko.')).toBeInTheDocument();
		// Tried again, it goes, and the line goes with it.
		refused = false;
		await new Promise((resolve) => setTimeout(resolve, 400));
		await fireEvent.click(screen.getByRole('button', { name: 'Next step' }));
		expect(await exactly('Pour in the water and simmer for about 7 minutes.')).toBeInTheDocument();
		await vi.waitFor(() =>
			expect(screen.queryByText('Kamosu could not save that. Try again.')).toBeNull(),
		);
	});

	it('writes a ticked Ingredient through to the server, sent whole rather than patched', async () => {
		const kamosu = await cook();
		const panko = await screen.findByRole('button', { name: /1 cup panko/i });
		await fireEvent.click(panko);
		const advance = kamosu.calls.find((call) => call.operation === 'advance_attempt');
		expect(advance?.input).toEqual({ attempt_id: 'at_1', ticked_ingredients: [2] });
	});

	it('shows the cook where they are without making them read a number', async () => {
		await cook({ start_attempt: attempt({ current_step_index: 2 }) });
		// Said in words beside the noren, where it was `2/3` (#651, C2).
		expect(await exactly('Step 2 of 3')).toBeVisible();
	});

	it('names the recipe, and the way out is on every step (#651, C1)', async () => {
		await cook({ start_attempt: attempt({ current_step_index: 2 }) });
		// One link, the back arrow and the name, read aloud as leaving.
		const out = await screen.findByRole('link', { name: 'Leave for now: Chicken Katsu Curry' });
		expect(out).toHaveAttribute('href', '/recipes/b_1');
		expect(out).toHaveTextContent('Chicken Katsu Curry');
		expect(out.closest('header')).not.toBeNull();
		// Pause was the way out, and nobody read it as one.
		expect(screen.queryByText('Pause')).toBeNull();
		// More says its word, where it was a ⋯ alone (U8).
		expect(screen.getByRole('button', { name: 'More' })).toHaveTextContent('More');
	});

	it('says what the tick box is for, and draws it to be hit with a wet thumb (#651, C6)', async () => {
		await cook({ advance_attempt: attempt({ ticked_ingredients: [2] }) });
		expect(await screen.findByText('Tick each one as it goes in')).toBeVisible();
		const panko = screen.getByRole('button', { name: /1 cup panko/i });
		await fireEvent.click(panko);
		await vi.waitFor(() => expect(panko).toHaveAttribute('aria-pressed', 'true'));
		// The box is drawn ticked, a tick inside it, not only filled.
		expect(panko.querySelector('svg')).not.toBeNull();
	});

	it('draws every control on the step as a button that says what it does (#651, C5)', async () => {
		await cook({ start_attempt: attempt({ current_step_index: 2 }) });
		const timer = await screen.findByRole('button', { name: 'Set a timer for 7 min' });
		expect(timer).toHaveClass('quiet-button');
		expect(timer).toHaveTextContent('Timer 7 min');
		// The recipe as written is no choice the cook made, so no amount stands
		// above the Step (#681, U12).
		expect(screen.queryByRole('button', { name: /change how much/i })).toBeNull();
		// The rarely needed ones are rows under More, each saying what it does (U2).
		await openMore();
		expect(screen.getByRole('button', { name: 'Note a change' })).toBeInTheDocument();
		expect(
			screen.getByLabelText('Take a photograph of this cooking').closest('label'),
		).toHaveTextContent('Take a photo');
		expect(screen.getByRole('button', { name: 'Throw this cooking away' })).toBeInTheDocument();
		// How much, asked again from here though nothing above the Step shows it.
		await fireEvent.click(screen.getByRole('button', { name: 'Change how much' }));
		expect(
			await screen.findByRole('heading', { name: 'How much are you making?' }),
		).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: 'Back to step 2' }));
		expect(await exactly('Pour in the water and simmer for about 7 minutes.')).toBeInTheDocument();
		// Next is the step's one main button, Back the second one beside it.
		expect(screen.getByRole('button', { name: 'Next step' })).toHaveClass('main-button');
		expect(screen.getByRole('button', { name: 'Back' })).toHaveClass('second-button');
	});

	it('offers to finish only on the last Step, and asks for no judgement to do it', async () => {
		const kamosu = await cook({ start_attempt: attempt({ current_step_index: 3 }) });
		const finish = await screen.findByRole('button', { name: /finish cooking/i });
		await fireEvent.click(finish);
		const finished = kamosu.calls.find((call) => call.operation === 'finish_attempt');
		// A cook who says nothing still cooked (ADR 0010): the rating and the
		// note are the diary's, not the stove's.
		expect(finished?.input).toEqual({ attempt_id: 'at_1' });
		// And nothing jumps anywhere. Both ways out are offered; neither is taken
		// for the cook.
		expect(await screen.findByRole('link', { name: /open the diary/i })).toBeInTheDocument();
	});

	it('undoes a false start, and never on one tap', async () => {
		const kamosu = await cook();
		// Starting counts as a cooking (ADR 0010), so opening the steps out of
		// curiosity would register as one. This is the counterweight, and it is
		// here at the stove where the false start happened.
		// Under the ⋯ since #222, with the screen's sleep.
		await fireEvent.click(await screen.findByRole('button', { name: /^more$/i }));
		await fireEvent.click(await screen.findByRole('button', { name: /throw this cooking away/i }));
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('delete_attempt');

		// Keeping on is the filled one, and Enter; throwing away is set apart in
		// beni, never the button a wet thumb reaches first (#651, U11).
		expect(screen.getByRole('button', { name: 'Keep cooking' })).toHaveClass('main-button');
		const away = screen.getByRole('button', { name: /throw it away/i });
		expect(away).toHaveClass('second-button', 'loses');
		await fireEvent.click(away);
		const deleted = kamosu.calls.find((call) => call.operation === 'delete_attempt');
		expect(deleted?.input).toEqual({ attempt_id: 'at_1' });
	});

	it('says the screen may sleep where the browser has no Wake Lock API', async () => {
		await cook();
		// jsdom has none, which is the honest case this must handle: no fallback
		// hack, no silent video trick — the screen says what is true.
		await fireEvent.click(await screen.findByRole('button', { name: /^more$/i }));
		const toggle = await screen.findByRole('button', { name: /screen may sleep/i });
		expect(toggle).toBeDisabled();
	});
});

/**
 * Saying *I did it differently* (#58, ADR 0005). This treatment was chosen
 * on 4 September 2026 against one built around a sheet: the step's own words
 * become fields where they already stand, and nothing opens over the Step.
 *
 * The rule every test below is really guarding: **a cooking that deviated from
 * nothing costs the screen one word and the server nothing at all.**
 */
describe('the cooking screen on a tablet on the counter (#197)', () => {
	/** The next Step, small, where the screen draws one. */
	const comingNext = () => screen.queryByRole('complementary', { name: /^Next · / });

	it('shows the next Step small beside the one the cook is on', async () => {
		await cook({}, { room: 'wide' });
		expect(await exactly('Coat the chicken in panko.')).toBeInTheDocument();
		const next = comingNext();
		expect(next).toBeInTheDocument();
		// The Section the recipe opens with is not a Step, so it is not counted.
		expect(within(next!).getByText('Next · 2/3')).toBeInTheDocument();
		expect(
			within(next!).getByText('Pour in the water and simmer for about 7 minutes.'),
		).toBeInTheDocument();
		// Read ahead, and nothing else: the next Step's amounts stay off the
		// screen until the cook is on it (ADR 0011).
		expect(screen.queryByText('800 ml water')).not.toBeInTheDocument();
	});

	it('follows the cook forward', async () => {
		await cook({ start_attempt: attempt({ current_step_index: 2 }) }, { room: 'roomy' });
		expect(await exactly('Pour in the water and simmer for about 7 minutes.')).toBeInTheDocument();
		expect(comingNext()).toContainElement(await exactly('Season with salt and leave it alone.'));
		expect(within(comingNext()!).getByText('Next · 3/3')).toBeInTheDocument();
		// Beside the Step, which fits.
		expect(comingNext()).toHaveClass('roomy:w-[30%]');
	});

	it('puts the next Step under a Step that beside it would run past the foot', async () => {
		// jsdom lays nothing out, so the body is told it is twice as tall as
		// its box: the soup's long step on a tablet on its side, beside a
		// preview that took a third of the width (round 1 of #681's climb).
		const body = (element: HTMLElement) => element.classList.contains('overflow-y-auto');
		Object.defineProperty(HTMLElement.prototype, 'scrollHeight', {
			configurable: true,
			get(this: HTMLElement) {
				return body(this) ? 1000 : 0;
			},
		});
		Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
			configurable: true,
			get(this: HTMLElement) {
				return body(this) ? 500 : 0;
			},
		});
		try {
			await cook({ start_attempt: attempt({ current_step_index: 2 }) }, { room: 'roomy' });
			expect(
				await exactly('Pour in the water and simmer for about 7 minutes.'),
			).toBeInTheDocument();
			await vi.waitFor(() => expect(comingNext()).not.toHaveClass('roomy:w-[30%]'));
			// Still shown, under the Step, as on a tablet upright.
			expect(within(comingNext()!).getByText('Next · 3/3')).toBeInTheDocument();
		} finally {
			Reflect.deleteProperty(HTMLElement.prototype, 'scrollHeight');
			Reflect.deleteProperty(HTMLElement.prototype, 'clientHeight');
		}
	});

	it('sends the next Step away rather than shrink the amounts for it (round 4 of #681)', async () => {
		// The body runs past its foot, but the Step's own words end above it:
		// only the next Step's words under them are past, so they go, and the
		// amounts keep their size. Held to three lines, they ran past the foot
		// and left their heading over nothing.
		const body = (element: HTMLElement) => element.classList.contains('overflow-y-auto');
		Object.defineProperty(HTMLElement.prototype, 'scrollHeight', {
			configurable: true,
			get(this: HTMLElement) {
				return body(this) ? 1000 : 0;
			},
		});
		Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
			configurable: true,
			get(this: HTMLElement) {
				return body(this) ? 500 : 0;
			},
		});
		const laidOut = HTMLElement.prototype.getBoundingClientRect;
		const box = (top: number, bottom: number) =>
			({ top, bottom, height: bottom - top, left: 0, right: 0, width: 0, x: 0, y: top }) as DOMRect;
		HTMLElement.prototype.getBoundingClientRect = function (this: HTMLElement) {
			if (body(this)) return box(0, 500);
			if (this.classList.contains('text-step')) return box(100, 400);
			if (this.closest('aside')) return box(530, 610);
			return laidOut.call(this);
		};
		try {
			await cook({}, { room: 'wide' });
			expect(await exactly('Coat the chicken in panko.')).toBeInTheDocument();
			await vi.waitFor(() => expect(comingNext()).not.toBeInTheDocument());
			expect(await exactly('2 chicken breasts')).toHaveClass('text-panel-figure');
		} finally {
			HTMLElement.prototype.getBoundingClientRect = laidOut;
			Reflect.deleteProperty(HTMLElement.prototype, 'scrollHeight');
			Reflect.deleteProperty(HTMLElement.prototype, 'clientHeight');
		}
	});

	it('shows none on the last Step', async () => {
		await cook({ start_attempt: attempt({ current_step_index: 3 }) }, { room: 'roomy' });
		expect(await exactly('Season with salt and leave it alone.')).toBeInTheDocument();
		expect(comingNext()).not.toBeInTheDocument();
	});

	it('shows none on the phone, where the Step needs every line', async () => {
		await cook();
		expect(await exactly('Coat the chicken in panko.')).toBeInTheDocument();
		expect(comingNext()).not.toBeInTheDocument();
		expect(
			screen.queryByText('Pour in the water and simmer for about 7 minutes.'),
		).not.toBeInTheDocument();
	});

	it('ticks an amount and offers a timer as the phone does', async () => {
		const kamosu = await cook(
			{ start_attempt: attempt({ current_step_index: 2 }) },
			{ room: 'wide' },
		);
		expect(await screen.findByRole('button', { name: /7 min/i })).toBeInTheDocument();
		await fireEvent.click(await screen.findByRole('button', { name: /800 ml water/i }));
		const ticked = kamosu.calls.find((call) => call.operation === 'advance_attempt');
		expect(ticked?.input).toMatchObject({ ticked_ingredients: [3] });
	});

	it('asks how much before a fresh cooking starts, with no next Step over the question', async () => {
		await cook({}, { asked: true, room: 'wide' });
		expect(await screen.findByRole('button', { name: 'Start cooking' })).toBeInTheDocument();
		expect(comingNext()).not.toBeInTheDocument();
	});

	it('gives the room back while the cook is writing on the Step', async () => {
		await cook({}, { room: 'wide' });
		await openMore();
		await fireEvent.click(await screen.findByRole('button', { name: 'Note a change' }));
		expect(comingNext()).not.toBeInTheDocument();
	});
});

describe('writing down what you actually cooked', () => {
	/** Turn the step in front of the cook into fields. */
	async function startWriting() {
		await openMore();
		await fireEvent.click(await screen.findByRole('button', { name: /note a change/i }));
	}

	/** The field holding one line, found by what it currently says. */
	const field = (value: string) =>
		screen.findByDisplayValue((_, node) => (node as HTMLInputElement)?.value === value);

	it('costs a cooking that deviated from nothing one button and no state', async () => {
		const kamosu = await cook();
		await openMore();
		expect(await screen.findByRole('button', { name: /note a change/i })).toBeInTheDocument();
		// Nothing else: no row, no panel, no badge. And crucially nothing sent —
		// ordinary cooking is free (ADR 0005).
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('set_as_cooked');
	});

	it('turns this step’s amounts and its sentence into fields, in place', async () => {
		await cook();
		await startWriting();
		// The same words, at the same size, in the same place. Nothing opened
		// over the Step and the cook never left it.
		expect(await field('2 chicken breasts')).toBeInTheDocument();
		expect(await field('Coat the chicken in panko.')).toHaveClass('text-step');
	});

	it('sends the whole recipe as cooked, not a record of what changed', async () => {
		const kamosu = await cook();
		await startWriting();
		await fireEvent.input(await field('1 cup panko'), { target: { value: '2 cups panko' } });
		await fireEvent.click(await screen.findByRole('button', { name: /^done$/i }));

		const written = kamosu.calls.find((call) => call.operation === 'set_as_cooked');
		const cooked = (written?.input as { as_cooked: { ingredients: { text: string }[] } }).as_cooked;
		// Every line, including the four nobody touched — an As Cooked is a
		// recipe you could stand in, not a diff (ADR 0005).
		expect(cooked.ingredients.map((line) => line.text)).toEqual([
			'For the cutlets',
			'2 chicken breasts',
			'2 cups panko',
			'800 ml water',
			'a pinch of salt',
		]);
	});

	it('marks the line it changed and drops the reading that described the old one', async () => {
		await cook();
		await startWriting();
		await fireEvent.input(await field('1 cup panko'), { target: { value: '2 cups panko' } });
		await fireEvent.click(await screen.findByRole('button', { name: /^done$/i }));

		expect(await screen.findByText('2 cups panko')).toBeInTheDocument();
		// `about 240 g` was the Core's reading of ONE cup. It describes the
		// amount the recipe asked for, not the amount that went in.
		expect(screen.queryByText('about 240 g')).not.toBeInTheDocument();
		// And the recipe's own line is kept under it — one short line, and
		// *what did it say?* is a real question with a hot pan in hand.
		expect(await screen.findByText('1 cup panko')).toBeInTheDocument();
	});

	it('reaches every other line, because most steps name none', async () => {
		await cook();
		await startWriting();
		// The salt belongs to no step in this recipe and the water to the next
		// one. Across the real corpus a step names an Ingredient Line only 42%
		// of the time, so without this the line a cook actually changed is
		// usually not on screen at all.
		expect(screen.queryByDisplayValue('a pinch of salt')).not.toBeInTheDocument();
		await fireEvent.click(await screen.findByRole('button', { name: /the other ingredients/i }));
		expect(await field('a pinch of salt')).toBeInTheDocument();
		expect(await field('800 ml water')).toBeInTheDocument();
	});

	it('adds a line, and drops one without it vanishing under a wet finger', async () => {
		const kamosu = await cook();
		await startWriting();
		await fireEvent.click(await screen.findByRole('button', { name: /the other ingredients/i }));

		// Dropped: still on screen, struck, and the same tap puts it back. Taken
		// by its own row, because every line in writing mode carries this control
		// — the water is the one this test is dropping.
		const water = await field('800 ml water');
		const row = water.closest('li');
		expect(row).not.toBeNull();
		await fireEvent.click(within(row as HTMLElement).getByRole('button', { name: /^drop$/i }));
		expect(
			within(row as HTMLElement).getByRole('button', { name: /^put back$/i }),
		).toBeInTheDocument();
		expect(water).toHaveClass('disabled:line-through');

		await fireEvent.click(await screen.findByRole('button', { name: /add a line/i }));
		const added = await screen.findByPlaceholderText(/a line you added/i);
		await fireEvent.input(added, { target: { value: '1 bay leaf' } });
		await fireEvent.click(await screen.findByRole('button', { name: /^done$/i }));

		const written = kamosu.calls.find((call) => call.operation === 'set_as_cooked');
		const cooked = (written?.input as { as_cooked: { ingredients: { text: string }[] } }).as_cooked;
		const lines = cooked.ingredients.map((line) => line.text);
		expect(lines).toContain('1 bay leaf');
		expect(lines).not.toContain('800 ml water');
	});

	it('adds a line from the start of Note a change, where the cook is', async () => {
		await cook();
		await startWriting();
		// No list to open first: a grader found Add a line only past twelve
		// other rows (round 1 of #681's climb).
		expect(screen.queryByDisplayValue('a pinch of salt')).not.toBeInTheDocument();
		await fireEvent.click(await screen.findByRole('button', { name: /add a line/i }));
		const added = await screen.findByPlaceholderText(/a line you added/i);
		await vi.waitFor(() => expect(added).toHaveFocus());
		// Under this Step's own lines, not inside the whole list.
		expect(screen.getByRole('button', { name: /the other ingredients/i })).toHaveAttribute(
			'aria-expanded',
			'false',
		);
	});

	it('keeps a line rewritten past recognition on its Step when the cooking is opened again', async () => {
		// The Core pairs a line rewritten in wholly other words as the recipe's
		// one dropped and the cook's one added straight after it (ADR 0019).
		// Opened again, the Step showed the struck panko and nothing in its
		// place (round 2 of #681's climb).
		const recipe = answers().get_recipe as GetRecipeOutput;
		const content = recipe.versions[0].content;
		type Row = NonNullable<StartAttemptOutput['as_cooked']>['against']['ingredients'][number];
		const same = (index: number, kind: string, text: string): Row => ({
			from_branch_point: false,
			kind,
			mine: { index, kind, text },
			place: { mine: index, theirs: index },
			state: 'same',
			theirs: { index, kind, text },
		});
		const ingredients: Row[] = [
			same(0, 'section', 'For the cutlets'),
			same(1, 'ingredient', '2 chicken breasts'),
			{
				from_branch_point: false,
				kind: 'ingredient',
				mine: { index: 2, kind: 'ingredient', text: '1 cup panko' },
				place: { mine: 2, theirs: 2 },
				state: 'only-mine',
				theirs: null,
			},
			{
				from_branch_point: false,
				kind: 'ingredient',
				mine: null,
				place: { mine: 3, theirs: 2 },
				state: 'only-theirs',
				theirs: { index: 2, kind: 'ingredient', text: '1 cup breadcrumbs' },
			},
			same(3, 'ingredient', '800 ml water'),
			same(4, 'ingredient', 'a pinch of salt'),
		];
		await cook({
			start_attempt: attempt({
				current_step_index: 1,
				as_cooked: {
					against: {
						ingredients,
						steps: content.steps.map((step, index) => same(index, step.kind, step.text)),
					},
					content: {
						...content,
						ingredients: [
							INGREDIENTS[0],
							INGREDIENTS[1],
							{ kind: 'ingredient', text: '1 cup breadcrumbs' },
							INGREDIENTS[3],
							INGREDIENTS[4],
						],
					},
					promotion_declined: false,
					version_id: 'v_2',
					// Not read here: what the cooking screen scales (#713).
					readings: [],
					measured: { ingredients: [], scaled_to: null },
				},
			}),
		});
		expect(await exactly('1 cup breadcrumbs')).toBeInTheDocument();
		// Drawn with no tick box: a line the cook wrote has nowhere to keep one.
		expect(screen.queryByRole('button', { name: /breadcrumbs/i })).toBeNull();
		expect(screen.getByRole('button', { name: /1 cup panko/i })).toBeInTheDocument();
	});

	it("puts an added line after the Step's own, and keeps it on the Step after Done", async () => {
		const kamosu = await cook();
		await startWriting();
		await fireEvent.click(await screen.findByRole('button', { name: /add a line/i }));
		const added = await screen.findByPlaceholderText(/a line you added/i);
		await fireEvent.input(added, { target: { value: '1 tsp paprika' } });
		await fireEvent.click(await screen.findByRole('button', { name: /^done$/i }));

		// After the panko, the last line this Step uses, rather than at the foot
		// of the list: the Core's pairing keeps it there, so it is this Step's
		// when the cooking is opened again (round 3 of #681's climb).
		const written = kamosu.calls.find((call) => call.operation === 'set_as_cooked');
		const cooked = (written?.input as { as_cooked: { ingredients: { text: string }[] } }).as_cooked;
		expect(cooked.ingredients.map((line) => line.text)).toEqual([
			'For the cutlets',
			'2 chicken breasts',
			'1 cup panko',
			'1 tsp paprika',
			'800 ml water',
			'a pinch of salt',
		]);
		expect(await exactly('1 tsp paprika')).toBeInTheDocument();
	});

	it('inserts a step before this one and leaves the cook standing on it', async () => {
		const kamosu = await cook();
		await startWriting();
		await fireEvent.click(await screen.findByRole('button', { name: /insert a step before/i }));

		// The cook is writing down what they have just done, so the new step is
		// where they now are — empty, and theirs.
		const step = await screen.findByPlaceholderText(/a step you added/i);
		// Its field has the caret, so the cook types rather than looks for it.
		await vi.waitFor(() => expect(step).toHaveFocus());
		await fireEvent.input(step, { target: { value: 'Rest the crumb twenty minutes.' } });
		await fireEvent.click(await screen.findByRole('button', { name: /^done$/i }));

		const written = kamosu.calls.find((call) => call.operation === 'set_as_cooked');
		const cooked = (written?.input as { as_cooked: { steps: { text: string }[] } }).as_cooked;
		expect(cooked.steps.map((line) => line.text)).toEqual([
			'Assemble',
			'Rest the crumb twenty minutes.',
			'Coat the chicken in panko.',
			'Pour in the water and simmer for about 7 minutes.',
			'Season with salt and leave it alone.',
		]);
	});

	it('sends nothing at all where the cook ends up back where they started', async () => {
		const kamosu = await cook();
		await startWriting();
		const panko = await field('1 cup panko');
		await fireEvent.input(panko, { target: { value: '2 cups panko' } });
		await fireEvent.input(await field('2 cups panko'), { target: { value: '1 cup panko' } });
		await fireEvent.click(await screen.findByRole('button', { name: /^done$/i }));

		const written = kamosu.calls.find((call) => call.operation === 'set_as_cooked');
		// Null rather than the recipe: the Core would reach the same answer by
		// fingerprint, and this saves it the round trip.
		expect((written?.input as { as_cooked: unknown }).as_cooked).toBeNull();
	});
});

describe('photographing the cooking (#77)', () => {
	it('offers one quiet word at the stove, and counts what has been taken', async () => {
		const keeping = keepingForTests();
		const kamosu = standIn(
			answers({ edit_attempt: attempt({ photographs: ['local:0000000000000001'] }) }),
		);
		render(CookingTestHarness, { props: { client: kamosu.client, branchId: 'b_1', keeping } });
		await fireEvent.click(await screen.findByRole('button', { name: 'Start cooking' }));

		await openMore();
		const picker = await screen.findByLabelText('Take a photograph of this cooking');
		expect(picker).toHaveAttribute('accept', 'image/*');
		expect(picker).toHaveAttribute('capture', 'environment');
		expect(screen.getByText('Take a photo')).toBeInTheDocument();

		const picture = new File(['a plate'], 'plate.jpg', { type: 'image/jpeg' });
		await fireEvent.change(picker, { target: { files: [picture] } });

		// Kept on the phone first, then put beside the cooking's others — never
		// sent as the whole list, which would erase a picture another device took.
		await vi.waitFor(() => {
			const asked = kamosu.calls.find((call) => call.operation === 'edit_attempt');
			expect(asked?.input).toEqual({
				attempt_id: 'at_1',
				add_photographs: ['local:0000000000000001'],
			});
		});
		expect(keeping.kept).toEqual([picture]);
		// Said on the step itself, where the cook is looking.
		expect(await screen.findByRole('status')).toHaveTextContent('Photo kept with this cooking.');
		// Taking one closes More; opened again, it counts what has been taken.
		await openMore();
		expect(await screen.findByText('Take a photo · 1 taken')).toBeInTheDocument();
		// The cook is still on the step they were on.
		expect(await exactly('Coat the chicken in panko.')).toBeInTheDocument();
	});

	it('counts a photograph taken in a Demo, and says the demo will not keep it (#403)', async () => {
		const kamosu = standIn(
			answers({
				edit_attempt: attempt({ photographs: [] }),
				instance_status: {
					version: '0.1.0',
					setup_complete: true,
					password_minimum: 15,
					passkey_origin: null,
					demo: true,
				},
			}),
		);
		render(CookingTestHarness, {
			props: { client: kamosu.client, branchId: 'b_1', keeping: keepingForTests() },
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'Start cooking' }));
		await openMore();
		const picker = await screen.findByLabelText('Take a photograph of this cooking');
		await fireEvent.change(picker, {
			target: { files: [new File(['a plate'], 'plate.jpg', { type: 'image/jpeg' })] },
		});
		await openMore();
		expect(await screen.findByText('Take a photo · 1 taken')).toBeInTheDocument();
		expect(
			await screen.findByText(/This is a demo, so photographs aren't kept/),
		).toBeInTheDocument();
	});

	// ---- how much are you making (#109, option B) ----------------------------

	/**
	 * The recipe as `get_recipe` answers once the Core has scaled it to eight
	 * servings: the panko's line doubled and converted in one slot, the chicken
	 * and the salt with nothing Kamosu could scale.
	 */
	function scaledTo(amount: string, noun: string): GetRecipeOutput {
		const recipe = answers().get_recipe as GetRecipeOutput;
		return {
			...recipe,
			versions: [
				{
					...recipe.versions[0],
					scaled_to: { amount, noun },
					measured: {
						// The whole line at the new amount, as the Core words it (#680).
						ingredients: [null, null, 'about 480 g panko', '1.6 l water', null],
						steps: [null, null, null, null],
					},
				},
			],
		};
	}

	/** A stand-in whose recipe comes back scaled once a Yield has been sent. */
	function scalingKamosu(over: Answers = {}) {
		let sent: StartAttemptOutput['cooking_yield'] = null;
		return {
			advance_attempt: () => {
				const call = kamosu?.calls.findLast((each) => each.operation === 'advance_attempt');
				sent = (call?.input as { cooking_yield?: typeof sent }).cooking_yield ?? sent;
				return attempt({ cooking_yield: sent });
			},
			get_recipe: () =>
				sent ? scaledTo(sent.amount, sent.noun) : (answers().get_recipe as GetRecipeOutput),
			...over,
		} satisfies Answers;
	}
	let kamosu: ReturnType<typeof standIn> | undefined;

	it('asks how much before a fresh cooking starts, and the foot starts it', async () => {
		kamosu = await cook({}, { asked: true });
		// The question is the heading, and nothing says a step has started
		// (#651, C2): no count, and no Back, since how much is no step.
		expect(
			await screen.findByRole('heading', { name: 'How much are you making?' }),
		).toBeInTheDocument();
		expect(screen.queryByText('Before you start')).toBeNull();
		expect(screen.queryByText(/^Step \d of/)).toBeNull();
		// The question stands where the Step will: the Step is not on screen yet.
		expect(screen.queryByText('Coat the chicken in panko.')).not.toBeInTheDocument();
		// The foot's one button starts the cooking, as the screen's main button.
		expect(screen.getAllByRole('button', { name: 'Start cooking' })).toHaveLength(1);
		expect(screen.getByRole('button', { name: 'Start cooking' })).toHaveClass('main-button');
		expect(screen.queryByRole('button', { name: 'Back' })).toBeNull();
		expect(screen.queryByRole('button', { name: 'Next step' })).not.toBeInTheDocument();
		// Four choices, each saying what it makes, the recipe as written chosen.
		const choices = screen.getAllByRole('radio');
		expect(choices.map((each) => each.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
			'Half 2 servings',
			'As written 4 servings',
			'Double 8 servings',
			'Triple 12 servings',
		]);
		expect(screen.getByRole('radio', { name: /As written/ })).toHaveAttribute(
			'aria-checked',
			'true',
		);
		// − and + are behind Another amount, since the four do the same job.
		expect(screen.queryByRole('button', { name: 'One more' })).toBeNull();
		expect(screen.getByText('Every amount changes to match.')).toBeVisible();

		await fireEvent.click(screen.getByRole('button', { name: 'Start cooking' }));
		expect(await exactly('Coat the chicken in panko.')).toBeInTheDocument();
		// As written is a choice too, and it costs nothing: no write was made.
		expect(kamosu.calls.filter((call) => call.operation === 'advance_attempt')).toHaveLength(0);
	});

	it('never asks a cooking that is already under way', async () => {
		await cook({ start_attempt: attempt({ current_step_index: 2 }) }, { asked: true });
		expect(await exactly('Pour in the water and simmer for about 7 minutes.')).toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'How much are you making?' })).toBeNull();
	});

	it('goes back to how much from the first step, so Back is never greyed out (#651)', async () => {
		await cook();
		const back = await screen.findByRole('button', { name: 'Back' });
		expect(back).toBeEnabled();
		await fireEvent.click(back);
		expect(
			await screen.findByRole('heading', { name: 'How much are you making?' }),
		).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: 'Start cooking' }));
		expect(await exactly('Coat the chicken in panko.')).toBeInTheDocument();
	});

	it("puts tonight's figures in a scaled Step's own words", async () => {
		// At twice the recipe the Step's own 800 ml was for half the pot: drawn
		// large beside a small grey (1.6 l), it was the figure a cook measured
		// (round 1 of #681's climb).
		const recipe = scaledTo('8', 'servings');
		const version = recipe.versions[0];
		const written = 'Pour in 800 ml water and simmer for about 7 minutes.';
		await cook({
			start_attempt: attempt({
				current_step_index: 2,
				cooking_yield: { amount: '8', noun: 'servings' },
			}),
			get_recipe: {
				...recipe,
				versions: [
					{
						...version,
						content: {
							...version.content,
							steps: version.content.steps.map((row, at) =>
								at === 2 ? { ...row, text: written } : row,
							),
						},
						measured: {
							...version.measured,
							steps: [null, null, [{ written: '800 ml', measured: '1.6 l' }], null],
						},
					},
				],
			},
		});
		const figure = await screen.findByText('1.6 l');
		expect(figure.closest('.text-step')?.textContent?.replace(/\s+/g, ' ').trim()).toBe(
			'Pour in 1.6 l water and simmer for about 7 minutes.',
		);
		expect(screen.queryByText('(1.6 l)')).not.toBeInTheDocument();
	});

	it("says Note a change holds the recipe's own amounts while cooking another", async () => {
		await cook({
			start_attempt: attempt({
				current_step_index: 1,
				cooking_yield: { amount: '8', noun: 'servings' },
			}),
			get_recipe: scaledTo('8', 'servings'),
		});
		await openMore();
		await fireEvent.click(await screen.findByRole('button', { name: 'Note a change' }));
		expect(
			await screen.findByText('These are the recipe’s own amounts, for 4 servings.'),
		).toBeInTheDocument();
		expect(await screen.findByDisplayValue('1 cup panko')).toBeInTheDocument();
	});

	it('stores the Yield on the Attempt, and the scaled amount leads', async () => {
		kamosu = await cook(scalingKamosu(), { asked: true });
		await fireEvent.click(await screen.findByRole('radio', { name: 'Double 8 servings' }));
		await vi.waitFor(() => {
			const sent = kamosu?.calls.filter((call) => call.operation === 'advance_attempt');
			expect(sent?.at(-1)?.input).toEqual({
				attempt_id: 'at_1',
				cooking_yield: { amount: '8', noun: 'servings' },
			});
		});
		// Chosen, and shown as chosen, before anything else.
		expect(await screen.findByRole('radio', { name: 'Double 8 servings' })).toHaveAttribute(
			'aria-checked',
			'true',
		);
		await fireEvent.click(screen.getByRole('button', { name: 'Start cooking' }));

		// The Core's scaled line at full size, standing alone: since #680 it says
		// what it is an amount of, and the recipe's own line no longer doubles
		// every row beneath it (#681).
		const scaled = await exactly('about 480 g panko');
		expect(scaled).toHaveClass('text-panel-figure');
		expect(screen.queryByText(/^recipe: /)).toBeNull();
		// A line with an amount Kamosu could not scale keeps its words and says so.
		expect(await exactly('2 chicken breasts')).toHaveClass('text-panel-figure');
		expect(screen.getByText('kept as written')).toBeInTheDocument();
		// Nothing about a recipe was written: only the Attempt moved.
		const wrote = kamosu.calls.map((call) => call.operation);
		expect(wrote).not.toContain('save_recipe_version');
		expect(wrote).not.toContain('set_as_cooked');
	});

	it('says nothing under a scaled line with no amount to scale (#680)', async () => {
		await cook(
			{
				start_attempt: attempt({
					current_step_index: 3,
					cooking_yield: { amount: '8', noun: 'servings' },
				}),
				get_recipe: scaledTo('8', 'servings'),
			},
			{ asked: true },
		);
		expect(await exactly('Season with salt and leave it alone.')).toBeInTheDocument();
		// The screen is cooking for eight, so the amounts are scaled.
		expect(screen.getByRole('button', { name: /8 servings/ })).toBeInTheDocument();
		expect(await exactly('a pinch of salt')).toBeInTheDocument();
		expect(screen.queryByText('kept as written')).toBeNull();
	});

	/**
	 * Cook at eight servings and write `2 cups panko` over the recipe's line,
	 * the Core answering with the As Cooked measured as `measuredFor` says (#713).
	 */
	/** The Attempt the Core answers once it holds `2 cups panko` (#713). */
	function pankoCookedAtEight(measuredFor: { amount: string; noun: string } | null) {
		// The editor holds the recipe's own amounts, so `2 cups panko` written
		// at eight servings of a recipe for four is four cups, and the Core says
		// so on the Attempt it answers the write with.
		const ingredients = INGREDIENTS.map((line) =>
			line.text === '1 cup panko' ? { ...line, text: '2 cups panko' } : line,
		);
		const recipe = scaledTo('8', 'servings').versions[0].content;
		return attempt({
			current_step_index: 1,
			cooking_yield: { amount: '8', noun: 'servings' },
			as_cooked: {
				version_id: 'v_cooked',
				content: { ...recipe, ingredients },
				against: { ingredients: [], steps: [] },
				promotion_declined: false,
				readings: [
					null,
					{ amount: '2', unit: null, target: 'chicken', lineage_id: null },
					{ amount: '2', unit: 'cups', target: 'panko', lineage_id: null },
					{ amount: '800', unit: 'ml', target: 'water', lineage_id: null },
					null,
				],
				measured: {
					ingredients: [null, null, 'about 960 g panko', '1.6 l water', null],
					scaled_to: measuredFor,
				},
			},
		});
	}

	async function changePankoAtEight(
		measuredFor: { amount: string; noun: string } | null,
		over: Answers = {},
	) {
		const kamosu = await cook({
			start_attempt: attempt({
				current_step_index: 1,
				cooking_yield: { amount: '8', noun: 'servings' },
			}),
			get_recipe: scaledTo('8', 'servings'),
			set_as_cooked: pankoCookedAtEight(measuredFor),
			...over,
		});
		await openMore();
		await fireEvent.click(await screen.findByRole('button', { name: /note a change/i }));
		await fireEvent.input(
			await screen.findByDisplayValue(
				(_, node) => (node as HTMLInputElement)?.value === '1 cup panko',
			),
			{ target: { value: '2 cups panko' } },
		);
		await fireEvent.click(await screen.findByRole('button', { name: /^done$/i }));
		return kamosu;
	}

	it('scales a line the cook changed, as the Core measured it (#713)', async () => {
		await changePankoAtEight({ amount: '8', noun: 'servings' });

		// The cook's own line scaled stands alone, as every scaled line does
		// (round 2 of #681's climb), and the recipe's line is struck under it
		// at the same amount, so the two say the same amount of the same thing
		// (round 5 of #681's climb).
		expect(await exactly('about 960 g panko')).toHaveClass('text-panel-figure');
		expect(screen.queryByText('2 cups panko')).not.toBeInTheDocument();
		expect(screen.getByText('about 480 g panko')).toHaveClass('line-through');
		expect(screen.queryByText('1 cup panko')).not.toBeInTheDocument();
	});

	it('keeps the measured line when a move answers from before the save (#713)', async () => {
		// `save` and the move go out together, and the move's answer can carry
		// the As Cooked as it stood before the save landed.
		const kamosu = await changePankoAtEight(
			{ amount: '8', noun: 'servings' },
			{
				advance_attempt: attempt({
					current_step_index: 1,
					cooking_yield: { amount: '8', noun: 'servings' },
				}),
			},
		);
		expect(await exactly('about 960 g panko')).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: /tick 2 chicken breasts/i }));
		await vi.waitFor(() =>
			expect(kamosu.calls.some((call) => call.operation === 'advance_attempt')).toBe(true),
		);
		expect(await exactly('about 960 g panko')).toBeInTheDocument();
	});

	it('takes the cook’s words measured from a later answer after a save held back (#713)', async () => {
		// A save the outbox held answers with the As Cooked as it was, and the
		// server's own, with the cook's words, comes on the next move.
		const kamosu = await changePankoAtEight(null, {
			set_as_cooked: attempt({
				current_step_index: 1,
				cooking_yield: { amount: '8', noun: 'servings' },
			}),
			advance_attempt: pankoCookedAtEight({ amount: '8', noun: 'servings' }),
		});
		expect(screen.queryByText('about 960 g panko')).not.toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: /tick 2 chicken breasts/i }));
		await vi.waitFor(() =>
			expect(kamosu.calls.some((call) => call.operation === 'advance_attempt')).toBe(true),
		);
		expect(await exactly('about 960 g panko')).toBeInTheDocument();
	});

	it('says nothing under a changed line measured for another amount (#713)', async () => {
		// Measured at the recipe as written, while eight servings are cooked:
		// as after a change of how much made with no network.
		await changePankoAtEight(null);
		const line = await exactly('2 cups panko');
		expect(line).toHaveClass('text-panel-figure');
		expect(screen.queryByText('about 960 g panko')).not.toBeInTheDocument();
		// Nor does it claim the line could not be scaled: the Core has not said.
		const row = line.closest('li') as HTMLElement;
		expect(within(row).queryByText('kept as written')).not.toBeInTheDocument();
	});

	it('scales a line the cook added on a Step, as the Core measured it (#713)', async () => {
		// Cooking eight servings of a recipe for four, the cook added
		// `1 tsp paprika` after the panko; the Core measured it at eight.
		const recipe = scaledTo('8', 'servings');
		const content = recipe.versions[0].content;
		type Row = NonNullable<StartAttemptOutput['as_cooked']>['against']['ingredients'][number];
		const same = (mine: number, theirs: number, kind: string, text: string): Row => ({
			from_branch_point: false,
			kind,
			mine: { index: mine, kind, text },
			place: { mine, theirs },
			state: 'same',
			theirs: { index: theirs, kind, text },
		});
		await cook({
			get_recipe: recipe,
			start_attempt: attempt({
				current_step_index: 1,
				cooking_yield: { amount: '8', noun: 'servings' },
				as_cooked: {
					against: {
						ingredients: [
							same(0, 0, 'section', 'For the cutlets'),
							same(1, 1, 'ingredient', '2 chicken breasts'),
							same(2, 2, 'ingredient', '1 cup panko'),
							{
								from_branch_point: false,
								kind: 'ingredient',
								mine: null,
								place: { mine: 3, theirs: 3 },
								state: 'only-theirs',
								theirs: { index: 3, kind: 'ingredient', text: '1 tsp paprika' },
							},
							same(3, 4, 'ingredient', '800 ml water'),
							same(4, 5, 'ingredient', 'a pinch of salt'),
						],
						steps: content.steps.map((step, index) => same(index, index, step.kind, step.text)),
					},
					content: {
						...content,
						ingredients: [
							INGREDIENTS[0],
							INGREDIENTS[1],
							INGREDIENTS[2],
							{ kind: 'ingredient', text: '1 tsp paprika' },
							INGREDIENTS[3],
							INGREDIENTS[4],
						],
					},
					promotion_declined: false,
					version_id: 'v_2',
					readings: [
						null,
						{ amount: '2', unit: null, target: 'chicken', lineage_id: null },
						{ amount: '1', unit: 'cup', target: 'panko', lineage_id: null },
						{ amount: '1', unit: 'tsp', target: 'paprika', lineage_id: null },
						{ amount: '800', unit: 'ml', target: 'water', lineage_id: null },
						null,
					],
					measured: {
						ingredients: [null, null, 'about 480 g panko', '2 tsp paprika', '1.6 l water', null],
						scaled_to: { amount: '8', noun: 'servings' },
					},
				},
			}),
		});
		expect(await exactly('2 tsp paprika')).toBeInTheDocument();
		expect(screen.queryByText('1 tsp paprika')).not.toBeInTheDocument();
	});

	it('offers only multipliers where the recipe never said what it makes', async () => {
		const recipe = answers().get_recipe as GetRecipeOutput;
		const noYield: GetRecipeOutput = {
			...recipe,
			versions: [
				{ ...recipe.versions[0], content: { ...recipe.versions[0].content, yield: null } },
			],
		};
		kamosu = await cook({ get_recipe: noYield }, { asked: true });
		expect(
			await screen.findByText('This recipe doesn’t say how much it makes, so choose a multiple.'),
		).toBeInTheDocument();
		expect(screen.getAllByRole('radio').map((each) => each.textContent?.trim())).toEqual([
			'Half the recipe',
			'The recipe as written',
			'Twice the recipe',
			'Three times the recipe',
		]);
		// Nothing to count from, so no − and +, and nothing that opens them.
		expect(screen.queryByRole('button', { name: 'One more' })).not.toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Another amount' })).toBeNull();
		await fireEvent.click(screen.getByRole('radio', { name: 'Twice the recipe' }));
		await vi.waitFor(() => {
			const sent = kamosu?.calls.find((call) => call.operation === 'advance_attempt');
			expect(sent?.input).toEqual({ attempt_id: 'at_1', cooking_yield: { amount: '2', noun: '' } });
		});
	});

	it('moves between the four choices with the arrow keys, as radio buttons do (#651)', async () => {
		kamosu = await cook(scalingKamosu(), { asked: true });
		const written = await screen.findByRole('radio', { name: 'As written 4 servings' });
		// One Tab stop for the group: the chosen one.
		expect(written).toHaveAttribute('tabindex', '0');
		expect(screen.getByRole('radio', { name: 'Double 8 servings' })).toHaveAttribute(
			'tabindex',
			'-1',
		);
		await fireEvent.keyDown(written, { key: 'ArrowDown' });
		await vi.waitFor(() => {
			const sent = kamosu?.calls.filter((call) => call.operation === 'advance_attempt');
			expect(sent?.at(-1)?.input).toEqual({
				attempt_id: 'at_1',
				cooking_yield: { amount: '8', noun: 'servings' },
			});
		});
		expect(screen.getByRole('radio', { name: 'Double 8 servings' })).toHaveFocus();
	});

	it('says a multiple of a recipe with no yield in words on the step (#651, C3)', async () => {
		const recipe = answers().get_recipe as GetRecipeOutput;
		const noYield: GetRecipeOutput = {
			...recipe,
			versions: [
				{ ...recipe.versions[0], content: { ...recipe.versions[0].content, yield: null } },
			],
		};
		await cook({
			get_recipe: noYield,
			start_attempt: attempt({ current_step_index: 2, cooking_yield: { amount: '2', noun: '' } }),
		});
		expect(
			await screen.findByRole('button', { name: 'Cooking Twice the recipe. Change how much' }),
		).toHaveTextContent('Twice the recipe');
	});

	it('closes More when how much is asked again', async () => {
		await cook(
			{
				start_attempt: attempt({
					current_step_index: 2,
					cooking_yield: { amount: '8', noun: 'servings' },
				}),
				get_recipe: scaledTo('8', 'servings'),
			},
			{ asked: true },
		);
		await openMore();
		expect(screen.getByRole('button', { name: 'Throw this cooking away' })).toBeInTheDocument();
		await fireEvent.click(
			screen.getByRole('button', { name: 'Cooking 8 servings. Change how much' }),
		);
		expect(screen.queryByRole('button', { name: 'Throw this cooking away' })).toBeNull();
	});

	it('counts one serving at a time in the recipe’s own noun', async () => {
		kamosu = await cook(scalingKamosu(), { asked: true });
		await fireEvent.click(await screen.findByRole('button', { name: 'Another amount' }));
		await fireEvent.click(await screen.findByRole('button', { name: 'One more' }));
		await vi.waitFor(() => {
			const sent = kamosu?.calls.find((call) => call.operation === 'advance_attempt');
			expect(sent?.input).toEqual({
				attempt_id: 'at_1',
				cooking_yield: { amount: '5', noun: 'servings' },
			});
		});
	});

	it('says one serving, not one servings (#705)', async () => {
		kamosu = await cook(scalingKamosu(), { asked: true });
		await fireEvent.click(await screen.findByRole('button', { name: 'Another amount' }));
		for (const count of ['3', '2', '1']) {
			await fireEvent.click(await screen.findByRole('button', { name: 'One fewer' }));
			await vi.waitFor(() => {
				const sent = kamosu?.calls.findLast((call) => call.operation === 'advance_attempt');
				expect(sent?.input).toMatchObject({ cooking_yield: { amount: count } });
			});
		}
		await vi.waitFor(() =>
			expect(
				screen
					.getAllByText(/serving/)
					.map((each) => each.closest('p')?.textContent?.replace(/\s+/g, ' ').trim()),
			).toContain('1 serving'),
		);
		await fireEvent.click(screen.getByRole('button', { name: 'Start cooking' }));
		expect(
			await screen.findByRole('button', { name: 'Cooking 1 serving. Change how much' }),
		).toBeInTheDocument();
	});

	it('asks again from the row mid-cook, and the foot goes back to the step', async () => {
		await cook(
			scalingKamosu({
				start_attempt: attempt({
					current_step_index: 2,
					cooking_yield: { amount: '8', noun: 'servings' },
				}),
				get_recipe: scaledTo('8', 'servings'),
			}),
		);
		await fireEvent.click(
			await screen.findByRole('button', { name: 'Cooking 8 servings. Change how much' }),
		);
		expect(
			await screen.findByRole('heading', { name: 'How much are you making?' }),
		).toBeInTheDocument();
		expect(screen.getByRole('radio', { name: 'Double 8 servings' })).toHaveAttribute(
			'aria-checked',
			'true',
		);
		expect(screen.queryByRole('button', { name: 'Back' })).toBeNull();
		await fireEvent.click(screen.getByRole('button', { name: 'Back to step 2' }));
		expect(await exactly('Pour in the water and simmer for about 7 minutes.')).toBeInTheDocument();
	});

	it('says the amounts wait for Kamosu, rather than showing ones for another amount', async () => {
		// With no network the phone holds the change (#77) and answers the
		// recipe it kept, scaled to nothing: the Core has not heard yet.
		kamosu = await cook(
			{ advance_attempt: attempt({ cooking_yield: { amount: '8', noun: 'servings' } }) },
			{ asked: true },
		);
		await screen.findByRole('radio', { name: 'Double 8 servings' });
		reached(false);
		try {
			await fireEvent.click(screen.getByRole('radio', { name: 'Double 8 servings' }));
			await fireEvent.click(screen.getByRole('button', { name: 'Start cooking' }));
			expect(
				await screen.findByText('Amounts scale to 8 servings once Kamosu can be reached.'),
			).toBeInTheDocument();
			// The written line stands, drawn soft, and no figure worked out for 4
			// sits under it.
			expect(await exactly('1 cup panko')).toHaveClass('text-cook-soft');
			expect(screen.queryByText('about 240 g')).not.toBeInTheDocument();
		} finally {
			reached(true);
		}
	});

	// ---- the amounts catch up with the choice (#674) -------------------------

	/** The recipe scaled to `amount` servings, its panko line saying how much. */
	function scaledPanko(amount: string): GetRecipeOutput {
		const recipe = scaledTo(amount, 'servings');
		const version = recipe.versions[0];
		const ingredients = [null, null, `about ${Number(amount) * 60} g`, 'about 1.6 l', null];
		return {
			...recipe,
			versions: [{ ...version, measured: { ...version.measured, ingredients } }],
		};
	}

	/**
	 * A stand-in that holds what was last sent as how much, as the Core does,
	 * and answers each read of the recipe through `read`: what the Core would
	 * answer now, and how many reads came before this one.
	 */
	function holdingKamosu(
		read: (
			held: StartAttemptOutput['cooking_yield'],
			asked: number,
		) => GetRecipeOutput | Refusal | Promise<GetRecipeOutput>,
	): Answers {
		let held: StartAttemptOutput['cooking_yield'] = null;
		let asked = 0;
		return {
			advance_attempt: (input) => {
				if (input.cooking_yield !== undefined) held = input.cooking_yield;
				return attempt({ cooking_yield: held });
			},
			get_recipe: () => read(held, asked++),
		};
	}

	const plain = () => answers().get_recipe as GetRecipeOutput;

	/** Every read of the recipe the screen asked for. */
	const recipeReads = (kamosu: ReturnType<typeof standIn>) =>
		kamosu.calls.filter((call) => call.operation === 'get_recipe');

	/** Whether this figure is on screen now, as `exactly` finds one. */
	const shows = (text: string) =>
		screen.queryAllByText(
			(_, node) => node?.textContent?.trim() === text && node.children.length === 0,
		).length > 0;

	it('reads the recipe again when the read after a change of how much fails (#674)', async () => {
		let failed = 0;
		await cook(
			holdingKamosu((held) => {
				if (held === null) return plain();
				if (failed++ === 0) return { refuse: 'internal', message: 'The read went wrong.' };
				return scaledPanko(held.amount);
			}),
			{ asked: true },
		);
		await fireEvent.click(await screen.findByRole('radio', { name: 'Double 8 servings' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Start cooking' }));
		await vi.waitFor(() => expect(shows('about 480 g')).toBe(true), { timeout: 5_000 });
		expect(screen.queryByText(/once Kamosu can be reached/)).toBeNull();
	});

	it('reads the recipe again when it came back before the change was applied (#674)', async () => {
		let early = 0;
		await cook(
			holdingKamosu((held) =>
				held === null || early++ === 0 ? plain() : scaledPanko(held.amount),
			),
			{ asked: true },
		);
		await fireEvent.click(await screen.findByRole('radio', { name: 'Double 8 servings' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Start cooking' }));
		// Kamosu is in reach, so the wait is its answer, not the network.
		expect(await screen.findByText('Working out the amounts for 8 servings…')).toBeInTheDocument();
		expect(screen.queryByText(/once Kamosu can be reached/)).toBeNull();
		await vi.waitFor(() => expect(shows('about 480 g')).toBe(true), { timeout: 5_000 });
		expect(screen.queryByText(/Working out the amounts/)).toBeNull();
		expect(await exactly('about 480 g')).not.toHaveClass('text-cook-soft');
	});

	it('shows the later choice where its read overtakes the read of an amount carried in (#674)', async () => {
		// Sent from a recipe page scaled to eight servings (#109).
		const address = vi
			.spyOn(page, 'url', 'get')
			.mockReturnValue(new URL('https://kamosu.example/cook/b_1?amount=8&noun=servings'));
		let release = () => {};
		const slow = new Promise<void>((resolve) => (release = resolve));
		try {
			const kamosu = await cook(
				holdingKamosu(async (held) => {
					if (held === null) return plain();
					// The carried amount's read is the slow one.
					if (held.amount === '8') await slow;
					return scaledPanko(held.amount);
				}),
				{ asked: true },
			);
			await vi.waitFor(() => expect(recipeReads(kamosu).length).toBeGreaterThanOrEqual(2));
			await fireEvent.click(await screen.findByRole('radio', { name: 'Triple 12 servings' }));
			await vi.waitFor(() => expect(recipeReads(kamosu)).toHaveLength(3));
			await fireEvent.click(screen.getByRole('button', { name: 'Start cooking' }));
			expect(await exactly('about 720 g')).toBeInTheDocument();
			// The carried amount's answer lands last, and is not shown over it.
			release();
			await new Promise((resolve) => setTimeout(resolve, 50));
			expect(shows('about 720 g')).toBe(true);
			expect(shows('about 480 g')).toBe(false);
		} finally {
			release();
			address.mockRestore();
		}
	});

	it('keeps the opening read where a refresh asked after it fails (#674)', async () => {
		let release = () => {};
		const slow = new Promise<void>((resolve) => (release = resolve));
		let reads = 0;
		try {
			const kamosu = await cook(
				{
					get_recipe: async () => {
						reads += 1;
						if (reads === 1) {
							await slow;
							return plain();
						}
						return { refuse: 'internal', message: 'The read went wrong.' };
					},
				},
				{ asked: true },
			);
			await vi.waitFor(() => expect(recipeReads(kamosu)).toHaveLength(1));
			// The worker says the recipe changed while the first read is out.
			refreshed.set('get_recipe', (refreshed.get('get_recipe') ?? 0) + 1);
			await vi.waitFor(() => expect(recipeReads(kamosu)).toHaveLength(2));
			release();
			expect(
				await screen.findByRole('heading', { name: 'How much are you making?' }),
			).toBeInTheDocument();
		} finally {
			release();
		}
	});

	it('waits while Kamosu cannot be reached, and scales once the outbox has sent (#674)', async () => {
		let landed = false;
		const kamosu = await cook(
			{
				// The phone holds the change (#77) and answers for the server.
				advance_attempt: attempt({ cooking_yield: { amount: '8', noun: 'servings' } }),
				get_recipe: () => (landed ? scaledPanko('8') : plain()),
			},
			{ asked: true },
		);
		await screen.findByRole('radio', { name: 'Double 8 servings' });
		reached(false);
		try {
			await fireEvent.click(screen.getByRole('radio', { name: 'Double 8 servings' }));
			await fireEvent.click(screen.getByRole('button', { name: 'Start cooking' }));
			expect(
				await screen.findByText('Amounts scale to 8 servings once Kamosu can be reached.'),
			).toBeInTheDocument();
			// Nothing is asked over and over of a server out of reach.
			const reads = recipeReads(kamosu).length;
			await new Promise((resolve) => setTimeout(resolve, 1_500));
			expect(recipeReads(kamosu)).toHaveLength(reads);
			expect(screen.getByText(/once Kamosu can be reached/)).toBeInTheDocument();

			// Back, and the outbox sends what it held, which tells the page.
			landed = true;
			reached(true);
			refreshed.set('get_recipe', (refreshed.get('get_recipe') ?? 0) + 1);
			expect(await exactly('about 480 g')).toBeInTheDocument();
			expect(screen.queryByText(/once Kamosu can be reached/)).toBeNull();
		} finally {
			reached(true);
		}
	});
});

/**
 * A Step's photograph at the stove (#110, K3): a small square the words wrap
 * around, opening across the screen — never a picture that takes the Step's
 * room, which is the largest type in the app for a reason.
 */
describe('a Step’s photograph while cooking', () => {
	/** The recipe with a photograph on its first Step, the one a cook opens on. */
	const withPhoto = (): GetRecipeOutput => {
		const recipe = answers().get_recipe as GetRecipeOutput;
		const version = recipe.versions[0];
		return {
			...recipe,
			versions: [
				{
					...version,
					content: {
						...version.content,
						steps: STEPS.map((row, index) => (index === 1 ? { ...row, photo: 'p_coated' } : row)),
					},
				},
			],
		};
	};

	it('stands beside the step count with its word, and opens across the screen', async () => {
		await cook({ get_recipe: withPhoto() });
		const open = await screen.findByRole('button', { name: 'Photo' });
		expect(open.querySelector('img')).toHaveAttribute('src', '/api/photographs/p_coated/card');
		// On the row that says which Step this is, out of the Step's words,
		// which keep their whole width (round 4 of #681's climb).
		expect(open.closest('.text-step')).toBeNull();
		expect(open.parentElement).toHaveTextContent(/Step 1 of \d/);

		await fireEvent.click(open);
		const large = await screen.findByRole('dialog', { name: 'A photograph of step 1' });
		expect(within(large).getByAltText('A photograph of step 1')).toHaveAttribute(
			'src',
			'/api/photographs/p_coated/page',
		);
		await fireEvent.click(within(large).getByRole('button', { name: 'Close' }));
		expect(screen.queryByRole('dialog')).toBeNull();
	});

	it('draws nothing on a Step the recipe gives no photograph', async () => {
		await cook();
		expect(await exactly('Coat the chicken in panko.')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Photo' })).toBeNull();
	});
});

/**
 * When Kamosu itself goes wrong mid-cook (#119, option B, chosen 23
 * September 2026). Everywhere else #98's card says so at the top of the page;
 * here it would cost the Step its height and put *Got it* under a wet thumb, so
 * the cooking screen says it in two words on the row it already has, and the
 * card is one deliberate tap away.
 */
describe('a mistake in Kamosu, mid-cook (#119)', () => {
	beforeEach(() => {
		putMistakeAway();
		// Written where a developer looks, which here is the test's own output.
		vi.spyOn(console, 'error').mockImplementation(() => {});
	});
	afterEach(() => {
		putMistakeAway();
		vi.restoreAllMocks();
	});

	/** A real Error from behind the seam, not a refusal: nothing answered it. */
	const goWrong = async (kamosu: ReturnType<typeof standIn>) =>
		expect(kamosu.client.listJobs()).rejects.toThrow(/did not answer/);

	it('says so in the top row, and the Step keeps its place and its size', async () => {
		const kamosu = await cook();
		await exactly('Coat the chicken in panko.');
		expect(screen.queryByRole('button', { name: 'Kamosu went wrong' })).toBeNull();

		await goWrong(kamosu);

		const mark = await screen.findByRole('button', { name: 'Kamosu went wrong' });
		expect(mark).toHaveTextContent('Went wrong');
		// On the row with the other ways off the step, not above the Step. The
		// false start is under the ⋯ since #222; the ⋯ is on the row.
		expect(mark.closest('header')).toBe(
			screen.getByRole('button', { name: /^more$/i }).closest('header'),
		);
		// Said aloud as well as shown: a screen reader hears it without finding it.
		expect(screen.getByRole('alert')).toHaveTextContent('Kamosu went wrong');
		// The card is not drawn, so there is nothing new under a thumb that did
		// not ask for it, and the Step is still the largest type on screen.
		expect(screen.queryByText(/a mistake in Kamosu, not you/)).toBeNull();
		expect(screen.queryByRole('button', { name: 'Got it' })).toBeNull();
		expect(await exactly('Coat the chicken in panko.')).toHaveClass('text-step');
	});

	it('opens the whole card on a tap, and Got it puts it away', async () => {
		const kamosu = await cook();
		await goWrong(kamosu);

		await fireEvent.click(await screen.findByRole('button', { name: 'Kamosu went wrong' }));
		const card = await screen.findByRole('dialog', { name: 'Kamosu went wrong' });
		expect(card).toHaveFocus();
		expect(within(card).getByText(/a mistake in Kamosu, not you/)).toBeInTheDocument();

		await fireEvent.click(within(card).getByRole('button', { name: 'Got it' }));
		expect(screen.queryByRole('dialog', { name: 'Kamosu went wrong' })).toBeNull();
		expect(screen.queryByRole('button', { name: 'Kamosu went wrong' })).toBeNull();
		expect(await exactly('Coat the chicken in panko.')).toBeInTheDocument();
	});

	it('never opens the card by itself when the next mistake comes', async () => {
		const kamosu = await cook();
		await goWrong(kamosu);
		await fireEvent.click(await screen.findByRole('button', { name: 'Kamosu went wrong' }));
		await fireEvent.click(await screen.findByRole('button', { name: 'Got it' }));

		await expect(kamosu.client.listKitchens()).rejects.toThrow();

		expect(await screen.findByRole('button', { name: 'Kamosu went wrong' })).toBeInTheDocument();
		expect(screen.queryByRole('dialog', { name: 'Kamosu went wrong' })).toBeNull();
	});

	it('draws the card where there is no Step, with the way out the screen lacks', async () => {
		// The cooking could not be started: no Step, no top row for the mark,
		// and no way back of the screen's own.
		const kamosu = standIn(answers({ start_attempt: { refuse: 'not_found' } }));
		render(CookingTestHarness, { props: { client: kamosu.client, branchId: 'b_1' } });
		await screen.findByText(/could/i);

		await goWrong(kamosu);

		const card = await screen.findByRole('region', { name: 'Kamosu went wrong' });
		expect(within(card).getByText(/a mistake in Kamosu, not you/)).toBeInTheDocument();
		expect(within(card).getByRole('link', { name: 'Back to the recipe' })).toHaveAttribute(
			'href',
			'/recipes/b_1',
		);
		expect(within(card).queryByRole('button', { name: 'Got it' })).toBeNull();
	});

	it('draws the card over a cooking thrown away, beside the way back it has', async () => {
		const kamosu = await cook();
		await fireEvent.click(await screen.findByRole('button', { name: /^more$/i }));
		await fireEvent.click(await screen.findByRole('button', { name: /throw this cooking away/i }));
		await fireEvent.click(await screen.findByRole('button', { name: /throw it away/i }));
		await screen.findByRole('status');

		await goWrong(kamosu);

		const card = await screen.findByRole('region', { name: 'Kamosu went wrong' });
		await fireEvent.click(within(card).getByRole('button', { name: 'Got it' }));
		expect(screen.queryByRole('region', { name: 'Kamosu went wrong' })).toBeNull();
		expect(screen.getByRole('link', { name: 'Back to the recipe' })).toBeInTheDocument();
	});

	it('raises nothing for a refusal the Core meant to send', async () => {
		const kamosu = await cook({ list_jobs: { refuse: 'not_found' } });
		await expect(kamosu.client.listJobs()).rejects.toThrow();
		await Promise.resolve();
		expect(screen.queryByRole('button', { name: 'Kamosu went wrong' })).toBeNull();
	});
});
