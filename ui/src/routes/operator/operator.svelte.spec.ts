/**
 * The screen-seam test: the Operator's screen, against a stand-in Kamosu
 * (#103).
 *
 * The answers below are checked against the Catalogue before the screen sees
 * them, so this cannot quietly keep passing against a shape the Core no longer
 * serves. That is not hypothetical here: `list_backups` was declared to answer
 * a Backup's `tier` while the Core has always sent `slot`, and a test written
 * against the declaration passed while the real screen would have rendered a
 * blank. The declaration is now `slot`, and this file asserts the word.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { screen, fireEvent, within } from '@testing-library/svelte';
import Operator from './+page.svelte';
import { renderScreen } from '../../testing/render';
import type { Answers } from '$lib/api/stand-in';
import type { ListFoodsOutput, RereadIngredientLinesOutput } from '$lib/api/catalogue';

type Food = ListFoodsOutput['foods'][number];

const food = (over: Partial<Food> & Pick<Food, 'id'>): Food => ({
	name: null,
	language: null,
	names: [],
	cup_weight_grams: null,
	nutrition: null,
	reading_count: 0,
	...over,
});

const flour = food({ id: 'f_flour', name: 'flour', language: 'en', reading_count: 9 });
const farine = food({ id: 'f_farine', name: 'farine', language: 'fr', reading_count: 5 });

/**
 * An instance in the state most instances are in: two accounts, three Backups,
 * an empty worklist, and every Food in use.
 */
const quiet: Answers = {
	list_accounts: {
		accounts: [
			{
				name: 'Stéphane',
				is_operator: true,
				disabled: false,
				is_you: true,
				created_at: '2026-01-04T10:00:00Z',
			},
			{
				name: 'Camille',
				is_operator: false,
				disabled: false,
				is_you: false,
				created_at: '2026-09-21T10:00:00Z',
			},
		],
	},
	list_backups: {
		backups: [
			{
				name: 'kamosu-backup-daily-20260920T175418Z.zip',
				slot: 'daily',
				taken_at: '2026-09-20T17:54:18Z',
				size_bytes: 21_895_835,
			},
		],
	},
	list_merge_suggestions: { suggestions: [] },
	list_foods: { foods: [flour, farine] },
	get_public_address: { public_address: 'https://kamosu.example' },
	list_runner_keys: { runner_keys: [] },
	runner_status: { has_runner: false, last_asked_at: null },
};

/** The same instance, with one pair of Foods waiting to be looked at. */
const withSuggestion: Answers = {
	...quiet,
	list_merge_suggestions: {
		suggestions: [
			{
				foods: [flour, farine],
				reason: 'arrived_as_one',
				words: [
					{ language: 'en', name: 'flour' },
					{ language: 'fr', name: 'farine' },
				],
				created_at: '2026-09-18T15:20:44Z',
			},
		],
	},
};

/** Move to the Instance room, where People, Backups and the address live. */
async function openInstance() {
	await fireEvent.click(await screen.findByRole('tab', { name: 'Instance' }));
}

/**
 * Open one Person's actions and hand back their row.
 *
 * The four acts a Person can be subject to are their own row's business and
 * are not on the screen until it is tapped, so every test that reaches for one
 * comes through here.
 */
async function openPerson(name: string) {
	const row = (await screen.findByText(name)).closest('li') as HTMLElement;
	await fireEvent.click(within(row).getByRole('button', { expanded: false }));
	return row;
}

