/**
 * The screen-seam test: Home, against a stand-in Kamosu (#64).
 *
 * The stand-in checks every answer below against the shape the Catalogue
 * declares before the screen sees it, so a test here can lie about the values
 * and never about the shape — rename a field in `src/catalogue.rs` and this
 * fails in the same commit.
 *
 * What it is really for is the part a behaviour test cannot reach, because
 * none of it is an Operation's answer: that `/` is Home to a Person and the
 * login form to anybody else, that an omitted shelf leaves no heading behind,
 * that the shelf which says *under 30 minutes* takes the 30 from the Core
 * rather than from a phrase file, and that a brand-new instance is told what to
 * do rather than shown four empty rows.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { screen, within, fireEvent } from '@testing-library/svelte';
import Page from './+page.svelte';
import { renderScreen } from '../testing/render';
import { went } from '../testing/navigation';
import { writtenByMe } from '../testing/recipes';
import type {
	GetCookbookOutput,
	HomeShelvesOutput,
	ListKitchensOutput,
} from '#lib/api/catalogue.js';
import type { Answers } from '#lib/api/stand-in.js';
import { demo, INSTALL_KAMOSU, openWelcomeAgain } from '#lib/demo.svelte.js';
import { m } from '#lib/paraglide/messages.js';

type Shelf = HomeShelvesOutput['shelves'][number];

/** One card, with everything the Catalogue requires present. */
const card = (over: Record<string, unknown> = {}) => ({
	lineage_id: 'l_1',
	branch_id: 'b_1',
	title: 'Miso Soup',
	language: 'en',
	language_fallback: false,
	main_photo: null,
	yield: null,
	// Nothing was searched for on Home, so nothing matched — the same field the
	// library's shelf fills in, left empty here by the same Operation family.
	matched: null,
	...writtenByMe(),
	...over,
});

/** The reader's Cookbook with no join waiting on them: Home asks for it (#135). */
const ALONE: GetCookbookOutput = {
	id: 'c_1',
	name: null,
	authors: [{ person_id: 'p_1', name: 'Tom' }],
	recipe_count: 3,
	kitchens: [],
	invites: [],
	joins: [],
};

type Join = GetCookbookOutput['joins'][number];

/** Camille, who writes with Tom, said yes to Stéphane's Invite; Stéphane writes with Bob. */
const joinOf = (over: Partial<Join> = {}): Join => ({
	join_id: 'cj_1',
	state: 'waiting',
	accepted_by: { person_id: 'p_2', name: 'Camille' },
	invited_by: { person_id: 'p_3', name: 'Stéphane' },
	joining: {
		id: 'c_1',
		name: null,
		authors: [
			{ person_id: 'p_2', name: 'Camille' },
			{ person_id: 'p_1', name: 'Tom' },
		],
	},
	into: {
		id: 'c_2',
		name: null,
		authors: [
			{ person_id: 'p_3', name: 'Stéphane' },
			{ person_id: 'p_4', name: 'Bob' },
		],
	},
	together_recipes: 42,
	waiting_on: [
		{ person_id: 'p_1', name: 'Tom' },
		{ person_id: 'p_4', name: 'Bob' },
	],
	you: 'asked',
	refused_by: null,
	refused_by_co_author: false,
	...over,
});

/** The reader, Tom, as `get_person` answers for them. */
const TOM = { person_id: 'p_1', name: 'Tom', is_operator: false };

/** Tom's Kitchen, which he cooks in with Camille (#650). */
const WITH_CAMILLE: ListKitchensOutput = {
	kitchens: [
		{
			id: 'k_1',
			name: 'Dupont kitchen',
			nickname: null,
			members: [
				{ person_id: 'p_1', name: 'Tom' },
				{ person_id: 'p_2', name: 'Camille' },
			],
			cookbooks: [{ id: 'c_1', name: null, authors: [{ person_id: 'p_1', name: 'Tom' }] }],
			invites: [],
		},
	],
};

/** Home, answering who the reader is and the Kitchen they cook in unless a test says otherwise. */
const renderHome = (answers: Answers = {}) =>
	renderScreen(Page, { get_person: TOM, list_kitchens: WITH_CAMILLE, ...answers });

const home = (shelves: Shelf[], minutes = 30): HomeShelvesOutput => ({
	quick_tonight_minutes: minutes,
	shelves,
});

