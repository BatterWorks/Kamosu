/**
 * The screen-seam test: the Plan, against a stand-in Kamosu (#486).
 *
 * The stand-in checks every answer against the shape the Catalogue declares,
 * so a test here can lie about the values and never about the shape. What it
 * is for is what a behaviour test cannot see: the week drawn as option B of
 * #486's prototype, a planned recipe opening at its planned Yield, one that
 * can no longer be read saying so, and an empty week saying how to plan. And
 * from #487's: the week's meals with no day above its days, written words,
 * the + on each day, moving, planning again and the order, and dragging on
 * the wide layout.
 */

import { describe, expect, it, vi } from 'vitest';
import { fireEvent, screen, within } from '@testing-library/svelte';
import Page from './+page.svelte';
import { renderScreen } from '../../testing/render';
import type {
	GetMealPlanInput,
	GetMealPlanOutput,
	SendToShoppingListOutput,
} from '$lib/api/catalogue';
import { landedSend } from '$lib/offline/device.svelte';
import { addDays, mondayOf, today } from '$lib/plan-days';
import { aPlannedDay } from '$lib/dates';

type Meal = GetMealPlanOutput['planned_meals'][number];

const monday = mondayOf(today());

const meal = (over: Partial<Meal> = {}): Meal => ({
	planned_meal_id: 'pm_0000000000000001',
	week: monday,
	day: addDays(monday, 3),
	position: 0,
	branch_id: 'b_katsu',
	words: null,
	title: 'Katsu Curry',
	gone: false,
	planned_yield: { amount: '6', noun: 'servings' },
	written_yield: { amount: '4', noun: 'servings' },
	main_photo: null,
	sent_factor: null,
	sent_at: null,
	...over,
});

/** This week holds what a test says; every other week holds nothing. */
const weekOf =
	(meals: Meal[]) =>
	({ from }: GetMealPlanInput): GetMealPlanOutput => ({
		planned_meals: from === monday ? meals : [],
	});