describe("the Operator's screen", () => {
	it('shows nothing at all to a Person who is not an Operator', async () => {
		const { kamosu } = renderScreen(Operator, {
			list_accounts: {
				refuse: 'unauthorized',
				message: 'requires a Credential naming an Operator',
			},
		});

		expect(await screen.findByText(/nothing here for you/i)).toBeInTheDocument();
		// Not merely hidden: the room is never asked about. A screen that listed
		// Backups and then declined to show them would still have asked.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('list_backups');
		expect(screen.queryByRole('tab', { name: 'Worklist' })).not.toBeInTheDocument();
	});

	it('names who holds an account, and which one is you', async () => {
		renderScreen(Operator, quiet);
		await openInstance();

		expect(await screen.findByText('Stéphane')).toBeInTheDocument();
		expect(screen.getByText('Camille')).toBeInTheDocument();
		expect(screen.getByText('You')).toBeInTheDocument();
	});

	it('says the Operator boundary out loud, and is no window onto anyone', async () => {
		renderScreen(Operator, quiet);
		await openInstance();

		expect(await screen.findByText(/Operator boundary is a courtesy/i)).toBeInTheDocument();

		// ADR 0007: no recipe, Attempt or Kitchen of anybody's is reachable here.
		// Asserted on what the screen actually rendered, with the screen-reader
		// text stripped first so a hidden word cannot pass for a visible one.
		const shown = document.body.cloneNode(true) as HTMLElement;
		for (const hidden of shown.querySelectorAll('.sr-only')) hidden.remove();
		// The Runner's own sentences say what a Runner does with a recipe
		// (#275). They name no recipe of anybody's, so they are not what this
		// guards; the rest of its section, its keys' names, still is.
		const runner = [...shown.querySelectorAll('h2')].find((h) => h.textContent === 'Runner');
		for (const said of runner?.closest('section')?.querySelectorAll('p') ?? []) said.remove();
		const words = (shown.textContent ?? '').toLowerCase();
		for (const forbidden of ['recipe', 'attempt', 'kitchen']) {
			expect(words).not.toContain(forbidden);
		}
	});

	it('shows the empty worklist, which is where most instances live', async () => {
		renderScreen(Operator, quiet);

		expect(await screen.findByText(/nothing to look at/i)).toBeInTheDocument();
		expect(screen.getByText(/never merges Foods by itself/i)).toBeInTheDocument();
		// Every Food here is in use, so none is offered for deleting.
		expect(screen.getByText(/is used by a recipe/i)).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument();
	});

	it('announces what a Merge will move before it moves it, and says that figure back', async () => {
		const { kamosu } = renderScreen(Operator, {
			...withSuggestion,
			preview_food_merge: {
				survivor: flour,
				absorbed: farine,
				ingredient_lines: 14,
				readings: 20,
				cup_weight_conflict: false,
			},
			merge_food: { food: flour, ingredient_lines: 14, readings: 20 },
		});

		expect(await screen.findByText(/arrived as one recipe/i)).toBeInTheDocument();
		await fireEvent.click(await screen.findByRole('button', { name: 'Keep flour' }));

		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).getByText('14')).toBeInTheDocument();
		expect(
			within(sheet).getByText(/Ingredient Lines move from farine to flour/),
		).toBeInTheDocument();
		expect(within(sheet).getByText(/no un-merge/i)).toBeInTheDocument();

		// Nothing has moved yet: the announcement is the whole point.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('merge_food');

		await fireEvent.click(within(sheet).getByRole('button', { name: 'Join them' }));
		const merge = kamosu.calls.find((call) => call.operation === 'merge_food');
		expect(merge?.input).toMatchObject({
			survivor_food_id: 'f_flour',
			absorbed_food_id: 'f_farine',
			// The figure the preview announced, said back — the safety net, not a
			// formality. Recomputing it here would defeat what it is for.
			ingredient_lines: 14,
		});
	});

	it('tells two Foods apart when they carry the same name', async () => {
		// Found in live acceptance: a Merge gives the survivor every name both
		// Foods held, so the next suggestion naming it read "liveflour ·
		// liveflour" over two identical Keep buttons, and neither said which
		// Food it kept. ADR 0022 makes doubt produce a new Food rather than a
		// merge, so same-named Foods are ordinary, not pathological.
		const english = food({ id: 'f_en', name: 'flour', language: 'en' });
		const french = food({ id: 'f_fr', name: 'flour', language: 'fr' });
		renderScreen(Operator, {
			...quiet,
			list_merge_suggestions: {
				suggestions: [
					{
						foods: [english, french],
						reason: 'name_typed_onto_another',
						words: [{ language: 'en', name: 'flour' }],
						created_at: '2026-09-18T15:20:44Z',
					},
				],
			},
		});

		expect(await screen.findByRole('button', { name: 'Keep flour (en)' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Keep flour (fr)' })).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Keep flour' })).not.toBeInTheDocument();
	});

	it('still tells them apart when even the language is the same', async () => {
		// The case the first fix missed, caught in live acceptance: both Foods
		// on the dev corpus were English, so naming the language printed the
		// identical label twice. The id always differs.
		renderScreen(Operator, {
			...quiet,
			list_merge_suggestions: {
				suggestions: [
					{
						foods: [
							food({ id: 'f_aaaaaaaa', name: 'flour', language: 'en' }),
							food({ id: 'f_bbbbbbbb', name: 'flour', language: 'en' }),
						],
						reason: 'name_typed_onto_another',
						words: [{ language: 'en', name: 'flour' }],
						created_at: '2026-09-18T15:20:44Z',
					},
				],
			},
		});

		const keeps = await screen.findAllByRole('button', { name: /^Keep flour/ });
		expect(keeps).toHaveLength(2);
		const labels = keeps.map((button) => button.textContent?.trim());
		expect(new Set(labels).size).toBe(2);
	});

	it('asks before deleting an account, and offers the reversible act beside it', async () => {
		const { kamosu } = renderScreen(Operator, { ...quiet, delete_account: { deleted: true } });
		await openInstance();

		const camille = await openPerson('Camille');
		await fireEvent.click(within(camille).getByRole('button', { name: 'Delete' }));

		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).getByText('Delete Camille?')).toBeInTheDocument();
		expect(within(sheet).getByText(/can't sign in any more/i)).toBeInTheDocument();
		expect(within(sheet).getByText(/cannot be undone/i)).toBeInTheDocument();
		expect(within(sheet).getByRole('button', { name: 'Disable' })).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('delete_account');

		await fireEvent.click(within(sheet).getByRole('button', { name: 'Delete the account' }));
		expect(kamosu.calls.find((call) => call.operation === 'delete_account')?.input).toMatchObject({
			name: 'Camille',
		});
	});

	it("shows the Core's refusal when the last Operator is the one being ended", async () => {
		const refusal =
			"'Stéphane' is the only Operator this instance has, and deleting the account would leave it with nobody able to administer it and no way to appoint anybody. Make somebody else an Operator first.";
		renderScreen(Operator, {
			...quiet,
			delete_account: { refuse: 'bad_request', message: refusal },
		});
		await openInstance();

		const you = await openPerson('Stéphane');
		await fireEvent.click(within(you).getByRole('button', { name: 'Delete' }));
		const sheet = await screen.findByRole('dialog');
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Delete the account' }));

		// In the words it arrived in: the reason is the useful half, and a
		// paraphrase here would keep only the fact.
		expect(await within(sheet).findByText(/only Operator this instance has/)).toBeInTheDocument();
		expect(within(sheet).getByText(/Make somebody else an Operator first/)).toBeInTheDocument();
	});

	it('shows an Invite once, and says it will not be shown again', async () => {
		renderScreen(Operator, { ...quiet, mint_invite: { link: '/invite/8f2c1a94e07b' } });
		await openInstance();

		await fireEvent.click(await screen.findByRole('button', { name: 'Invite someone' }));
		// Whole, with the address it is read from in front of it: what is on the
		// screen is what you send somebody, and `/invite/…` alone is a path.
		expect(
			await screen.findByText(`${window.location.origin}/invite/8f2c1a94e07b`),
		).toBeInTheDocument();
		expect(screen.getByText(/won't be shown again/i)).toBeInTheDocument();
	});

	it('keeps a Person\u2019s four acts in their own row, one row open at a time', async () => {
		renderScreen(Operator, quiet);
		await openInstance();

		// The list is a list of people first. Four buttons under every one of
		// them at once is what made this room a page of buttons with the names
		// lost among them.
		expect(await screen.findByText('Camille')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Recovery link' })).not.toBeInTheDocument();

		const camille = await openPerson('Camille');
		expect(within(camille).getByRole('button', { name: 'Recovery link' })).toBeInTheDocument();
		expect(within(camille).getByRole('button', { name: 'Disable' })).toBeInTheDocument();

		// Opening somebody else closes them, so the screen never grows two sets.
		await openPerson('St\u00e9phane');
		expect(
			within(camille).queryByRole('button', { name: 'Recovery link' }),
		).not.toBeInTheDocument();
	});

	it('hands over the whole Invite, not the path it was minted as', async () => {
		const written: string[] = [];
		Object.assign(navigator, {
			clipboard: {
				writeText: (text: string) => {
					written.push(text);
					return Promise.resolve();
				},
			},
		});
		renderScreen(Operator, { ...quiet, mint_invite: { link: '/invite/8f2c1a94e07b' } });
		await openInstance();

		await fireEvent.click(await screen.findByRole('button', { name: 'Invite someone' }));
		await fireEvent.click(await screen.findByRole('button', { name: 'Copy the link' }));

		// Forty hex characters selected by dragging on a phone is the failure
		// this button exists to prevent, so what it hands over is the thing you
		// can send somebody and not the Core's path.
		expect(written).toEqual([`${window.location.origin}/invite/8f2c1a94e07b`]);
		expect(await screen.findByRole('button', { name: 'Copied' })).toBeInTheDocument();
	});

	it('shows a recovery link under the Person it belongs to', async () => {
		renderScreen(Operator, {
			...quiet,
			mint_recovery_link: { link: '/recover/3b71d0ae5c92' },
		});
		await openInstance();

		const camille = await openPerson('Camille');
		await fireEvent.click(within(camille).getByRole('button', { name: 'Recovery link' }));

		// Shown once means shown where the eye already is: at the top of the
		// room it would be off-screen behind however many people came first.
		expect(
			await within(camille).findByText(`${window.location.origin}/recover/3b71d0ae5c92`),
		).toBeInTheDocument();
		expect(within(camille).getByText(/recovery link for Camille/i)).toBeInTheDocument();
	});

	it('raises a Person to Operator and stands one down, by name', async () => {
		const { kamosu } = renderScreen(Operator, {
			...quiet,
			set_operator: { name: 'Camille', is_operator: true },
		});
		await openInstance();

		const camille = await openPerson('Camille');
		await fireEvent.click(within(camille).getByRole('button', { name: 'Make an Operator' }));
		expect(kamosu.calls.find((call) => call.operation === 'set_operator')?.input).toEqual({
			name: 'Camille',
			is_operator: true,
		});

		// Somebody who already administers is offered the other direction.
		const you = await openPerson('Stéphane');
		expect(within(you).getByRole('button', { name: 'Stand down' })).toBeInTheDocument();
	});

	it('mints an Invite that carries whether they arrive as an Operator', async () => {
		const { kamosu } = renderScreen(Operator, {
			...quiet,
			mint_invite: { link: '/invite/8f2c1a94e07b' },
		});
		await openInstance();

		await fireEvent.click(
			await screen.findByRole('checkbox', { name: 'They arrive as an Operator' }),
		);
		await fireEvent.click(screen.getByRole('button', { name: 'Invite someone' }));
		expect(kamosu.calls.find((call) => call.operation === 'mint_invite')?.input).toEqual({
			is_operator: true,
		});
	});

	it('asks before sweeping, because a swept Photograph does not come back', async () => {
		const { kamosu } = renderScreen(Operator, {
			...quiet,
			sweep_photographs: {
				referenced: 65,
				newly_unreferenced: 0,
				back_in_use: 0,
				swept: 2,
				swept_photograph_ids: ['ph_1', 'ph_2'],
			},
		});

		await fireEvent.click(await screen.findByRole('button', { name: 'Sweep now' }));
		const sheet = await screen.findByRole('dialog');
		expect(within(sheet).getByText(/cannot be undone/i)).toBeInTheDocument();
		expect(within(sheet).getByText(/A picture a recipe shows is kept/i)).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('sweep_photographs');

		await fireEvent.click(within(sheet).getByRole('button', { name: 'Sweep now' }));
		expect(kamosu.calls.map((call) => call.operation)).toContain('sweep_photographs');
		expect(await screen.findByText(/Swept 2\. 65 are still in use\./)).toBeInTheDocument();
	});

	it('reads every line again and says what it changed (#166)', async () => {
		const line = (index: number): RereadIngredientLinesOutput['changed'][number] => ({
			branch_id: 'b_char_siu',
			title: 'Air Fryer Char Siu Chicken',
			line_index: index,
			line: `${index + 1} lb boneless, skinless chicken thighs`,
			before: { amount: `${index + 1}`, unit: 'lb', target: 'boneless' },
			after: { amount: `${index + 1}`, unit: 'lb', target: 'boneless, skinless chicken thighs' },
			on_head: true,
			older_versions: index === 0 ? 1 : 0,
		});
		const changed = [...Array(7).keys()].map(line);
		changed.push({
			...line(7),
			branch_id: 'b_dan_dan',
			title: 'Dan dan noodles',
			line: '1/2 lb ground pork or beef',
			before: { amount: '1/2', unit: 'lb', target: 'ground pork or beef' },
			after: { amount: '1/2', unit: 'lb', target: null },
		});
		changed.push({
			...line(8),
			branch_id: 'b_dan_dan',
			title: 'Dan dan noodles',
			line: '1/4 cup yacai or preserved mustard greens',
			before: { amount: '1/4', unit: 'cup', target: 'yacai or preserved mustard greens' },
			after: { amount: '1/4', unit: 'cup', target: null },
			on_head: false,
			older_versions: 2,
		});
		const { kamosu } = renderScreen(Operator, {
			...quiet,
			reread_ingredient_lines: { job_id: 'j_reread' },
			get_job: {
				id: 'j_reread',
				operation: 'reread_ingredient_lines',
				status: 'completed',
				progress: {},
				error: null,
				errorCode: null,
				created_at: '2026-09-27T20:00:00.000Z',
				updated_at: '2026-09-27T20:00:01.000Z',
				result: {
					changed,
					older_versions_changed: 3,
					emptied_foods: [{ food_id: 'f_boneless', name: 'boneless' }],
					kept_by_hand: 3,
				},
			},
		});

		expect(await screen.findByText(/keeps the ones you set by hand/)).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Read the unread lines' })).not.toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: 'Read every line again' }));

		expect(
			await screen.findByText(
				'Changed 8 lines in 2 recipes, and 3 in their older versions. Left alone 3 lines you set by hand.',
			),
		).toBeInTheDocument();
		expect(screen.getByText(/1 Food now has nothing pointing at it/)).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('reread_ingredient_lines');
		expect(
			kamosu.calls.filter((call) => call.operation === 'list_foods'),
			'the Foods it emptied join the list above',
		).toHaveLength(2);

		// Six changes at first, the rest behind "Show all".
		expect(screen.getAllByText('Air Fryer Char Siu Chicken')).toHaveLength(6);
		expect(screen.getByText('and 1 older version')).toBeInTheDocument();
		expect(screen.queryByText('Dan dan noodles')).not.toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: 'Show all 9 changes' }));
		const [danDan, danDanOlder] = screen
			.getAllByText('Dan dan noodles')
			.map((title) => title.closest('li')!);
		expect(danDanOlder.textContent, 'a change only in older versions is listed too').toContain(
			'Only in older versions: 2',
		);
		expect(danDan.textContent).toMatch(/1\/2 lb ·\s+ground pork or beef\s*→\s*1\/2 lb ·\s+no Food/);
	});

	it('says so when reading every line again changed nothing', async () => {
		renderScreen(Operator, {
			...quiet,
			reread_ingredient_lines: { job_id: 'j_reread' },
			get_job: {
				id: 'j_reread',
				operation: 'reread_ingredient_lines',
				status: 'completed',
				progress: {},
				error: null,
				errorCode: null,
				created_at: '2026-09-27T20:00:00.000Z',
				updated_at: '2026-09-27T20:00:01.000Z',
				result: { changed: [], older_versions_changed: 0, emptied_foods: [], kept_by_hand: 4 },
			},
		});

		await fireEvent.click(await screen.findByRole('button', { name: 'Read every line again' }));
		expect(
			await screen.findByText(/Nothing changed\. 4 lines you set by hand/),
		).toBeInTheDocument();
		expect(screen.queryByText(/nothing pointing at them/)).not.toBeInTheDocument();
	});

	it('does not mistake a broken Kamosu for a room that is not yours', async () => {
		renderScreen(Operator, {
			...quiet,
			list_accounts: { refuse: 'internal', message: 'the database is unreadable' },
		});

		// An Operator whose instance is failing is owed the reason, not an
		// empty room that says the powers were never theirs.
		expect(await screen.findByText('the database is unreadable')).toBeInTheDocument();
		expect(screen.queryByText(/nothing here for you/i)).not.toBeInTheDocument();
	});

	it('lists the Backups that exist, each by its slot', async () => {
		renderScreen(Operator, quiet);
		await openInstance();

		// `slot`, the word the Core has always sent. This assertion is what the
		// declaration's old `tier` would now fail on.
		expect(await screen.findByText(/daily · 21\.9 MB/)).toBeInTheDocument();
		expect(screen.getByRole('link', { name: 'Get' })).toHaveAttribute(
			'href',
			'/api/backups/kamosu-backup-daily-20260920T175418Z.zip',
		);
	});

	it('tells the truth about what changing the public address reaches', async () => {
		renderScreen(Operator, quiet);
		await openInstance();

		expect(await screen.findByText('https://kamosu.example')).toBeInTheDocument();
		const warning = screen.getByText(/Links already sent/);
		expect(warning).toHaveTextContent(/keep the old one/);
		expect(warning).toHaveTextContent(/can't be reissued/);
		// The false promise this ticket corrected must not come back.
		expect(warning.textContent ?? '').not.toMatch(/follow/i);
	});
});