describe('a Cookbook join waiting on you (#135)', () => {
	const shelves = home([{ name: 'cooked_most', recipes: [card({ title: 'Katsu Curry' })] }]);

	it('is asked above the shelves, and a yes answers it by the Operation', async () => {
		let answered = false;
		const { kamosu } = renderHome({
			home_shelves: shelves,
			get_cookbook: () => (answered ? ALONE : { ...ALONE, joins: [joinOf()] }),
			answer_cookbook_join: () => {
				answered = true;
				return ALONE;
			},
		});

		expect(
			await screen.findByRole('heading', { name: 'Write one Cookbook with Stéphane and Bob?' }),
		).toBeInTheDocument();
		expect(screen.getByText('Stéphane invited Camille, who said yes.')).toBeInTheDocument();
		expect(
			screen.getByText(
				'42 recipes in one Cookbook, which all of you can change. Nothing changes until everyone says yes.',
			),
		).toBeInTheDocument();
		expect(screen.getByText('Katsu Curry')).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Write together' }));
		expect(kamosu.calls).toContainEqual({
			operation: 'answer_cookbook_join',
			input: { join_id: 'cj_1', yes: true },
		});
		await vi.waitFor(() =>
			expect(
				screen.queryByRole('heading', { name: 'Write one Cookbook with Stéphane and Bob?' }),
			).not.toBeInTheDocument(),
		);
	});

	it('answers no by the Operation too', async () => {
		const { kamosu } = renderHome({
			home_shelves: shelves,
			get_cookbook: { ...ALONE, joins: [joinOf()] },
			answer_cookbook_join: ALONE,
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'No' }));
		expect(kamosu.calls).toContainEqual({
			operation: 'answer_cookbook_join',
			input: { join_id: 'cj_1', yes: false },
		});
	});

	it('shows only the question among joins that also wait on others', async () => {
		renderHome({
			home_shelves: shelves,
			get_cookbook: {
				...ALONE,
				joins: [joinOf(), joinOf({ join_id: 'cj_2', you: 'invited' })],
			},
		});
		expect(
			await screen.findByRole('heading', { name: 'Write one Cookbook with Stéphane and Bob?' }),
		).toBeInTheDocument();
		expect(screen.queryByText(/^Waiting for/)).not.toBeInTheDocument();
	});

	it('says nothing on Home about a join that is not waiting on you', async () => {
		renderHome({
			home_shelves: shelves,
			get_cookbook: { ...ALONE, joins: [joinOf({ you: 'answered' })] },
		});
		expect(await screen.findByText('Katsu Curry')).toBeInTheDocument();
		await vi.waitFor(() =>
			expect(screen.queryByText(/Waiting for|Write one Cookbook/)).not.toBeInTheDocument(),
		);
	});
});