describe('the Plan', () => {
	it('draws the week Monday to Sunday and opens a planned recipe at its planned Yield', async () => {
		const { kamosu } = renderScreen(Page, {
			get_meal_plan: weekOf([
				meal(),
				meal({
					planned_meal_id: 'pm_0000000000000002',
					position: 1,
					branch_id: 'b_ceviche',
					title: 'Ceviche',
					planned_yield: null,
					written_yield: { amount: '2', noun: 'servings' },
				}),
				meal({
					planned_meal_id: 'pm_0000000000000003',
					day: addDays(monday, 5),
					branch_id: 'b_pie',
					title: 'Chicken pie',
					planned_yield: { amount: '2', noun: '' },
					written_yield: { amount: '4 to 6', noun: 'servings' },
				}),
			]),
		});

		const katsu = await screen.findByRole('link', { name: /Katsu Curry/ });
		expect(katsu).toHaveAttribute('href', '/recipes/b_katsu?amount=6&noun=servings');
		expect(katsu).toHaveTextContent('For 6 servings');
		// As written: no amount in the address, and the recipe's own in words.
		const ceviche = screen.getByRole('link', { name: /Ceviche/ });
		expect(ceviche).toHaveAttribute('href', '/recipes/b_ceviche?written');
		expect(ceviche).toHaveTextContent('For 2 servings');
		expect(screen.getByRole('link', { name: /Chicken pie/ })).toHaveTextContent('×2 the recipe');
		// The two on Thursday share its row, in the order they were put there.
		const thursday = katsu.closest('section') as HTMLElement;
		expect(
			within(thursday)
				.getAllByRole('link')
				.map((link) => link.getAttribute('href')?.split('?')[0]),
		).toEqual(['/recipes/b_katsu', '/recipes/b_ceviche']);
		// The week's list with no day, then its seven days.
		expect(document.querySelectorAll('section')).toHaveLength(8);
		expect(screen.getAllByText('Nothing planned')).toHaveLength(5);
		expect(kamosu.calls).toContainEqual({
			operation: 'get_meal_plan',
			input: { from: monday, to: addDays(monday, 6) },
		});
	});

	it('says how to plan something in a week with nothing planned', async () => {
		renderScreen(Page, { get_meal_plan: { planned_meals: [] } });

		expect(await screen.findByText('Nothing planned for this week.')).toBeInTheDocument();
		expect(screen.getByText(/press Plan this to put it on a day/)).toBeInTheDocument();
		expect(screen.getByRole('link', { name: 'Choose a recipe' })).toHaveAttribute(
			'href',
			'/recipes',
		);
	});

	it('moves a week at a time, and back to this one', async () => {
		const { kamosu } = renderScreen(Page, { get_meal_plan: weekOf([meal()]) });
		await screen.findByRole('link', { name: /Katsu Curry/ });

		await fireEvent.click(screen.getByRole('button', { name: 'The week after' }));
		expect(await screen.findByText('Next week')).toBeInTheDocument();
		expect(await screen.findByText('Nothing planned for this week.')).toBeInTheDocument();
		expect(kamosu.calls.at(-1)?.input).toEqual({
			from: addDays(monday, 7),
			to: addDays(monday, 13),
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Back to this week' }));
		expect(await screen.findByRole('link', { name: /Katsu Curry/ })).toBeInTheDocument();
		expect(screen.getByText('This week')).toBeInTheDocument();
	});

	it('changes how much is planned from its sheet, which saves as it is tapped', async () => {
		const { kamosu } = renderScreen(Page, {
			get_meal_plan: weekOf([meal()]),
			set_planned_yield: ({ planned_meal_id, planned_yield }) => ({
				planned_meal: meal({ planned_meal_id, planned_yield }),
			}),
		});
		await screen.findByRole('link', { name: /Katsu Curry/ });

		await fireEvent.click(screen.getByRole('button', { name: 'Change' }));
		const sheet = await screen.findByRole('dialog');
		await fireEvent.click(within(sheet).getByRole('button', { name: 'One more' }));

		expect(kamosu.calls.at(-1)).toEqual({
			operation: 'set_planned_yield',
			input: {
				planned_meal_id: 'pm_0000000000000001',
				planned_yield: { amount: '7', noun: 'servings' },
			},
		});
		expect(
			await within(sheet).findByRole('link', { name: 'Open it for 7 servings' }),
		).toHaveAttribute('href', '/recipes/b_katsu?amount=7&noun=servings');
	});

	it('keeps a recipe that can no longer be read, by name, and takes it off', async () => {
		const { kamosu } = renderScreen(Page, {
			get_meal_plan: weekOf([
				meal({ title: 'Shakshuka', gone: true, planned_yield: null, written_yield: null }),
			]),
			remove_planned_meal: { planned_meal: null },
		});

		expect(await screen.findByText('Shakshuka')).toBeInTheDocument();
		expect(screen.getByText('Can no longer be read')).toBeInTheDocument();
		expect(screen.queryByRole('link', { name: /Shakshuka/ })).toBeNull();

		await fireEvent.click(screen.getByRole('button', { name: 'Remove' }));
		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).getByText(/no longer shared with you/)).toBeInTheDocument();
		expect(within(sheet).queryByRole('button', { name: 'One more' })).toBeNull();
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Take it off the plan' }));

		expect(kamosu.calls.at(-1)).toEqual({
			operation: 'remove_planned_meal',
			input: { planned_meal_id: 'pm_0000000000000001' },
		});
		expect(await screen.findByText('Nothing planned for this week.')).toBeInTheDocument();
	});

	it('says a recipe planned as written that names no Yield is as written', async () => {
		renderScreen(Page, {
			get_meal_plan: weekOf([meal({ planned_yield: null, written_yield: null })]),
		});

		const katsu = await screen.findByRole('link', { name: /Katsu Curry/ });
		expect(katsu).toHaveTextContent('As written');
		expect(katsu).toHaveAttribute('href', '/recipes/b_katsu?written');
	});

	it('says another week could not be read rather than drawing this one empty', async () => {
		renderScreen(Page, {
			get_meal_plan: ({ from }) =>
				from === monday ? { planned_meals: [meal()] } : { refuse: 'internal' as const },
		});
		await screen.findByRole('link', { name: /Katsu Curry/ });

		await fireEvent.click(screen.getByRole('button', { name: 'The week after' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('The plan could not be read');
		expect(screen.queryByText('Nothing planned')).toBeNull();

		// Back to a week that reads, the failure is gone with the week it was.
		await fireEvent.click(screen.getByRole('button', { name: 'Back to this week' }));
		expect(await screen.findByRole('link', { name: /Katsu Curry/ })).toBeInTheDocument();
		expect(screen.queryByRole('alert')).toBeNull();
	});

	it('lays the week out as seven columns of cards on the wide layout', async () => {
		renderScreen(Page, { get_meal_plan: weekOf([meal()]) }, undefined, 'wide');

		const katsu = await screen.findByRole('link', { name: /Katsu Curry/ });
		expect(katsu.closest('.grid-cols-7')).not.toBeNull();
		expect(screen.getAllByText('Nothing')).toHaveLength(6);
	});
});

describe('the Plan, a week with no day and written words (#487)', () => {
	const words = (over: Partial<Meal> = {}): Meal =>
		meal({
			planned_meal_id: 'pm_00000000000000a1',
			branch_id: null,
			words: 'Leftovers',
			title: null,
			planned_yield: null,
			written_yield: null,
			...over,
		});
	const anyDay = (over: Partial<Meal> = {}): Meal =>
		meal({
			planned_meal_id: 'pm_00000000000000b1',
			day: null,
			branch_id: 'b_korean',
			title: 'Korean Fried Chicken',
			planned_yield: null,
			...over,
		});
	const answered = ({ planned_meal_id }: { planned_meal_id: string }) => ({
		planned_meal: meal({ planned_meal_id }),
	});

	it("draws the week's meals with no day above Monday, and says so when there are none", async () => {
		renderScreen(Page, {
			get_meal_plan: ({ from }) => ({
				planned_meals: from === monday ? [anyDay()] : [meal({ week: from, day: addDays(from, 1) })],
			}),
		});

		const korean = await screen.findByRole('link', { name: /Korean Fried Chicken/ });
		const sections = [...document.querySelectorAll('section')];
		expect(sections[0]).toContainElement(korean);
		expect(sections[0]).toHaveTextContent('Any day');

		await fireEvent.click(screen.getByRole('button', { name: 'The week after' }));
		expect(
			await screen.findByText(
				'Nothing yet. A recipe planned for the week, with no day, waits here.',
			),
		).toBeInTheDocument();
	});

	it('draws written words with no photo and nothing to open, and changes them on Done', async () => {
		const { kamosu } = renderScreen(Page, {
			get_meal_plan: weekOf([words()]),
			set_planned_words: ({ planned_meal_id, words: typed }) => ({
				planned_meal: words({ planned_meal_id, words: typed }),
			}),
		});

		const written = await screen.findByText('Leftovers');
		expect(written.closest('a')).toBeNull();
		expect(written.closest('li')?.querySelector('img')).toBeNull();

		await fireEvent.click(screen.getByRole('button', { name: 'Change' }));
		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).queryByRole('link', { name: /Open it/ })).toBeNull();
		expect(within(sheet).queryByRole('button', { name: 'One more' })).toBeNull();
		await fireEvent.input(within(sheet).getByLabelText('The words'), {
			target: { value: 'Leftovers and a salad ' },
		});
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Done' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'set_planned_words',
			input: { planned_meal_id: 'pm_00000000000000a1', words: 'Leftovers and a salad' },
		});
		expect(await screen.findByText('Leftovers and a salad')).toBeInTheDocument();
	});

	it('writes words on a day from its +, and on the week from the list with no day', async () => {
		const { kamosu } = renderScreen(Page, {
			get_meal_plan: weekOf([meal()]),
			plan_words: ({ words: typed }) => ({ planned_meal: words({ words: typed }) }),
		});
		await screen.findByRole('link', { name: /Katsu Curry/ });
		const friday = addDays(monday, 4);

		const plus = screen.getAllByRole('button', { name: /^Add to / });
		expect(plus).toHaveLength(8);
		await fireEvent.click(plus[5]);
		const sheet = await screen.findByRole('dialog');
		const put = within(sheet).getByRole('button', { name: /^Put it on / });
		expect(put).toBeDisabled();
		expect(within(sheet).getByRole('link', { name: 'Choose a recipe to plan' })).toHaveAttribute(
			'href',
			'/recipes',
		);
		await fireEvent.input(within(sheet).getByLabelText('Write something'), {
			target: { value: 'Eating out' },
		});
		await fireEvent.click(put);
		expect(kamosu.calls).toContainEqual({
			operation: 'plan_words',
			input: { words: 'Eating out', day: friday },
		});
		expect(screen.queryByRole('dialog')).toBeNull();

		await fireEvent.click(screen.getByRole('button', { name: 'Add to any day this week' }));
		const again = await screen.findByRole('dialog');
		await fireEvent.input(within(again).getByLabelText('Write something'), {
			target: { value: 'Soup' },
		});
		await fireEvent.click(
			within(again).getByRole('button', { name: 'Put it on any day this week' }),
		);
		expect(kamosu.calls).toContainEqual({
			operation: 'plan_words',
			input: { words: 'Soup', week: monday },
		});
	});

	it('moves a meal to a week with no day from its sheet, and says where it went', async () => {
		const { kamosu } = renderScreen(Page, {
			get_meal_plan: weekOf([meal()]),
			move_planned_meal: answered,
		});
		await screen.findByRole('link', { name: /Katsu Curry/ });

		await fireEvent.click(screen.getByRole('button', { name: 'Change' }));
		await fireEvent.click(
			within(await screen.findByRole('dialog')).getByRole('button', { name: 'Move' }),
		);
		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).getByText(/^Now on /)).toBeInTheDocument();
		await fireEvent.click(within(sheet).getByRole('radio', { name: /^Any day next week/ }));
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Move it to next week' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'move_planned_meal',
			input: { planned_meal_id: 'pm_0000000000000001', week: addDays(monday, 7) },
		});
		const said = await screen.findByRole('status');
		expect(said).toHaveTextContent('Moved: Katsu Curry, any day next week.');
		await fireEvent.click(within(said).getByRole('button', { name: 'See next week' }));
		expect(await screen.findByText('Next week')).toBeInTheDocument();
	});

	it('plans a recipe again at its amount and leaves the first where it is', async () => {
		const { kamosu } = renderScreen(Page, {
			get_meal_plan: weekOf([meal()]),
			plan_recipe: { planned_meal: meal({ planned_meal_id: 'pm_00000000000000c1' }) },
		});
		await screen.findByRole('link', { name: /Katsu Curry/ });

		await fireEvent.click(screen.getByRole('button', { name: 'Change' }));
		await fireEvent.click(
			within(await screen.findByRole('dialog')).getByRole('button', { name: 'Plan again' }),
		);
		const sheet = await screen.findByRole('dialog');
		expect(
			within(sheet).getByText(/^Stays on .* too, and keeps 6 servings\.$/),
		).toBeInTheDocument();
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Show 3 more weeks' }));
		await fireEvent.click(within(sheet).getAllByRole('radio', { name: /^Any day that week/ })[0]);

		expect(kamosu.calls).toContainEqual({
			operation: 'get_meal_plan',
			input: { from: today(), to: addDays(monday, 41) },
		});
		await fireEvent.click(within(sheet).getByRole('button', { name: /^Plan it again for / }));
		expect(kamosu.calls).toContainEqual({
			operation: 'plan_recipe',
			input: {
				branch_id: 'b_katsu',
				week: addDays(monday, 14),
				planned_yield: { amount: '6', noun: 'servings' },
			},
		});
		expect(kamosu.calls.some((call) => call.operation === 'move_planned_meal')).toBe(false);
		expect(await screen.findByRole('status')).toHaveTextContent(/^Planned again: Katsu Curry, /);
	});

	it('plans written words again with the same words', async () => {
		const { kamosu } = renderScreen(Page, {
			get_meal_plan: weekOf([words()]),
			plan_words: ({ words: typed }) => ({ planned_meal: words({ words: typed }) }),
		});
		await screen.findByText('Leftovers');

		await fireEvent.click(screen.getByRole('button', { name: 'Change' }));
		await fireEvent.click(
			within(await screen.findByRole('dialog')).getByRole('button', { name: 'Plan again' }),
		);
		const sheet = await screen.findByRole('dialog');
		await fireEvent.click(within(sheet).getByRole('radio', { name: /^Any day next week/ }));
		await fireEvent.click(
			within(sheet).getByRole('button', { name: 'Plan it again for next week' }),
		);
		expect(kamosu.calls).toContainEqual({
			operation: 'plan_words',
			input: { words: 'Leftovers', week: addDays(monday, 7) },
		});
	});

	it('puts a meal earlier or later in its day, and says which of how many it is', async () => {
		const { kamosu } = renderScreen(Page, {
			get_meal_plan: weekOf([
				meal(),
				meal({ planned_meal_id: 'pm_0000000000000002', position: 1, title: 'Ceviche' }),
			]),
			move_planned_meal: answered,
		});
		await screen.findByRole('link', { name: /Ceviche/ });

		await fireEvent.click(screen.getAllByRole('button', { name: 'Change' })[1]);
		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).getByText('2 of 2 on this day')).toBeInTheDocument();
		expect(within(sheet).getByRole('button', { name: /Later/ })).toBeDisabled();
		await fireEvent.click(within(sheet).getByRole('button', { name: /Earlier/ }));

		expect(kamosu.calls).toContainEqual({
			operation: 'move_planned_meal',
			input: { planned_meal_id: 'pm_0000000000000002', day: addDays(monday, 3), position: 0 },
		});
	});

	it('offers no Earlier and Later for the only meal on its day', async () => {
		renderScreen(Page, { get_meal_plan: weekOf([meal()]) });
		await screen.findByRole('link', { name: /Katsu Curry/ });
		await fireEvent.click(screen.getByRole('button', { name: 'Change' }));
		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).queryByRole('button', { name: /Earlier/ })).toBeNull();
	});

	it('moves a recipe that can no longer be read, and never plans it again', async () => {
		renderScreen(Page, {
			get_meal_plan: weekOf([anyDay({ title: 'Shakshuka', gone: true, written_yield: null })]),
		});
		await screen.findByText('Shakshuka');
		await fireEvent.click(screen.getByRole('button', { name: 'Remove' }));
		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).getByRole('button', { name: 'Move' })).toBeInTheDocument();
		expect(within(sheet).queryByRole('button', { name: 'Plan again' })).toBeNull();
		expect(within(sheet).getByText(/^Any day · This week$/)).toBeInTheDocument();
	});

	it('draws the week with no day as a band on the wide layout, and a meal dragged onto a day moves there', async () => {
		const { kamosu } = renderScreen(
			Page,
			{ get_meal_plan: weekOf([anyDay(), meal()]), move_planned_meal: answered },
			undefined,
			'wide',
		);

		const korean = await screen.findByRole('link', { name: /Korean Fried Chicken/ });
		const band = screen.getByRole('region', { name: 'Any day this week' });
		expect(band).toContainElement(korean);
		expect(screen.getByText(/^Drag a meal to another day/)).toBeInTheDocument();

		const card = korean.closest('li') as HTMLElement;
		const saturday = screen.getByRole('group', { name: aPlannedDay(addDays(monday, 5)) });
		await fireEvent.dragStart(card);
		await fireEvent.dragOver(saturday);
		// An empty day keeps its box under the pointer, lit, rather than
		// shrinking to a line and losing the drop (found live).
		const nothing = within(saturday).getByText('Nothing');
		expect(nothing).toHaveClass('border-accent');
		await fireEvent.drop(saturday);

		expect(kamosu.calls).toContainEqual({
			operation: 'move_planned_meal',
			input: { planned_meal_id: 'pm_00000000000000b1', day: addDays(monday, 5), position: 0 },
		});
	});
});