/**
 * The Runner (#275), in the Instance room, as option 1 of 15 October 2026
 * drew it. It is 15 Oct 2026, 10:42 in every test: "Kitchen laptop" asked a
 * minute ago, "Old server" last asked 3 days ago.
 */
describe("the Operator's Runner", () => {
	const NOW = new Date('2026-10-15T10:42:00Z');
	const RECENT = '2026-10-15T10:41:00Z';
	const STALE = '2026-10-12T09:15:00Z';

	const key = (id: string, name: string, last_used_at: string | null, revoked = false) => ({
		id,
		name,
		created_at: '2026-10-01T08:00:00Z',
		last_used_at,
		revoked,
	});

	const laptop = key('rk_laptop', 'Kitchen laptop', RECENT);
	const server = key('rk_server', 'Old server', STALE);

	const runner = (keys: ReturnType<typeof key>[], last_asked_at: string | null): Answers => ({
		...quiet,
		list_runner_keys: { runner_keys: keys },
		runner_status: { has_runner: keys.some((k) => !k.revoked), last_asked_at },
	});

	const section = async () =>
		(await screen.findByRole('heading', { name: 'Runner' })).closest('section') as HTMLElement;

	beforeEach(() => {
		vi.useFakeTimers({ toFake: ['Date'] });
		vi.setSystemTime(NOW);
	});
	afterEach(() => vi.useRealTimers());

	it('says what a Runner is, and that none has asked when there is no key', async () => {
		renderScreen(Operator, runner([], null));
		await openInstance();
		const runnerSection = await section();

		expect(
			within(runnerSection).getByText(
				/A Runner is a small program you start on a computer beside Kamosu\. It hands what Kamosu can't read by itself/,
			),
		).toBeInTheDocument();
		expect(
			await within(runnerSection).findByText('No Runner has asked for work yet.'),
		).toBeInTheDocument();
		expect(within(runnerSection).getByText(/Make a Runner Key below/)).toBeInTheDocument();
		expect(within(runnerSection).getByText('No Runner Keys yet.')).toBeInTheDocument();
	});

	it('tells a key whose Runner never asked from no key at all', async () => {
		renderScreen(Operator, runner([key('rk_laptop', 'Kitchen laptop', null)], null));
		await openInstance();
		const runnerSection = await section();

		expect(await within(runnerSection).findByText('Never used')).toBeInTheDocument();
		expect(
			within(runnerSection).getByText('No Runner has asked for work yet.'),
		).toBeInTheDocument();
		expect(within(runnerSection).getByText(/Once one is started/)).toBeInTheDocument();
	});

	it('says how long ago a Runner asked, and each key its last use', async () => {
		renderScreen(Operator, runner([laptop, server], RECENT));
		await openInstance();
		const runnerSection = await section();

		expect(
			await within(runnerSection).findByText('A Runner asked for work 1 minute ago.'),
		).toBeInTheDocument();
		expect(within(runnerSection).getByText(/asks every few seconds/)).toBeInTheDocument();
		const rows = within(runnerSection).getAllByRole('listitem');
		expect(rows.map((row) => row.textContent)).toEqual([
			expect.stringMatching(/Kitchen laptop.*Last used 1 minute ago/),
			expect.stringMatching(/Old server.*Last used 3 days ago/),
		]);
	});

	it('says a Runner quiet for days may need looking at', async () => {
		renderScreen(Operator, runner([server], STALE));
		await openInstance();
		const runnerSection = await section();

		expect(
			await within(runnerSection).findByText('A Runner last asked for work 3 days ago.'),
		).toBeInTheDocument();
		expect(within(runnerSection).getByText(/check that it's still running/)).toBeInTheDocument();
	});

	it('stops describing a working Runner once every key is revoked', async () => {
		renderScreen(Operator, runner([key('rk_old', 'Gone', STALE, true)], RECENT));
		await openInstance();
		const runnerSection = await section();

		expect(
			await within(runnerSection).findByText('A Runner last asked for work 1 minute ago.'),
		).toBeInTheDocument();
		expect(within(runnerSection).getByText(/Make a Runner Key below/)).toBeInTheDocument();
		expect(within(runnerSection).queryByText(/asks every few seconds/)).not.toBeInTheDocument();
	});

	it('lists no revoked key', async () => {
		renderScreen(Operator, runner([laptop, key('rk_old', 'Gone', STALE, true)], RECENT));
		await openInstance();
		const runnerSection = await section();

		expect(await within(runnerSection).findByText('Kitchen laptop')).toBeInTheDocument();
		expect(within(runnerSection).queryByText('Gone')).not.toBeInTheDocument();
	});

	it('mints a named key and shows its Secret once, in a sheet', async () => {
		const { kamosu } = renderScreen(Operator, {
			...runner([], null),
			mint_runner_key: { id: 'rk_new', name: 'Kitchen laptop', secret: 'kr_SECRET' },
		});
		await openInstance();
		const runnerSection = await section();

		await fireEvent.input(within(runnerSection).getByLabelText(/A name, to tell this Runner/), {
			target: { value: 'Kitchen laptop' },
		});
		kamosu.answer('list_runner_keys', { runner_keys: [key('rk_new', 'Kitchen laptop', null)] });
		await fireEvent.click(within(runnerSection).getByRole('button', { name: 'Make a Runner Key' }));

		const sheet = await screen.findByRole('dialog', { name: 'Your Runner Key for Kitchen laptop' });
		expect(within(sheet).getByText('kr_SECRET')).toBeInTheDocument();
		expect(within(sheet).getByText(/won't show it again/)).toBeInTheDocument();
		expect(kamosu.calls.find((call) => call.operation === 'mint_runner_key')?.input).toEqual({
			name: 'Kitchen laptop',
		});

		await fireEvent.click(within(sheet).getByRole('button', { name: 'Done, I copied it' }));
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		// The list was asked again: the new key is there by name, its Secret is not.
		expect(await within(runnerSection).findByText('Never used')).toBeInTheDocument();
		expect(screen.queryByText('kr_SECRET')).not.toBeInTheDocument();
	});

	it('asks before revoking, and revokes that key alone', async () => {
		const { kamosu } = renderScreen(Operator, {
			...runner([laptop, server], RECENT),
			revoke_runner_key: { revoked: true },
		});
		await openInstance();
		const runnerSection = await section();

		const row = (await within(runnerSection).findByText('Kitchen laptop')).closest(
			'li',
		) as HTMLElement;
		await fireEvent.click(within(row).getByRole('button', { name: 'Revoke' }));
		const sheet = await screen.findByRole('dialog', { name: 'Revoke Kitchen laptop?' });
		expect(within(sheet).getByText(/cannot be undone/)).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('revoke_runner_key');

		kamosu.answer('list_runner_keys', { runner_keys: [{ ...laptop, revoked: true }, server] });
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Revoke' }));

		expect(kamosu.calls.find((call) => call.operation === 'revoke_runner_key')?.input).toEqual({
			runner_key_id: 'rk_laptop',
		});
		await vi.waitFor(() =>
			expect(within(runnerSection).queryByText('Kitchen laptop')).not.toBeInTheDocument(),
		);
		expect(within(runnerSection).getByText('Old server')).toBeInTheDocument();
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});
});
