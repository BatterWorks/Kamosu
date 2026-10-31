/**
 * The screen-seam test: the Plan, against a stand-in Kamosu (#486).
 *
 * The stand-in checks every answer against the shape the Catalogue declares,
 * so a test here can lie about the values and never about the shape. What it
 * is for is what a behaviour test cannot see: the week drawn as option B of
 * #486's prototype, a planned recipe opening at its planned Yield, one that
 * can no longer be read saying so, and an empty week saying how to plan.
 */

import { describe, expect, it } from 'vitest';
import { fireEvent, screen, within } from '@testing-library/svelte';
import Page from './+page.svelte';
import { renderScreen } from '../../testing/render';
import type { GetMealPlanInput, GetMealPlanOutput } from '$lib/api/catalogue';
import { addDays, mondayOf, today } from '$lib/plan-days';

type Meal = GetMealPlanOutput['planned_meals'][number];

const monday = mondayOf(today());

const meal = (over: Partial<Meal> = {}): Meal => ({
	planned_meal_id: 'pm_0000000000000001',
	day: addDays(monday, 3),
	position: 0,
	branch_id: 'b_katsu',
	title: 'Katsu Curry',
	gone: false,
	planned_yield: { amount: '6', noun: 'servings' },
	written_yield: { amount: '4', noun: 'servings' },
	main_photo: null,
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
		expect(document.querySelectorAll('section')).toHaveLength(7);
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