describe('the Plan when something goes wrong or nothing is there (#487)', () => {
	it('keeps a + on every day of an empty week, as its sentence says', async () => {
		renderScreen(Page, { get_meal_plan: { planned_meals: [] } });

		expect(await screen.findByText(/or press \+ on a day to write something/)).toBeInTheDocument();
		expect(screen.getAllByRole('button', { name: /^Add to / })).toHaveLength(8);
	});

	it('says so when a move is refused, and keeps the sheet open', async () => {
		renderScreen(Page, {
			get_meal_plan: weekOf([meal()]),
			move_planned_meal: { refuse: 'not_found' as const },
		});
		await screen.findByRole('link', { name: /Katsu Curry/ });
		await fireEvent.click(screen.getByRole('button', { name: 'Change' }));
		await fireEvent.click(
			within(await screen.findByRole('dialog')).getByRole('button', { name: 'Move' }),
		);
		const sheet = await screen.findByRole('dialog');
		await fireEvent.click(within(sheet).getByRole('radio', { name: /^Any day next week/ }));
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Move it to next week' }));

		expect(await within(sheet).findByRole('alert')).toHaveTextContent("That didn't save");
	});

	it('says so in the sheet when Earlier is refused', async () => {
		renderScreen(Page, {
			get_meal_plan: weekOf([
				meal(),
				meal({ planned_meal_id: 'pm_0000000000000002', position: 1, title: 'Ceviche' }),
			]),
			move_planned_meal: { refuse: 'not_found' as const },
		});
		await screen.findByRole('link', { name: /Ceviche/ });
		await fireEvent.click(screen.getAllByRole('button', { name: 'Change' })[1]);
		const sheet = await screen.findByRole('dialog');
		await fireEvent.click(within(sheet).getByRole('button', { name: /Earlier/ }));

		expect(await within(sheet).findByRole('alert')).toHaveTextContent("That didn't save");
	});

	it('says so when a move by dragging is refused', async () => {
		renderScreen(
			Page,
			{
				get_meal_plan: weekOf([meal()]),
				move_planned_meal: { refuse: 'not_found' as const },
			},
			undefined,
			'wide',
		);
		const katsu = await screen.findByRole('link', { name: /Katsu Curry/ });
		const saturday = screen.getByRole('group', { name: aPlannedDay(addDays(monday, 5)) });
		await fireEvent.dragStart(katsu.closest('li') as HTMLElement);
		await fireEvent.dragOver(saturday);
		await fireEvent.drop(saturday);

		expect(await screen.findByRole('alert')).toHaveTextContent("That didn't save");

		// Gone with the week it was said on.
		await fireEvent.click(screen.getByRole('button', { name: 'The week after' }));
		expect(screen.queryByRole('alert')).toBeNull();
	});

	it('drops a meal onto an empty week with no day, and the lit box goes when the pointer leaves', async () => {
		const { kamosu } = renderScreen(
			Page,
			{
				get_meal_plan: weekOf([meal()]),
				move_planned_meal: ({ planned_meal_id }) => ({ planned_meal: meal({ planned_meal_id }) }),
			},
			undefined,
			'wide',
		);
		const katsu = await screen.findByRole('link', { name: /Katsu Curry/ });
		const band = screen.getByRole('region', { name: 'Any day this week' });
		const empty = within(band).getByText(/^Nothing on .any day. for this week\.$/);

		await fireEvent.dragStart(katsu.closest('li') as HTMLElement);
		await fireEvent.dragOver(band);
		expect(empty).toHaveClass('text-accent');
		await fireEvent.dragLeave(band, { relatedTarget: document.body });
		expect(empty).not.toHaveClass('text-accent');
		await fireEvent.dragOver(band);
		await fireEvent.drop(band);

		expect(kamosu.calls).toContainEqual({
			operation: 'move_planned_meal',
			input: { planned_meal_id: 'pm_0000000000000001', week: monday, position: 0 },
		});
	});
});