describe('Home', () => {
	it('gives each shelf an earlier and a later button that a pointer sees and a finger does not (#223)', async () => {
		const scrollBy = vi.fn();
		Element.prototype.scrollBy = scrollBy;
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'cooked_most', recipes: [card({ title: 'Katsu Curry' })] }]),
		});
		await screen.findByRole('heading', { name: 'Cooked most' });

		const later = screen.getByRole('button', { name: 'Later' });
		const earlier = screen.getByRole('button', { name: 'Earlier' });
		// Hidden until `(hover: hover)` holds, which only a pointer answers; jsdom
		// cannot say, so what is pinned is the gate, not the showing.
		expect(later.parentElement?.className).toContain('pointer-only');
		expect(earlier.parentElement?.className).toContain('pointer-only');
		expect(later.className).toContain('shelf-arrow');

		await fireEvent.click(later);
		expect(scrollBy).toHaveBeenCalledWith(expect.objectContaining({ behavior: 'smooth' }));
		expect(scrollBy.mock.calls[0]?.[0].left).toBeGreaterThan(0);
		await fireEvent.click(earlier);
		expect(scrollBy.mock.calls[1]?.[0].left).toBeLessThan(0);
	});

	it('shows each shelf the Core sent, under its own heading', async () => {
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([
				{ name: 'cooked_most', recipes: [card({ title: 'Katsu Curry' })] },
				{
					name: 'quick_tonight',
					recipes: [card({ lineage_id: 'l_2', branch_id: 'b_2', title: 'Omelette' })],
				},
			]),
		});

		expect(await screen.findByRole('heading', { name: 'Cooked most' })).toBeInTheDocument();
		expect(screen.getByRole('heading', { name: 'Quick tonight' })).toBeInTheDocument();
		expect(screen.getByText('Katsu Curry')).toBeInTheDocument();
		expect(screen.getByText('Omelette')).toBeInTheDocument();
	});

	it('leaves no heading behind for a shelf the Core did not send', async () => {
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'never_cooked', recipes: [card()] }]),
		});

		expect(await screen.findByRole('heading', { name: 'Never cooked' })).toBeInTheDocument();
		// An empty shelf is omitted rather than shown empty. The Core decides
		// that, so the screen has no case where a heading draws itself over
		// nothing — and this is what proves the absence is a real absence.
		expect(screen.queryByRole('heading', { name: 'Cooked most' })).not.toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Quick tonight' })).not.toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Recently opened' })).not.toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Recently added' })).not.toBeInTheDocument();
	});

	it('leads with what was recently added, newest first, in the order the Core sent', async () => {
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([
				{
					name: 'recently_added',
					recipes: [
						card({ lineage_id: 'l_2', branch_id: 'b_2', title: 'Tarte Tatin' }),
						card({ title: 'Miso Soup' }),
					],
				},
				{ name: 'never_cooked', recipes: [card({ title: 'Miso Soup' })] },
			]),
		});

		const heading = await screen.findByRole('heading', { name: 'Recently added' });
		// First on the screen, where #151 put it. The Core decides
		// the order, and the screen draws what it was sent without resorting.
		expect(screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent?.trim())).toEqual([
			'Recently added',
			'Never cooked',
		]);
		const section = heading.closest('section') as HTMLElement;
		expect(within(section).getByText('What just came in')).toBeInTheDocument();
		expect(
			within(section)
				.getAllByRole('listitem')
				.map((item) => item.textContent),
		).toEqual([expect.stringContaining('Tarte Tatin'), expect.stringContaining('Miso Soup')]);
	});

	it('takes the line quick tonight is drawn at from the Core, never from its own words', async () => {
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'quick_tonight', recipes: [card()] }], 20),
		});

		// The Core said twenty, so the screen says twenty. A number written into
		// the phrase file would go on saying thirty after the Core moved.
		expect(await screen.findByText('On the table inside 20 minutes')).toBeInTheDocument();
		expect(screen.queryByText('On the table inside 30 minutes')).not.toBeInTheDocument();
	});

	it('puts no count beside a shelf, and offers no filtered library that does not exist (#650)', async () => {
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([
				{
					name: 'cooked_most',
					recipes: [card(), card({ lineage_id: 'l_2', branch_id: 'b_2', title: 'Pain' })],
				},
			]),
		});

		const heading = await screen.findByRole('heading', { name: 'Cooked most' });
		const section = heading.closest('section') as HTMLElement;
		expect(within(section).getAllByRole('listitem')).toHaveLength(2);
		// The bare number read as a link to all of them (H3), and is gone.
		expect(within(section).queryByText('2')).not.toBeInTheDocument();

		// Every link on the shelf opens a recipe. Recipes filters by Kitchen and
		// by *mine*, never by *cooked most*, so a link there would promise this
		// shelf and deliver the whole library.
		const links = within(section).getAllByRole('link');
		expect(links.every((link) => link.getAttribute('href')?.startsWith('/recipes/'))).toBe(true);
	});

	it('draws a shelf heading as every section heading is drawn (#650)', async () => {
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'cooked_most', recipes: [card()] }]),
		});
		const heading = await screen.findByRole('heading', { name: 'Cooked most' });
		expect(heading.className).toContain('text-accent');
		expect(heading.closest('section')?.className).toContain('mt-8');
	});

	it('says no subtitle under its title (#650)', async () => {
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'cooked_most', recipes: [card()] }]),
		});
		await screen.findByRole('heading', { name: 'Home', level: 1 });
		expect(screen.queryByText(/Five ways in/)).not.toBeInTheDocument();
	});

	it('names who the reader cooks with and in which Kitchen, leading to that page (#650)', async () => {
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'cooked_most', recipes: [card()] }]),
		});
		const line = await screen.findByRole('link', {
			name: /You cook with Camille, in Dupont kitchen/,
		});
		expect(line).toHaveAttribute('href', '/settings/kitchens');
	});

	it('says the reader cooks alone where nobody else is in their Kitchen', async () => {
		const [kitchen] = WITH_CAMILLE.kitchens;
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'cooked_most', recipes: [card()] }]),
			list_kitchens: {
				kitchens: [{ ...kitchen!, nickname: 'Home', members: [kitchen!.members[0]!] }],
			},
		});
		expect(await screen.findByRole('link', { name: /Home: just you/ })).toBeInTheDocument();
	});

	it('names three people at most, then how many more', async () => {
		const [kitchen] = WITH_CAMILLE.kitchens;
		const members = ['Tom', 'Camille', 'Marc', 'Inès', 'Paul', 'Zoé'].map((name, at) => ({
			person_id: `p_${at + 1}`,
			name,
		}));
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'cooked_most', recipes: [card()] }]),
			list_kitchens: { kitchens: [{ ...kitchen!, members }] },
		});
		expect(
			await screen.findByRole('link', {
				name: /You cook with Camille, Marc, Inès and 2 more, in Dupont kitchen/,
			}),
		).toBeInTheDocument();
	});

	it('names every Kitchen the reader cooks in, where there are several', async () => {
		const [kitchen] = WITH_CAMILLE.kitchens;
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'cooked_most', recipes: [card()] }]),
			list_kitchens: {
				kitchens: [kitchen!, { ...kitchen!, id: 'k_2', name: 'Lyon', nickname: null }],
			},
		});
		expect(
			await screen.findByRole('link', { name: /You cook in Dupont kitchen and Lyon/ }),
		).toBeInTheDocument();
	});

	it('leaves the shelves standing without the line when the Kitchens cannot be read', async () => {
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'cooked_most', recipes: [card({ title: 'Katsu Curry' })] }]),
			list_kitchens: { refuse: 'busy' },
		});
		expect(await screen.findByText('Katsu Curry')).toBeInTheDocument();
		await vi.waitFor(() =>
			expect(screen.queryByRole('link', { name: /You cook/ })).not.toBeInTheDocument(),
		);
	});

	it('opens the add sheet from beside its title (#650)', async () => {
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'cooked_most', recipes: [card()] }]),
			runner_status: { has_runner: false, last_asked_at: null },
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'Add a recipe' }));
		const sheet = await screen.findByRole('dialog', { name: 'Add a recipe' });
		expect(within(sheet).getByRole('button', { name: /From a web page/ })).toBeInTheDocument();
		expect(within(sheet).getByRole('button', { name: 'Close' })).toBeInTheDocument();
	});

	it('takes the cook to Recipes when its search box is touched (#650)', async () => {
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([{ name: 'cooked_most', recipes: [card()] }]),
		});
		const box = await screen.findByRole('searchbox', { name: 'Name, ingredient, step…' });
		// Tabbed onto, it stays: only a tap or a key leaves Home.
		await fireEvent.focus(box);
		expect(went).not.toHaveBeenCalled();
		await fireEvent.click(box);
		expect(went).toHaveBeenCalledWith('/recipes');
	});

	it('tells a brand-new instance what to do instead of showing four empty rows', async () => {
		renderHome({
			get_cookbook: ALONE,
			home_shelves: home([]),
			runner_status: { has_runner: false, last_asked_at: null },
		});

		expect(await screen.findByRole('heading', { name: 'No recipes yet' })).toBeInTheDocument();
		// The one thing to do, which *does* it: the add sheet, not a link to a
		// second empty screen.
		// One Add a recipe, the main button: none stands beside the title too (#650).
		expect(screen.getByRole('button', { name: 'Add a recipe' })).toHaveClass('main-button');
		expect(screen.queryByRole('link', { name: 'Add a recipe' })).not.toBeInTheDocument();
		// No search box over nothing to search, and no heading for a shelf that is not there.
		expect(screen.queryByRole('searchbox')).not.toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Never cooked' })).not.toBeInTheDocument();
	});

	it('writes the recipe from the empty state rather than pointing at a screen to write it on', async () => {
		const { kamosu } = renderHome({
			get_cookbook: ALONE,
			home_shelves: home([]),
			runner_status: { has_runner: false, last_asked_at: null },
			create_recipe: {
				branch_id: 'b_new',
				lineage_id: 'l_new',
				cookbook: { id: 'c_1', name: null, authors: [{ person_id: 'p_1', name: 'Stéphane' }] },
				name: null,
				writes: true,
				mine: true,
				arrived: false,
				hand_id: 'h_1',
				language: 'en',
				origin_address: null,
				head_version_id: 'v_new',
				versions: [],
				translation: null,
				tags: [],
				related_recipes: [],
				cooked: { count: 0, last_cooked_at: null, ratings: [] },
				foods_created: [],
			},
		});

		await fireEvent.click(await screen.findByRole('button', { name: 'Add a recipe' }));
		const sheet = await screen.findByRole('dialog', { name: 'Add a recipe' });
		await fireEvent.click(within(sheet).getByRole('button', { name: /Write it yourself/ }));
		const field = await screen.findByLabelText('What is it called?');
		await fireEvent.input(field, { target: { value: 'Tarte Tatin' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Write a recipe' }));

		// It lands in the writer's own Cookbook, the only place a new recipe
		// can (#131), under the name that was typed: nothing is asked.
		await vi.waitFor(() => {
			const asked = kamosu.calls.find((call) => call.operation === 'create_recipe');
			expect(asked?.input).toEqual({ title: 'Tarte Tatin' });
		});
	});

	it('stands the login form at / for anybody who is not a Person yet', async () => {
		// Refused for want of a Credential is not a failure — it is the answer,
		// and the answer is the form. There is no cookie read anywhere here.
		renderHome({
			get_cookbook: ALONE,
			home_shelves: { refuse: 'unauthorized' },
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
		});

		expect(await screen.findByRole('heading', { name: 'Welcome back' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Log in' })).toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Home' })).not.toBeInTheDocument();
	});

	it('says so when Kamosu could not be reached, rather than asking you to log in again', async () => {
		// A refusal that is not about a Credential must never be answered with
		// the login form: telling a signed-in cook to sign in again is the worst
		// possible reading of a server that is simply down.
		renderHome({
			get_cookbook: ALONE,
			home_shelves: { refuse: 'internal', message: 'boom' },
		});

		expect(
			await screen.findByText('Kamosu could not work out your shelves just now.'),
		).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Log in' })).not.toBeInTheDocument();
	});
});

/**
 * A Demo's welcome card (#407, option B chosen on 23 October 2026), set into
 * the top of Home above its heading until Got it puts it away on this device.
 */
describe("a Demo's welcome card", () => {
	const shelves = () => ({
		get_cookbook: ALONE,
		home_shelves: home([{ name: 'cooked_most', recipes: [card({ title: 'Katsu Curry' })] }]),
	});
	const welcome = () => screen.queryByRole('region', { name: m.demo_welcome_title() });

	beforeEach(() => Object.assign(demo, { is: false, key: undefined, welcome: 'first' }));

	it('sits above the heading, saying what saves, when it cleans, and how an AI reads it', async () => {
		Object.assign(demo, { is: true, key: 'a-published-key' });
		renderHome(shelves());
		const heading = await screen.findByRole('heading', { level: 1 });

		const card = welcome()!;
		expect(card.compareDocumentPosition(heading) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
		expect(card).toHaveTextContent(m.demo_welcome_saves());
		expect(card).toHaveTextContent('04:00 UTC');
		expect(card).toHaveTextContent(`${location.origin}/mcp`);
		expect(card).toHaveTextContent('a-published-key');
		expect(within(card).getByRole('link', { name: m.demo_welcome_install() })).toHaveAttribute(
			'href',
			INSTALL_KAMOSU,
		);
	});

	it('is put away on this device by Got it', async () => {
		Object.assign(demo, { is: true, key: 'a-published-key' });
		renderHome(shelves());
		await screen.findByRole('heading', { level: 1 });

		await fireEvent.click(within(welcome()!).getByRole('button', { name: m.demo_welcome_close() }));

		expect(welcome()).toBeNull();
		expect(localStorage.getItem('kamosu.demo-welcome')).toBe('closed');
	});

	it('stands again at the top once the strip brought it back', async () => {
		Object.assign(demo, { is: true, key: 'a-published-key' });
		openWelcomeAgain();
		renderHome(shelves());
		await screen.findByRole('heading', { level: 1 });

		expect(welcome()).not.toBeNull();
	});

	it('copies the key', async () => {
		Object.assign(demo, { is: true, key: 'a-published-key' });
		const writeText = vi.fn().mockResolvedValue(undefined);
		Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
		renderHome(shelves());
		await screen.findByRole('heading', { level: 1 });

		await fireEvent.click(
			within(welcome()!).getByRole('button', {
				name: `${m.demo_copy()} ${m.demo_welcome_key()}`,
			}),
		);

		expect(writeText).toHaveBeenCalledWith('a-published-key');
		expect(
			await within(welcome()!).findByRole('button', {
				name: `${m.demo_copied()} ${m.demo_welcome_key()}`,
			}),
		).toBeInTheDocument();
	});

	it('is not there on an instance that is no Demo', async () => {
		renderHome(shelves());
		await screen.findByRole('heading', { level: 1 });

		expect(welcome()).toBeNull();
	});
});