describe('sending the plan to the Shopping List (#488)', () => {
	const katsu = meal();
	const ceviche = meal({
		planned_meal_id: 'pm_0000000000000002',
		position: 1,
		branch_id: 'b_ceviche',
		title: 'Ceviche',
		planned_yield: null,
		written_yield: { amount: '2', noun: 'servings' },
		sent_factor: 1,
		sent_at: '2026-10-24T09:00:00.000Z',
	});
	const leftovers = meal({
		planned_meal_id: 'pm_0000000000000003',
		day: addDays(monday, 4),
		branch_id: null,
		words: 'Leftovers',
		title: null,
		planned_yield: null,
		written_yield: null,
	});
	const list = { chosen: [], rows: [] };
	const answer = (over: Partial<SendToShoppingListOutput> = {}): SendToShoppingListOutput => ({
		outcome: 'sent',
		sent: [],
		already_sent: [],
		skipped: [],
		planned_meals: [],
		shopping_list: list,
		...over,
	});

	it('chooses a week in its sheet, says what each day would send, and says what went', async () => {
		const { kamosu } = renderScreen(Page, {
			get_meal_plan: weekOf([katsu, ceviche, leftovers]),
			send_to_shopping_list: answer({
				sent: [
					{
						branch_id: 'b_katsu',
						title: 'Katsu Curry',
						added_factor: 1.5,
						raised: false,
						planned_meals: [katsu.planned_meal_id],
						was_on_list: true,
						shopping_yield: { amount: '10', noun: 'servings' },
						written_yield: { amount: '4', noun: 'servings' },
					},
				],
				already_sent: [
					{
						planned_meal_id: ceviche.planned_meal_id,
						branch_id: 'b_ceviche',
						title: 'Ceviche',
						planned_fewer: false,
					},
				],
				skipped: [
					{
						planned_meal_id: leftovers.planned_meal_id,
						title: null,
						words: 'Leftovers',
						reason: 'words',
					},
				],
			}),
		});
		await screen.findByRole('link', { name: /Katsu Curry/ });

		await fireEvent.click(screen.getByRole('button', { name: 'Send to shopping' }));
		const sheet = await screen.findByRole('dialog');
		expect(await within(sheet).findByText(/What are you shopping for/)).toBeInTheDocument();
		const go = within(sheet).getByRole('button', { name: 'Choose what to send' });
		expect(go).toBeDisabled();
		// What each day would send: a recipe at its amount, one sent, and words that stay.
		const thursday = within(sheet).getByRole('button', {
			name: new RegExp(aPlannedDay(addDays(monday, 3))),
		});
		expect(thursday).toHaveTextContent('Katsu Curry 6 servings');
		expect(thursday).toHaveTextContent('Ceviche sent');
		expect(
			within(sheet).getByRole('button', { name: new RegExp(aPlannedDay(addDays(monday, 4))) }),
		).toHaveTextContent('Leftovers stays on the plan');

		await fireEvent.click(within(sheet).getByRole('button', { name: /^This week/ }));
		expect(thursday).toBeDisabled();
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Send 1 recipe to shopping' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'send_to_shopping_list',
			input: { days: [], weeks: [monday], any_day_of: [] },
		});
		const said = await screen.findByRole('status');
		expect(said).toHaveTextContent('Sent to your Shopping List:');
		expect(said).toHaveTextContent('Katsu Curry, 6 servings · now 10 servings on the list');
		expect(said).toHaveTextContent('Already sent before: Ceviche.');
		expect(said).toHaveTextContent('Not sent: Leftovers (written words).');
		expect(within(said).getByRole('link', { name: 'See the list' })).toHaveAttribute(
			'href',
			'/shopping',
		);
		expect(screen.queryByRole('dialog')).toBeNull();
	});

	it('sends a day, or a week with no day, on its own', async () => {
		const undated = meal({
			planned_meal_id: 'pm_0000000000000004',
			day: null,
			branch_id: 'b_chili',
			title: 'Chili',
		});
		const { kamosu } = renderScreen(Page, {
			get_meal_plan: weekOf([undated, katsu]),
			send_to_shopping_list: answer(),
		});
		await screen.findByRole('link', { name: /Chili/ });
		await fireEvent.click(screen.getByRole('button', { name: 'Send to shopping' }));
		const sheet = await screen.findByRole('dialog');
		await fireEvent.click(await within(sheet).findByRole('button', { name: /^Any day/ }));
		await fireEvent.click(
			within(sheet).getByRole('button', { name: new RegExp(aPlannedDay(addDays(monday, 3))) }),
		);
		await fireEvent.click(
			within(sheet).getByRole('button', { name: 'Send 2 recipes to shopping' }),
		);
		expect(kamosu.calls).toContainEqual({
			operation: 'send_to_shopping_list',
			input: { days: [addDays(monday, 3)], weeks: [], any_day_of: [monday] },
		});
		expect(await screen.findByRole('status')).toHaveTextContent(
			'Nothing to send: no recipe is planned on what you chose.',
		);
	});

	it('names a recipe once: not as sent before when it went now, nor when planned for fewer', async () => {
		renderScreen(Page, {
			get_meal_plan: weekOf([katsu]),
			send_to_shopping_list: answer({
				sent: [
					{
						branch_id: 'b_katsu',
						title: 'Katsu Curry',
						added_factor: 0.5,
						raised: true,
						planned_meals: [katsu.planned_meal_id],
						was_on_list: false,
						shopping_yield: { amount: '2', noun: 'servings' },
						written_yield: { amount: '4', noun: 'servings' },
					},
				],
				already_sent: [
					{
						planned_meal_id: 'pm_0000000000000009',
						branch_id: 'b_katsu',
						title: 'Katsu Curry',
						planned_fewer: false,
					},
					{
						planned_meal_id: 'pm_0000000000000008',
						branch_id: 'b_dandan',
						title: 'Dan Dan',
						planned_fewer: true,
					},
					{
						planned_meal_id: 'pm_0000000000000007',
						branch_id: 'b_ceviche',
						title: 'Ceviche',
						planned_fewer: false,
					},
				],
			}),
		});
		await screen.findByRole('link', { name: /Katsu Curry/ });
		await fireEvent.click(screen.getByRole('button', { name: 'Send to shopping' }));
		const sheet = await screen.findByRole('dialog');
		await fireEvent.click(await within(sheet).findByRole('button', { name: /^This week/ }));
		await fireEvent.click(within(sheet).getByRole('button', { name: /^Send/ }));
		const said = await screen.findByRole('status');
		expect(said).toHaveTextContent('Katsu Curry, 2 servings more');
		expect(said).toHaveTextContent('Already sent before: Ceviche.');
		expect(said).toHaveTextContent('Dan Dan: planned for fewer since it was sent');
	});

	it('says nothing is planned on what was chosen, rather than that it all went', async () => {
		renderScreen(Page, {
			get_meal_plan: weekOf([katsu]),
			send_to_shopping_list: answer(),
		});
		await screen.findByRole('link', { name: /Katsu Curry/ });
		await fireEvent.click(screen.getByRole('button', { name: 'Send to shopping' }));
		const sheet = await screen.findByRole('dialog');
		await fireEvent.click(await within(sheet).findByRole('button', { name: /^The week after/ }));
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Send to shopping' }));
		const said = await screen.findByRole('status');
		expect(said).toHaveTextContent('Nothing to send: no recipe is planned on what you chose.');
		expect(said).not.toHaveTextContent('went to your list');
	});

	it('never says words left behind went to the list', async () => {
		renderScreen(Page, {
			get_meal_plan: weekOf([leftovers]),
			send_to_shopping_list: answer({
				skipped: [
					{
						planned_meal_id: leftovers.planned_meal_id,
						title: null,
						words: 'Leftovers',
						reason: 'words',
					},
				],
			}),
		});
		await screen.findByText('Leftovers');
		await fireEvent.click(screen.getByRole('button', { name: 'Send to shopping' }));
		const sheet = await screen.findByRole('dialog');
		await fireEvent.click(await within(sheet).findByRole('button', { name: /^This week/ }));
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Send to shopping' }));
		const said = await screen.findByRole('status');
		expect(said).toHaveTextContent('Nothing new to send. Not sent: Leftovers (written words).');
		expect(said).not.toHaveTextContent('went to your list');
	});

	it('marks what went, what more is to send, and what was planned for fewer', async () => {
		renderScreen(Page, {
			get_meal_plan: weekOf([
				meal({ sent_factor: 1, sent_at: '2026-10-24T09:00:00.000Z' }),
				ceviche,
				meal({
					planned_meal_id: 'pm_0000000000000005',
					day: addDays(monday, 5),
					branch_id: 'b_pie',
					title: 'Chicken pie',
					planned_yield: { amount: '2', noun: '' },
					written_yield: { amount: '4 to 6', noun: 'servings' },
					sent_factor: 3,
					sent_at: '2026-10-24T09:00:00.000Z',
				}),
			]),
		});
		expect(await screen.findByRole('link', { name: /Katsu Curry/ })).toHaveTextContent(
			'Sent for 4 · 2 servings more to send',
		);
		expect(screen.getByRole('link', { name: /Ceviche/ })).toHaveTextContent('Sent to shopping');
		expect(screen.getByRole('link', { name: /Chicken pie/ })).toHaveTextContent('Sent for ×3');

		// The meal's own sheet says when, and what the next send would do.
		await fireEvent.click(screen.getAllByRole('button', { name: 'Change' })[0]);
		const sheet = await screen.findByRole('dialog');
		expect(sheet).toHaveTextContent(
			`Sent to your Shopping List for 4 servings on ${aPlannedDay('2026-10-24')}. Planned for more since: the next send adds 2 servings.`,
		);
	});

	it('says a send made offline waits, and what it did once it landed', async () => {
		renderScreen(Page, {
			get_meal_plan: weekOf([katsu]),
			send_to_shopping_list: answer({ outcome: 'waiting' }),
		});
		await screen.findByRole('link', { name: /Katsu Curry/ });
		await fireEvent.click(screen.getByRole('button', { name: 'Send to shopping' }));
		const sheet = await screen.findByRole('dialog');
		await fireEvent.click(await within(sheet).findByRole('button', { name: /^This week/ }));
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Send 1 recipe to shopping' }));
		expect(await screen.findByRole('status')).toHaveTextContent(
			'Waiting to send. This send waits on this phone',
		);
		expect(screen.getByRole('link', { name: /Katsu Curry/ })).not.toHaveTextContent('Sent');

		// The list changed on another device before it arrived.
		landedSend.answer = answer({ outcome: 'refused' });
		await vi.waitFor(() =>
			expect(screen.getByRole('status')).toHaveTextContent(
				'Nothing was sent. Your Shopping List changed on another device',
			),
		);
		expect(landedSend.answer).toBeNull();
	});
});
