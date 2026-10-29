/**
 * The screen-seam test: Settings' menu and the pages everybody has (#469),
 * against a stand-in Kamosu. This instance's pages are tested in
 * `this-instance.svelte.spec.ts`.
 *
 * What makes this worth having is not that it runs — it is that the answers
 * below are checked against the Catalogue before the screen sees them. Rename
 * `setup_complete` in `src/catalogue.rs` and this test fails, loudly, in the
 * same commit; it cannot quietly keep passing against a shape the Core no
 * longer serves.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, screen, fireEvent, within, waitFor } from '@testing-library/svelte';
import Settings from './+page.svelte';
import SettingsMenu from './SettingsMenu.svelte';
import Name from './name/+page.svelte';
import SigningIn from './signing-in/+page.svelte';
import Keys from './keys/+page.svelte';
import Language from './language/+page.svelte';
import Device from './device/+page.svelte';
import Kitchens from './kitchens/+page.svelte';
import Cookbook from './cookbook/+page.svelte';
import Tags from './tags/+page.svelte';
import About from './about/+page.svelte';
import { renderScreen, type ScreenComponent } from '../../testing/render';
import type { Answers } from '$lib/api/stand-in';
import { went } from '../../testing/navigation';
import type { GetCookbookOutput, MeaningSearchStatusOutput } from '$lib/api/catalogue';
import { cookbookLabel, kitchenAnswer } from '../../testing/recipes';
import { holdTheInstallOffer } from '$lib/offline/device.svelte';
import { signingIn } from '$lib/shell/signing-in.svelte';
import { offerToInstall } from '../../testing/install';
import { demo } from '$lib/demo.svelte';

/**
 * Paraglide's `setLocale` reloads the page, which jsdom cannot do. What a test
 * wants to know is whether the interface was asked to change, and to what.
 */
const { setLocale } = vi.hoisted(() => ({ setLocale: vi.fn() }));
vi.mock('$lib/paraglide/runtime', async (original) => ({
	...(await original<typeof import('$lib/paraglide/runtime')>()),
	setLocale,
}));

/** An instance at an https:// name where nobody has added a passkey yet (#370). */
const NO_PASSKEYS = {
	public_address: 'https://kamosu.example',
	available: true,
	unavailable_because: null,
	origin: 'https://kamosu.example',
	passkeys: [],
};

/** Every screen visit asks for Sessions and Access Keys beside instance_status
 * (ADR 0031). Tests unconcerned with Access answer both as an anonymous
 * visitor would be refused — the same shape a stranger meets at either Door. */
const anonymous: Answers = {
	list_sessions: { refuse: 'unauthorized' },
	// Whether this Person administers the instance is learnt by asking
	// something only an Operator may ask (#103). A stranger is refused, and the
	// way into the Operator's screen is simply not there.
	list_accounts: { refuse: 'unauthorized' },
	list_passkeys: { refuse: 'unauthorized' },
	list_access_keys: { refuse: 'unauthorized' },
	get_reading_preferences: { refuse: 'unauthorized' },
	get_person: { refuse: 'unauthorized' },
	meaning_search_status: { refuse: 'unauthorized' },
};

/** Meaning Search's status, with everything the Catalogue requires present. */
const meaningStatus = (
	over: Partial<MeaningSearchStatusOutput> = {},
): MeaningSearchStatusOutput => ({
	state: 'unasked',
	on: false,
	offer: false,
	may_change: false,
	model: 'EmbeddingGemma-300M (4-bit)',
	terms_url: 'https://ai.google.dev/gemma/terms',
	prohibited_use_policy_url: 'https://ai.google.dev/gemma/prohibited_use_policy',
	terms_version: '2026-04-01',
	accepted_by: null,
	accepted_via_access_key: null,
	accepted_at: null,
	declined_at: null,
	model_present: false,
	indexed_at: null,
	recipes_not_yet_indexed: 0,
	...over,
});

/** Your Cookbook as `get_cookbook` answers it: yours alone, unless a test says otherwise. */
function cookbookOf(over: Partial<GetCookbookOutput> = {}): GetCookbookOutput {
	return {
		id: 'c_1',
		name: null,
		authors: [{ person_id: 'p_1', name: 'Stéphane' }],
		recipe_count: 87,
		kitchens: [],
		invites: [],
		joins: [],
		...over,
	};
}

/**
 * A signed-in Person who has never touched the setting: reading in English, in
 * American measures. That is the stated default rather than a guess about
 * anybody (#49, ADR 0016), and every signed-in screen visit asks for it.
 */
const readsInAmerican: Answers = {
	get_reading_preferences: { reading_language: 'en', reading_measures: 'us' },
	// Who is signed in, and what they are called (#113).
	get_person: { person_id: 'p_1', name: 'Stéphane' },
	// A signed-in Person who does not administer the instance (#103).
	list_accounts: { refuse: 'unauthorized' },
	// Meaning Search as most instances have it: nobody has been asked, so this
	// screen shows nothing about it at all. It is not where the feature is
	// discovered — that is inside *nothing found* (ADR 0029) — so its absence
	// here is the ordinary case rather than an omission.
	meaning_search_status: meaningStatus(),
	// No Crouton library brought in yet, so Settings links to no Report (#69).
	list_jobs: { jobs: [] },
	// A Cookbook of one, as every account starts with (#131), with no words
	// filed yet: its Tags card reads them whatever Kitchens there are.
	get_cookbook: cookbookOf(),
	list_tags: { tags: [] },
};

describe('the settings screen', () => {
	it('says which Kamosu this is, under About Kamosu (#469)', async () => {
		const { kamosu } = renderScreen(About, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			...anonymous,
		});

		expect(await screen.findByText('Kamosu 0.1.0')).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('instance_status');
		// What Kamosu does not protect left Settings with #469: it is in
		// SECURITY.md, where a project keeps such a list.
		expect(screen.queryByText(/does not protect/)).not.toBeInTheDocument();
	});

	// The story (#158), for showing somebody what Kamosu is.
	it('leads to the story of what Kamosu is', async () => {
		renderScreen(About, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			...anonymous,
		});

		expect(await screen.findByRole('link', { name: /What is Kamosu\?/ })).toHaveAttribute(
			'href',
			'/about',
		);
	});

	it('says so plainly when the instance cannot be reached', async () => {
		renderScreen(About, {
			instance_status: { refuse: 'internal', message: 'Kamosu could not be reached.' },
			...anonymous,
		});

		expect(await screen.findByText(/could not be reached/)).toBeInTheDocument();
	});

	it("shows a cook no group of the instance's (#469, ADR 0040)", async () => {
		renderScreen(Settings, { ...readsInAmerican });

		expect(await screen.findByRole('link', { name: /Stéphane/ })).toHaveTextContent('Cook');
		expect(screen.queryByRole('heading', { name: 'This instance' })).not.toBeInTheDocument();
		expect(screen.queryByRole('link', { name: /People/ })).not.toBeInTheDocument();
	});

	it("shows an Operator the instance's group, People first (#469)", async () => {
		renderScreen(Settings, {
			...readsInAmerican,
			list_accounts: {
				accounts: [
					{
						name: 'Stéphane',
						is_operator: true,
						disabled: false,
						is_you: true,
						created_at: '2026-01-04T10:00:00Z',
					},
				],
			},
		});

		expect(await screen.findByRole('link', { name: /People/ })).toHaveAttribute(
			'href',
			'/settings/people',
		);
		expect(screen.getByRole('heading', { name: 'This instance' })).toBeInTheDocument();
		expect(screen.getByRole('link', { name: /Stéphane/ })).toHaveTextContent('Operator');
	});

	it('offers every language Paraglide compiled, with the current one pressed', () => {
		renderScreen(Language, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			...anonymous,
		});

		for (const name of ['English', 'Français', 'Español']) {
			expect(screen.getByRole('button', { name })).toBeInTheDocument();
		}
		expect(screen.getByRole('button', { name: 'English' })).toHaveAttribute('aria-pressed', 'true');
	});

	it('shows nothing about signing in when the visitor is not signed in', async () => {
		const { kamosu } = renderScreen(SigningIn, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			...anonymous,
		});

		await vi.waitFor(() =>
			expect(kamosu.calls.map((call) => call.operation)).toContain('list_sessions'),
		);
		expect(screen.queryByRole('heading', { name: 'Password' })).not.toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Devices signed in' })).not.toBeInTheDocument();
	});

	it("marks a phone's Add-only key, and changes where what it shares goes (#452)", async () => {
		const phoneKey = {
			id: 'ak_phone',
			name: 'iPhone',
			read_only: false,
			operator: false,
			add_only: true,
			everything_to_agent: false,
			created_at: '2026-01-01T00:00:00Z',
			last_used_at: null,
			revoked: false,
		};
		const { kamosu } = renderScreen(Keys, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			list_accounts: { refuse: 'unauthorized' },
			list_sessions: { sessions: [] },
			...readsInAmerican,
			list_passkeys: NO_PASSKEYS,
			list_access_keys: { access_keys: [phoneKey] },
			set_key_agent_choice: { everything_to_agent: true },
			runner_status: { has_runner: true, last_asked_at: null },
		});

		const row = (await screen.findByText('iPhone')).closest('li')!;
		expect(row).toHaveTextContent('Add-only');
		const choice = within(row).getByRole('checkbox', {
			name: "Send everything it shares to Kamosu's AI agent, recipe sites too",
		});
		expect(choice).not.toBeChecked();
		await fireEvent.click(choice);
		await vi.waitFor(() =>
			expect(kamosu.calls.find((call) => call.operation === 'set_key_agent_choice')?.input).toEqual(
				{ access_key_id: 'ak_phone', everything_to_agent: true },
			),
		);
	});

	it('ends a device signed in and an Access Key, each from its own page (#469)', async () => {
		const answers: Answers = {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			list_accounts: { refuse: 'unauthorized' },
			list_sessions: {
				sessions: [
					{
						id: 's_1',
						name: 'iPhone',
						created_at: '2026-01-01T00:00:00Z',
						last_used_at: null,
						revoked: false,
						current: false,
					},
				],
			},
			...readsInAmerican,
			list_passkeys: NO_PASSKEYS,
			list_access_keys: {
				access_keys: [
					{
						id: 'ak_1',
						name: 'my agent',
						read_only: true,
						operator: false,
						add_only: false,
						everything_to_agent: false,
						created_at: '2026-01-01T00:00:00Z',
						last_used_at: null,
						revoked: false,
					},
				],
			},
			revoke_session: { revoked: true, current: false },
			revoke_access_key: { revoked: true },
		};

		const devices = renderScreen(SigningIn, answers);
		const sessionRow = (await screen.findByText('iPhone')).closest('li');
		expect(screen.queryByText('my agent')).not.toBeInTheDocument();
		await fireEvent.click(within(sessionRow as HTMLElement).getByRole('button', { name: 'End' }));
		expect(devices.kamosu.calls.map((call) => call.operation)).toContain('revoke_session');
		cleanup();

		const keys = renderScreen(Keys, answers);
		const keyRow = (await screen.findByText('my agent')).closest('li');
		expect(within(keyRow as HTMLElement).getByText('Read-only')).toBeInTheDocument();
		await fireEvent.click(within(keyRow as HTMLElement).getByRole('button', { name: 'End' }));
		expect(keys.kamosu.calls.map((call) => call.operation)).toContain('revoke_access_key');
	});

	it('offers Reading Measures on the account, defaulting to American', async () => {
		const { kamosu } = renderScreen(Language, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			list_sessions: { sessions: [] },
			list_passkeys: NO_PASSKEYS,
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_kitchens: { kitchens: [] },
			set_reading_preferences: { reading_language: 'en', reading_measures: 'metric' },
		});

		// American is the default, stated plainly rather than guessed at from
		// anything about the person (ADR 0016).
		const american = await screen.findByRole('button', { name: 'American' });
		expect(american).toHaveAttribute('aria-pressed', 'true');
		expect(screen.getByRole('button', { name: 'Metric' })).toHaveAttribute('aria-pressed', 'false');

		await fireEvent.click(screen.getByRole('button', { name: 'Metric' }));

		// It goes to the ACCOUNT, not to this browser — so the same cook reads
		// the same recipe the same way on the iPad on the worktop. The Reading
		// Language rides along unchanged.
		const sent = kamosu.calls.find((call) => call.operation === 'set_reading_preferences');
		expect(sent?.input).toEqual({ reading_language: 'en', reading_measures: 'metric' });
		expect(screen.getByRole('button', { name: 'Metric' })).toHaveAttribute('aria-pressed', 'true');
	});

	it("shows a minted Access Key's secret where the form was, once, then never again (#142)", async () => {
		const { kamosu } = renderScreen(Keys, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			list_sessions: { sessions: [] },
			list_passkeys: NO_PASSKEYS,
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			mint_access_key: {
				id: 'ak_new',
				name: 'a new agent',
				read_only: false,
				operator: false,
				add_only: false,
				everything_to_agent: false,
				secret: 'the-one-time-secret',
			},
			list_kitchens: { kitchens: [] },
		});

		await screen.findByText('No Access Keys yet.');
		await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'a new agent' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Make the key' }));

		const secret = await screen.findByText('the-one-time-secret');
		expect(kamosu.calls.map((call) => call.operation)).toContain('mint_access_key');

		// In the form's place, under the list, not at the top of the page: the
		// key is where the eye already is when Make the key is pressed.
		const heading = screen.getByRole('heading', { name: 'Make a key' });
		expect(heading.compareDocumentPosition(secret) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
		expect(screen.queryByLabelText('Name')).not.toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Make the key' })).not.toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Copy' })).toHaveFocus();

		await fireEvent.click(screen.getByRole('button', { name: 'Done, I copied it' }));
		expect(screen.queryByText('the-one-time-secret')).not.toBeInTheDocument();
		expect(screen.getByLabelText('Name')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Make the key' })).toBeInTheDocument();
	});

	it('offers an Operator, and nobody else, a key that acts as Operator, and asks first (#369, #469)', async () => {
		const { kamosu } = renderScreen(Keys, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			list_sessions: { sessions: [] },
			...readsInAmerican,
			list_accounts: {
				accounts: [
					{
						name: 'Stéphane',
						is_operator: true,
						disabled: false,
						is_you: true,
						created_at: '2026-01-04T10:00:00Z',
					},
				],
			},
			list_passkeys: NO_PASSKEYS,
			list_access_keys: {
				access_keys: [
					{
						id: 'ak_1',
						name: 'house agent',
						read_only: false,
						operator: true,
						add_only: false,
						everything_to_agent: false,
						created_at: '2026-01-01T00:00:00Z',
						last_used_at: null,
						revoked: false,
					},
				],
			},
			mint_access_key: {
				id: 'ak_new',
				name: 'a new agent',
				read_only: false,
				operator: true,
				add_only: false,
				everything_to_agent: false,
				secret: 'the-one-time-secret',
			},
			list_kitchens: { kitchens: [] },
		});

		expect(await screen.findByText('Acts as Operator')).toBeInTheDocument();
		const asCook = await screen.findByRole('radio', { name: /You, a cook/ });
		expect(asCook).toBeChecked();
		const warning = /House Notes every agent reads/;
		expect(screen.queryByText(warning)).not.toBeInTheDocument();

		// The warning is said where the key is made, not under the choice.
		await fireEvent.click(screen.getByRole('radio', { name: /You, and an Operator too/ }));
		expect(screen.queryByText(warning)).not.toBeInTheDocument();

		await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'a new agent' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Make the key' }));
		const sheet = await screen.findByRole('dialog', { name: 'Make an Operator key?' });
		expect(within(sheet).getByText(warning)).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('mint_access_key');

		await fireEvent.click(within(sheet).getByRole('button', { name: 'Make the key' }));
		await screen.findByText('the-one-time-secret');
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		const sent = kamosu.calls.find((call) => call.operation === 'mint_access_key');
		expect(sent?.input).toEqual({ name: 'a new agent', read_only: false, operator: true });
	});

	it('offers a Person who is no Operator no choice, and mints them an ordinary key (#369)', async () => {
		const { kamosu } = renderScreen(Keys, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			list_sessions: { sessions: [] },
			...readsInAmerican,
			list_accounts: { refuse: 'unauthorized' },
			list_passkeys: NO_PASSKEYS,
			list_access_keys: { access_keys: [] },
			mint_access_key: {
				id: 'ak_new',
				name: 'a new agent',
				read_only: false,
				operator: false,
				add_only: false,
				everything_to_agent: false,
				secret: 'the-one-time-secret',
			},
			list_kitchens: { kitchens: [] },
		});

		await screen.findByText('No Access Keys yet.');
		expect(screen.queryByRole('radio')).not.toBeInTheDocument();
		await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'a new agent' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Make the key' }));
		await screen.findByText('the-one-time-secret');
		const sent = kamosu.calls.find((call) => call.operation === 'mint_access_key');
		expect(sent?.input).toEqual({ name: 'a new agent', read_only: false, operator: false });
	});

	it('copies a minted Access Key, and still shows it when the browser will not copy (#142)', async () => {
		renderScreen(Keys, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			list_sessions: { sessions: [] },
			list_passkeys: NO_PASSKEYS,
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			mint_access_key: {
				id: 'ak_new',
				name: 'a new agent',
				read_only: false,
				operator: false,
				add_only: false,
				everything_to_agent: false,
				secret: 'the-one-time-secret',
			},
			list_kitchens: { kitchens: [] },
		});
		const writeText = vi.fn(async () => {});
		Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });

		await screen.findByText('No Access Keys yet.');
		await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'a new agent' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Make the key' }));
		await fireEvent.click(await screen.findByRole('button', { name: 'Copy' }));

		expect(writeText).toHaveBeenCalledWith('the-one-time-secret');
		expect(await screen.findByRole('button', { name: 'Copied' })).toBeInTheDocument();

		writeText.mockRejectedValueOnce(new Error('not allowed'));
		await fireEvent.click(screen.getByRole('button', { name: 'Copied' }));
		expect(await screen.findByRole('button', { name: 'Copy' })).toBeInTheDocument();
		expect(screen.getByText('the-one-time-secret')).toBeInTheDocument();
		Reflect.deleteProperty(navigator, 'clipboard');
	});

	it("lists a signed-in Person's Kitchens, whose recipes each shows, and lets a member be removed", async () => {
		const { kamosu } = renderScreen(Kitchens, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			list_sessions: { sessions: [] },
			list_passkeys: NO_PASSKEYS,
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_tags: { tags: [] },
			list_kitchens: {
				kitchens: [
					{
						id: 'k_home',
						name: 'Maison Batterman',
						cookbooks: [cookbookLabel()],
						nickname: null,
						members: [{ person_id: 'p_1', name: 'Stéphane' }],
					},
					{
						id: 'k_shared',
						name: 'Supper Club',
						cookbooks: [cookbookLabel(), cookbookLabel('c_2', ['Marie'])],
						nickname: 'Nos amis',
						members: [
							{ person_id: 'p_1', name: 'Stéphane' },
							{ person_id: 'p_2', name: 'Marie' },
						],
					},
				],
			},
			remove_kitchen_member: { removed: true },
		});

		expect(await screen.findByDisplayValue('Supper Club')).toBeInTheDocument();
		expect(screen.getByDisplayValue('Maison Batterman')).toBeInTheDocument();
		// No Home Kitchen any more: a recipe lives in a Cookbook (#131).
		expect(screen.queryByText('Home')).not.toBeInTheDocument();
		expect(screen.getByDisplayValue('Nos amis')).toBeInTheDocument();
		expect(
			screen.getByText('Recipes here: Stéphane’s Cookbook and Marie’s Cookbook'),
		).toBeInTheDocument();

		const marieRow = screen.getByText('Marie').closest('li');
		await fireEvent.click(within(marieRow as HTMLElement).getByRole('button', { name: 'Remove' }));
		expect(kamosu.calls).toContainEqual({
			operation: 'remove_kitchen_member',
			input: { kitchen_id: 'k_shared', person_id: 'p_2' },
		});
	});

	// #131, screen choice 4: leaving is its own act at the foot of the card,
	// asked first with what each side keeps. Your own row carries no button.
	describe('leaving a Kitchen', () => {
		const kitchen = (id: string, members: [string, string][]) =>
			kitchenAnswer(id, {
				name: id,
				members: members.map(([person_id, name]) => ({ person_id, name })),
			});
		const signedIn = (kitchens: ReturnType<typeof kitchen>[], over: Answers = {}): Answers => ({
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			list_sessions: { sessions: [] },
			list_passkeys: NO_PASSKEYS,
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_tags: { tags: [] },
			list_kitchens: { kitchens },
			remove_kitchen_member: { removed: true },
			preview_leaving_kitchen: { they_keep: 4, you_keep: 6 },
			...over,
		});
		const rowOf = (name: string) => screen.getByText(name).closest('li') as HTMLElement;
		const TWO = [
			kitchen('Family', [
				['p_1', 'Stéphane'],
				['p_2', 'Hélène'],
				['p_3', 'Luc'],
			]),
			kitchen('Supper Club', [
				['p_1', 'Stéphane'],
				['p_4', 'Marie'],
			]),
		];
		const cardOf = async (name: string) =>
			(await screen.findByDisplayValue(name)).closest('li') as HTMLElement;

		it("offers Remove on everyone else's row and nothing on your own", async () => {
			renderScreen(Kitchens, signedIn(TWO));
			const family = await cardOf('Family');
			const mine = within(within(family).getByText('Stéphane').closest('li') as HTMLElement);
			expect(mine.queryByRole('button')).not.toBeInTheDocument();
			expect(within(rowOf('Hélène')).getByRole('button', { name: 'Remove' })).toBeInTheDocument();
			expect(
				within(family).getByRole('button', { name: 'Leave this Kitchen' }),
			).toBeInTheDocument();
		});

		it('asks first, saying what each side keeps, as the Core counted it', async () => {
			const { kamosu } = renderScreen(Kitchens, signedIn(TWO));
			await fireEvent.click(
				within(await cardOf('Family')).getByRole('button', { name: 'Leave this Kitchen' }),
			);

			const sheet = within(await screen.findByRole('dialog'));
			expect(sheet.getByRole('heading', { name: 'Leave Family?' })).toBeInTheDocument();
			expect(
				sheet.getByText('Family stops seeing your recipes, and you stop seeing theirs.'),
			).toBeInTheDocument();
			expect(sheet.getByText('Hélène and Luc keep')).toBeInTheDocument();
			expect(
				sheet.getByText('their own copy of the 4 recipes of yours they cooked'),
			).toBeInTheDocument();
			expect(sheet.getByText('your own copy of the 6 of theirs you cooked')).toBeInTheDocument();
			expect(kamosu.calls).toContainEqual({
				operation: 'preview_leaving_kitchen',
				input: { kitchen_id: 'Family' },
			});
			// Asking is not leaving.
			expect(kamosu.calls.map((call) => call.operation)).not.toContain('remove_kitchen_member');

			await fireEvent.click(sheet.getByRole('button', { name: 'Leave Family' }));
			await waitFor(() =>
				expect(kamosu.calls).toContainEqual({
					operation: 'remove_kitchen_member',
					input: { kitchen_id: 'Family', person_id: 'p_1' },
				}),
			);
			await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
		});

		it('says one person keeps, not one person keep', async () => {
			renderScreen(Kitchens, signedIn(TWO));
			await fireEvent.click(
				within(await cardOf('Supper Club')).getByRole('button', { name: 'Leave this Kitchen' }),
			);
			expect(
				within(await screen.findByRole('dialog')).getByText('Marie keeps'),
			).toBeInTheDocument();
		});

		it('says one recipe and nothing in words, never "1 recipes" or "0 of theirs"', async () => {
			renderScreen(
				Kitchens,
				signedIn(TWO, { preview_leaving_kitchen: { they_keep: 1, you_keep: 0 } }),
			);
			await fireEvent.click(
				within(await cardOf('Family')).getByRole('button', { name: 'Leave this Kitchen' }),
			);
			const sheet = within(await screen.findByRole('dialog'));
			expect(
				sheet.getByText('their own copy of the one recipe of yours they cooked'),
			).toBeInTheDocument();
			expect(sheet.getByText('nothing: you cooked none of theirs')).toBeInTheDocument();
		});

		it('stays without leaving anything', async () => {
			const { kamosu } = renderScreen(Kitchens, signedIn(TWO));
			await fireEvent.click(
				within(await cardOf('Family')).getByRole('button', { name: 'Leave this Kitchen' }),
			);
			await fireEvent.click(
				within(await screen.findByRole('dialog')).getByRole('button', { name: 'Stay' }),
			);
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
			expect(kamosu.calls.map((call) => call.operation)).not.toContain('remove_kitchen_member');
		});
	});

	it('creates a Kitchen from the form', async () => {
		const { kamosu } = renderScreen(Kitchens, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			list_sessions: { sessions: [] },
			list_passkeys: NO_PASSKEYS,
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_kitchens: { kitchens: [] },
			create_kitchen: {
				id: 'k_new',
				name: 'Supper Club',
				cookbooks: [cookbookLabel()],
				nickname: null,
				members: [{ person_id: 'p_1', name: 'Stéphane' }],
			},
		});

		await screen.findByText('Create a Kitchen');
		await fireEvent.input(screen.getByLabelText('Kitchen name'), {
			target: { value: 'Supper Club' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Create Kitchen' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'create_kitchen',
			input: { name: 'Supper Club' },
		});
	});

	it('makes a Kitchen Invite a link to send, copies it, and cancels it (#469)', async () => {
		const { kamosu } = renderScreen(Kitchens, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			...readsInAmerican,
			list_kitchens: {
				kitchens: [
					{
						id: 'k_home',
						name: 'Maison Batterman',
						cookbooks: [cookbookLabel()],
						nickname: null,
						members: [{ person_id: 'p_1', name: 'Stéphane' }],
					},
				],
			},
			invite_to_kitchen: { invite_id: 'ki_1', secret: 'the-invite-secret' },
			cancel_kitchen_invite: { ended: true },
		});

		await screen.findByDisplayValue('Maison Batterman');
		// Nothing to paste any more: the link does the joining.
		expect(screen.queryByText('Join a Kitchen')).not.toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: 'Invite someone' }));

		const link = `${location.origin}/kitchen-invite/the-invite-secret`;
		expect(await screen.findByText(link)).toBeInTheDocument();
		expect(screen.getByText('Send this link to the person. It works once.')).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('invite_to_kitchen');

		const writeText = vi.fn(async () => {});
		Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
		await fireEvent.click(screen.getByRole('button', { name: 'Copy' }));
		expect(writeText).toHaveBeenCalledWith(link);
		expect(await screen.findByRole('button', { name: 'Copied' })).toBeInTheDocument();
		Reflect.deleteProperty(navigator, 'clipboard');

		await fireEvent.click(screen.getByRole('button', { name: 'Cancel it' }));
		await waitFor(() =>
			expect(kamosu.calls).toContainEqual({
				operation: 'cancel_kitchen_invite',
				input: { invite_id: 'ki_1' },
			}),
		);
		await waitFor(() => expect(screen.queryByText(link)).not.toBeInTheDocument());
		expect(screen.getByRole('button', { name: 'Invite someone' })).toBeInTheDocument();
	});

	it('carries the expanding pair, and names no Kitchen in its title, on six Kitchens (#102)', async () => {
		// The Kitchen half of #102's test criterion, asked where a Kitchen count
		// is real. The header itself cannot answer it — it takes no client, which
		// is the design (ADR 0027): a switcher is forbidden, and filtering by
		// Kitchen already lives on the Recipes shelf (#62). So what is provable
		// is the other end: with a Person in six Kitchens, the screen the card
		// opens onto still says *Settings* and still carries the one name that
		// makes the card grow into it.
		renderScreen(Settings, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			list_sessions: { sessions: [] },
			list_passkeys: NO_PASSKEYS,
			list_access_keys: { access_keys: [] },
			...readsInAmerican,
			list_tags: { tags: [] },
			list_kitchens: {
				// The dev instance's own six.
				kitchens: [
					'Maison Batterman',
					'Chez Marc',
					'Chez Élodie',
					'Le Chalet',
					'Chez Papi',
					'Chez Marie',
				].map((name, index) => ({
					id: `k_${index}`,
					name,
					cookbooks: [cookbookLabel()],
					nickname: null,
					members: [{ person_id: 'p_1', name: 'Stéphane' }],
				})),
			},
		});

		// The menu reads nothing (#221): the six Kitchens are listed on their own
		// page, and the title here is the one the gear grows into.
		const title = await screen.findByRole('heading', { level: 1, name: 'Settings' });
		expect(title.getAttribute('style')).toContain('view-transition-name: settings');
		expect(title.getAttribute('style')).toContain('view-transition-class: expanding');

		// Not "Settings — Chez Marc", and not a Kitchen's name in its place.
		expect(title).toHaveTextContent(/^Settings$/);
	});

	it('offers no Save from your phone in a Demo, which refuses imports (ADR 0055)', async () => {
		demo.is = true;
		try {
			renderScreen(Settings, { ...readsInAmerican });
			await screen.findByRole('heading', { name: 'Add recipes' });
			expect(screen.queryByRole('link', { name: /Save from your phone/ })).not.toBeInTheDocument();
			expect(screen.getByRole('link', { name: /Import from another app/ })).toBeInTheDocument();
		} finally {
			demo.is = false;
		}
	});

	it('opens the first page beside the menu on the wide layout, and marks it there (#469)', async () => {
		renderScreen(
			Settings,
			{
				...readsInAmerican,
				list_sessions: { sessions: [] },
				list_passkeys: NO_PASSKEYS,
				list_access_keys: { access_keys: [] },
			},
			undefined,
			'wide',
		);
		expect(
			await screen.findByRole('heading', { level: 1, name: 'Signing in' }),
		).toBeInTheDocument();
		cleanup();

		renderScreen(
			SettingsMenu as unknown as ScreenComponent,
			{ ...readsInAmerican },
			undefined,
			'wide',
		);
		// A column of its own beside the page, which the layout draws.
		const nav = await screen.findByRole('navigation', { name: 'Settings' });
		expect(within(nav).getByRole('heading', { name: 'Settings' })).toBeInTheDocument();
		expect(within(nav).getByRole('link', { name: /Signing in/ })).toHaveAttribute(
			'href',
			'/settings/signing-in',
		);
	});

	it('groups its pages by whose they are, each row saying what is inside (#469)', async () => {
		const { kamosu } = renderScreen(Settings, {
			get_person: { person_id: 'p_1', name: 'Stéphane' },
			list_accounts: {
				accounts: [
					{
						name: 'Stéphane',
						is_operator: true,
						disabled: false,
						is_you: true,
						created_at: '2026-01-04T10:00:00Z',
					},
				],
			},
		});

		await screen.findByRole('heading', { name: 'This instance' });
		expect(screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent?.trim())).toEqual([
			'You',
			'Your household',
			'Add recipes',
			'This instance',
		]);
		const rows = screen.getAllByRole('link');
		expect(rows.map((row) => row.getAttribute('href'))).toEqual([
			'/settings/name',
			'/settings/signing-in',
			'/settings/keys',
			'/settings/language',
			'/settings/device',
			'/settings/security',
			'/settings/kitchens',
			'/settings/cookbook',
			'/settings/tags',
			'/foods',
			'/settings/import',
			'/settings/share',
			'/imports',
			'/settings/agent',
			'/settings/people',
			'/settings/backups',
			'/settings/runner',
			'/settings/address',
			'/settings/meaning',
			'/settings/upkeep',
			'/settings/instance-security',
			'/settings/about',
		]);
		const row = (href: string) => rows.find((each) => each.getAttribute('href') === href)!;
		expect(row('/settings/signing-in')).toHaveTextContent('Password, passkeys, devices signed in');
		expect(row('/settings/kitchens')).toHaveTextContent('Who sees and cooks your recipes');
		expect(row('/settings/cookbook')).toHaveTextContent('Who can change your recipes');
		expect(row('/settings/import')).toHaveTextContent('Crouton');
		expect(row('/settings/share')).toHaveTextContent('Save from your phone');
		expect(row('/settings/share')).toHaveTextContent(
			'With the Share button in Safari, Instagram, Photos…',
		);
		// Foods is for every Person: all five of its Operations are
		// Permission::Person, so it is no Operator's power (#107).
		expect(row('/foods')).toHaveTextContent('Foods');
		// The menu reads who is signed in, and whether they administer the
		// instance, and nothing else: every page reads what it needs.
		expect(kamosu.calls.map((call) => call.operation).sort()).toEqual([
			'get_person',
			'list_accounts',
		]);
		expect(screen.getByRole('button', { name: 'Sign out' })).toBeInTheDocument();
	});

	it('keeps the "not right now" facts once their cards are put away (#76)', async () => {
		renderScreen(Device, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			...anonymous,
		});
		// The test browser is not a secure page, so this device can keep nothing.
		expect(
			await screen.findByRole('heading', { level: 2, name: 'Offline copy' }),
		).toBeInTheDocument();
		expect(screen.getByText(/Kamosu is on http:\/\//)).toBeInTheDocument();
	});

	describe('on a phone a browser may offer to install Kamosu on (#173)', () => {
		let stop: () => void;
		beforeEach(() => {
			vi.stubGlobal('isSecureContext', true);
			stop = holdTheInstallOffer();
		});
		afterEach(() => {
			stop();
			vi.unstubAllGlobals();
			localStorage.removeItem('kamosu.put-away');
		});
		const showSettings = () =>
			renderScreen(Device, {
				instance_status: {
					version: '0.1.0',
					setup_complete: true,
					password_minimum: 15,
					passkey_origin: null,
					demo: false,
				},
				...anonymous,
			});

		it("offers the browser's install dialog while one is held, and puts the card away once accepted", async () => {
			const { prompt } = offerToInstall('accepted');
			showSettings();
			await fireEvent.click(await screen.findByRole('button', { name: 'Install Kamosu' }));
			expect(prompt).toHaveBeenCalledOnce();
			// This tab is still not the app: the reason stays, with nothing left to do.
			await waitFor(() =>
				expect(screen.queryByRole('button', { name: 'Install Kamosu' })).not.toBeInTheDocument(),
			);
			expect(screen.queryByText('Open the browser menu')).not.toBeInTheDocument();
			expect(screen.getByText(/Kamosu opens like an app/)).toBeInTheDocument();
			// Accepted here, the Home card stays away too, as it does from the card.
			expect(JSON.parse(localStorage.getItem('kamosu.put-away')!)).toEqual({ install: true });
		});

		it('falls back to the written steps when the dialog is turned down', async () => {
			offerToInstall('dismissed');
			showSettings();
			await fireEvent.click(await screen.findByRole('button', { name: 'Install Kamosu' }));
			expect(await screen.findByText('Open the browser menu')).toBeInTheDocument();
			expect(screen.queryByRole('button', { name: 'Install Kamosu' })).not.toBeInTheDocument();
		});

		it('writes the steps out while no install is on offer', async () => {
			showSettings();
			expect(await screen.findByText('Open the browser menu')).toBeInTheDocument();
			expect(screen.queryByRole('button', { name: 'Install Kamosu' })).not.toBeInTheDocument();
		});
	});

	// --- Tags (#104) --------------------------------------------------------
	//
	// Renaming, merging and deleting are HERE and not on the recipe page, which
	// is the choice of 22 September 2026. These guard the two things that
	// make the section worth having: the count beside each word, so deleting is
	// an informed act, and that a tag known only in one Language can be given a
	// name in another.

	/** One signed-in Person with one Kitchen and a Cookbook of their own, the ordinary case. */
	const withKitchen: Answers = {
		instance_status: {
			version: '0.1.0',
			setup_complete: true,
			password_minimum: 15,
			passkey_origin: null,
			demo: false,
		},
		list_sessions: { sessions: [] },
		list_passkeys: NO_PASSKEYS,
		list_access_keys: { access_keys: [] },
		...readsInAmerican,
		list_kitchens: {
			kitchens: [
				{
					id: 'k_home',
					name: 'Maison Batterman',
					cookbooks: [cookbookLabel()],
					nickname: null,
					members: [{ person_id: 'p_1', name: 'Stéphane' }],
				},
			],
		},
	};

	const settingsTag = (id: string, name: string, recipes: number, language = 'en') => ({
		id,
		cookbook_id: 'c_1',
		name,
		language,
		names: [{ language, name }],
		recipes,
		// The Core decides this against the Reading Language (#104); this
		// Person reads in English, which `readsInAmerican` above says.
		language_fallback: language !== 'en',
	});

	it('says a Cookbook has no tags yet rather than showing an empty box', async () => {
		renderScreen(Tags, { ...withKitchen, list_tags: { tags: [] } });

		expect(await screen.findByRole('heading', { name: 'Tags' })).toBeInTheDocument();
		expect(
			await screen.findByText('No tags yet. Add one to a recipe and it appears here.'),
		).toBeInTheDocument();
		// And it says what a change to a tag reaches.
		expect(screen.getByText(/reaches every recipe with the tag/)).toBeInTheDocument();
	});

	it('renames a tag through the Operation, saying it reaches every recipe at once', async () => {
		const { kamosu } = renderScreen(Tags, {
			...withKitchen,
			list_tags: { tags: [settingsTag('t_dessert', 'dessert', 12)] },
			rename_tag: settingsTag('t_dessert', 'desserts', 9),
		});

		// The count is on the row: 12 recipes carry this word, and that is the
		// fact a person weighs before touching it.
		expect(await screen.findByDisplayValue('dessert')).toBeInTheDocument();
		expect(screen.getByText('12 recipes')).toBeInTheDocument();
		// The promise a rename makes, said once at the top of the page.
		expect(screen.getByText(/reaches every recipe with the tag/)).toBeInTheDocument();

		const field = screen.getByDisplayValue('dessert');
		await fireEvent.input(field, { target: { value: 'desserts' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Rename' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'rename_tag',
			input: { tag_id: 't_dessert', language: 'en', name: 'desserts' },
		});

		// And it says what it reached. "Renaming reaches every recipe carrying
		// it, immediately" is the whole reason a Tag is kept once per Kitchen,
		// and a promise nothing on screen stands behind is not the criterion
		// being met.
		//
		// The number is the one `rename_tag` ANSWERED with — 9 here, against a
		// row that was showing 12 — because a row read a minute ago is the
		// stale one, and this sentence is the wrong place to be approximately
		// right.
		expect(await screen.findByText('Renamed on 9 recipes, all at once.')).toBeInTheDocument();
	});

	it('names a tag in the Language the Person reads recipes in, not the interface locale', async () => {
		// Two separate settings (ADR 0006, ADR 0016). A French interface over an
		// English Reading Language must not name a new word `fr`.
		const { kamosu } = renderScreen(Tags, {
			...withKitchen,
			get_reading_preferences: { reading_language: 'es', reading_measures: 'metric' },
			list_tags: { tags: [settingsTag('t_mijote', 'mijoté', 7, 'fr')] },
			rename_tag: settingsTag('t_mijote', 'guisado', 7, 'es'),
		});

		const adding = await screen.findByPlaceholderText('Name it in Spanish');
		await fireEvent.input(adding, { target: { value: 'guisado' } });
		await fireEvent.click(within(adding.closest('form') as HTMLElement).getByRole('button'));

		expect(kamosu.calls).toContainEqual({
			operation: 'rename_tag',
			input: { tag_id: 't_mijote', language: 'es', name: 'guisado' },
		});
	});

	it('gives a tag known only in French an English name, without renaming the French one', async () => {
		// The fallback ADR 0006 built, finally fixable: nothing until now let
		// anybody add the name it fell back from.
		const { kamosu } = renderScreen(Tags, {
			...withKitchen,
			list_tags: { tags: [settingsTag('t_mijote', 'mijoté', 7, 'fr')] },
			rename_tag: settingsTag('t_mijote', 'slow-cooked', 7),
		});

		expect(await screen.findByDisplayValue('mijoté')).toBeInTheDocument();
		expect(screen.getByText('fr')).toBeInTheDocument();

		const adding = screen.getByPlaceholderText('Name it in English');
		await fireEvent.input(adding, { target: { value: 'slow-cooked' } });
		await fireEvent.click(within(adding.closest('form') as HTMLElement).getByRole('button'));

		// Named in the reader's Language, leaving the French name where it is: a
		// tag holds a name per Language rather than one tag per Language.
		expect(kamosu.calls).toContainEqual({
			operation: 'rename_tag',
			input: { tag_id: 't_mijote', language: 'en', name: 'slow-cooked' },
		});
	});

	it('asks before deleting a tag, leading with how many recipes lose it', async () => {
		const { kamosu } = renderScreen(Tags, {
			...withKitchen,
			list_tags: { tags: [settingsTag('t_try', 'to try', 18)] },
			delete_tag: { deleted: true },
		});

		await screen.findByDisplayValue('to try');
		await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));

		// The figure leads, because "18 recipes lose this tag" is something a
		// person can weigh and "are you sure?" is not (#103).
		expect(await screen.findByText('Delete the tag to try?')).toBeInTheDocument();
		expect(screen.getByText('18')).toBeInTheDocument();
		expect(screen.getByText('recipes lose this tag')).toBeInTheDocument();
		// And it says outright that the recipes themselves are untouched.
		expect(screen.getByText(/The recipes are untouched/)).toBeInTheDocument();

		// Not yet done: the sheet is the act, not the button behind it.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('delete_tag');

		const sheet = screen.getByRole('dialog');
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Delete' }));
		expect(kamosu.calls).toContainEqual({
			operation: 'delete_tag',
			input: { tag_id: 't_try' },
		});
	});

	it('says “1 recipe loses this tag”, never “1 recipes”', async () => {
		// The figure and the phrase sit side by side in the sheet, so the phrase
		// has to inflect with it.
		renderScreen(Tags, {
			...withKitchen,
			list_tags: { tags: [settingsTag('t_one', 'brunch', 1)] },
			delete_tag: { deleted: true },
		});

		await screen.findByDisplayValue('brunch');
		await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));

		expect(await screen.findByText('recipe loses this tag')).toBeInTheDocument();
		expect(screen.queryByText('recipes lose this tag')).not.toBeInTheDocument();
	});

	it('asks before merging two tags, saying which way round it goes', async () => {
		const { kamosu } = renderScreen(Tags, {
			...withKitchen,
			list_tags: {
				tags: [settingsTag('t_quick', 'quick', 9), settingsTag('t_weeknight', 'weeknight', 21)],
			},
			merge_tags: settingsTag('t_weeknight', 'weeknight', 30),
		});

		await screen.findByDisplayValue('quick');
		const row = screen.getByDisplayValue('quick').closest('li') as HTMLElement;
		await fireEvent.change(within(row).getByRole('combobox'), { target: { value: 't_weeknight' } });

		// Which way round matters and is easy to get backwards, so it is spelt
		// out in both directions before anything moves.
		expect(await screen.findByText('Merge quick into weeknight?')).toBeInTheDocument();
		expect(screen.getByText('recipes move over')).toBeInTheDocument();

		// The button that acts says what it does, without the picker's ellipsis.
		const sheet = screen.getByRole('dialog');
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Merge' }));
		expect(kamosu.calls).toContainEqual({
			operation: 'merge_tags',
			input: { keep_tag_id: 't_weeknight', merge_tag_id: 't_quick' },
		});
	});
});

describe('the Reading Language (#112)', () => {
	/**
	 * The choice of 23 September 2026 (option C): one Language control
	 * that recipes follow, which can split in two for somebody who reads
	 * Kamosu in one Language and their recipes in another.
	 */
	const signedIn = (reading_language: 'en' | 'fr' | 'es'): Answers => ({
		instance_status: {
			version: '0.1.0',
			setup_complete: true,
			password_minimum: 15,
			passkey_origin: null,
			demo: false,
		},
		list_sessions: { sessions: [] },
		list_passkeys: NO_PASSKEYS,
		list_access_keys: { access_keys: [] },
		list_kitchens: { kitchens: [] },
		...readsInAmerican,
		get_reading_preferences: { reading_language, reading_measures: 'us' },
	});

	const words = () => screen.getByRole('list', { name: 'Language' });
	const recipes = () => screen.getByRole('list', { name: 'Recipes in' });

	it('says recipes follow the one control, and moves both when it is changed', async () => {
		setLocale.mockClear();
		const { kamosu } = renderScreen(Language, {
			...signedIn('en'),
			set_reading_preferences: { reading_language: 'fr', reading_measures: 'us' },
		});

		expect(await screen.findByText(/Recipe titles follow it\./)).toBeInTheDocument();
		expect(screen.queryByRole('list', { name: 'Recipes in' })).not.toBeInTheDocument();

		// The account first, then the interface: `setLocale` reloads the page,
		// and a save still in flight when it does is a save that may not land.
		let savedFirst = false;
		setLocale.mockImplementationOnce(() => {
			savedFirst = kamosu.calls.some((call) => call.operation === 'set_reading_preferences');
		});

		await fireEvent.click(within(words()).getByRole('button', { name: 'Français' }));

		await vi.waitFor(() => expect(setLocale).toHaveBeenCalledWith('fr'));
		expect(savedFirst).toBe(true);
		expect(kamosu.calls).toContainEqual({
			operation: 'set_reading_preferences',
			input: { reading_language: 'fr', reading_measures: 'us' },
		});
		// The reload would take the line saying what moved with it, so it is
		// left for the next load of this screen.
		expect(sessionStorage.getItem('kamosu.reading-language-moved')).toBe('fr');
	});

	it('says what moved once the reload that carried both is over', async () => {
		sessionStorage.setItem('kamosu.reading-language-moved', 'fr');
		renderScreen(Language, signedIn('en'));

		expect(
			await screen.findByText("Titles now show in French where there's one. Others are marked."),
		).toBeInTheDocument();
		// Said once: the next visit is quiet.
		expect(sessionStorage.getItem('kamosu.reading-language-moved')).toBeNull();
	});

	it('splits the recipes off on request, and moves only them', async () => {
		setLocale.mockClear();
		const { kamosu } = renderScreen(Language, {
			...signedIn('en'),
			set_reading_preferences: { reading_language: 'es', reading_measures: 'us' },
		});

		await fireEvent.click(
			await screen.findByRole('button', { name: 'Read recipes in another language' }),
		);

		// Split, the recipes have a control of their own.
		expect(within(recipes()).getByRole('button', { name: 'English' })).toHaveAttribute(
			'aria-pressed',
			'true',
		);

		await fireEvent.click(within(recipes()).getByRole('button', { name: 'Español' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'set_reading_preferences',
			input: { reading_language: 'es', reading_measures: 'us' },
		});
		// It says what just changed on the shelf, in the reader's words.
		expect(
			await screen.findByText("Titles now show in Spanish where there's one. Others are marked."),
		).toBeInTheDocument();
		expect(within(recipes()).getByRole('button', { name: 'Español' })).toHaveAttribute(
			'aria-pressed',
			'true',
		);
		// Kamosu's own words did not move.
		expect(setLocale).not.toHaveBeenCalled();
	});

	it('opens split when the account reads in a Language this browser does not speak', async () => {
		// A new phone, or a split made on another device. The screen says what
		// is true rather than pretending the two are one.
		setLocale.mockClear();
		const { kamosu } = renderScreen(Language, signedIn('fr'));

		await vi.waitFor(() =>
			expect(within(recipes()).getByRole('button', { name: 'Français' })).toHaveAttribute(
				'aria-pressed',
				'true',
			),
		);
		expect(within(words()).getByRole('button', { name: 'English' })).toHaveAttribute(
			'aria-pressed',
			'true',
		);

		// Changing the words while split leaves the account alone.
		await fireEvent.click(within(words()).getByRole('button', { name: 'Español' }));
		expect(setLocale).toHaveBeenCalledWith('es');
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('set_reading_preferences');
	});

	it('joins them again, reading recipes in the interface Language', async () => {
		const { kamosu } = renderScreen(Language, {
			...signedIn('fr'),
			set_reading_preferences: { reading_language: 'en', reading_measures: 'us' },
		});

		await fireEvent.click(
			await screen.findByRole('button', { name: 'Recipes in the same language as Kamosu' }),
		);

		expect(kamosu.calls).toContainEqual({
			operation: 'set_reading_preferences',
			input: { reading_language: 'en', reading_measures: 'us' },
		});
		expect(await screen.findByText(/Recipe titles follow it\./)).toBeInTheDocument();
		expect(screen.queryByRole('list', { name: 'Recipes in' })).not.toBeInTheDocument();
		expect(
			screen.getByText("Titles now show in English where there's one. Others are marked."),
		).toBeInTheDocument();
	});

	it('reads no Tags here: they live on their own page, which reads the Reading Language afresh (#221)', async () => {
		const { kamosu } = renderScreen(Language, {
			...signedIn('en'),
			list_kitchens: {
				kitchens: [
					{
						id: 'k_home',
						name: 'Maison Batterman',
						cookbooks: [cookbookLabel()],
						nickname: null,
						members: [{ person_id: 'p_1', name: 'Stéphane' }],
					},
				],
			},
			list_tags: { tags: [] },
			set_reading_preferences: { reading_language: 'fr', reading_measures: 'us' },
		});

		const tagReads = () => kamosu.calls.filter((call) => call.operation === 'list_tags').length;
		await fireEvent.click(
			await screen.findByRole('button', { name: 'Read recipes in another language' }),
		);
		await fireEvent.click(within(recipes()).getByRole('button', { name: 'Français' }));
		await vi.waitFor(() =>
			expect(kamosu.calls.some((call) => call.operation === 'set_reading_preferences')).toBe(true),
		);

		expect(tagReads()).toBe(0);
		expect(kamosu.calls.some((call) => call.operation === 'list_kitchens')).toBe(false);
	});

	it('puts the Reading Language back when the account refuses it', async () => {
		renderScreen(Language, {
			...signedIn('fr'),
			set_reading_preferences: { refuse: 'busy' },
		});

		const spanish = await vi.waitFor(() =>
			within(recipes()).getByRole('button', { name: 'Español' }),
		);
		await fireEvent.click(spanish);

		await vi.waitFor(() =>
			expect(within(recipes()).getByRole('button', { name: 'Français' })).toHaveAttribute(
				'aria-pressed',
				'true',
			),
		);
		expect(screen.queryByText(/Your shelf now shows/)).not.toBeInTheDocument();
	});

	it('promises a stranger nothing about recipes, since there is no account to follow', async () => {
		const { kamosu } = renderScreen(Language, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			...anonymous,
		});

		await screen.findByRole('heading', { name: 'Language' });
		await vi.waitFor(() =>
			expect(kamosu.calls.map((call) => call.operation)).toContain('get_reading_preferences'),
		);
		expect(screen.queryByText(/Recipe titles follow it/)).not.toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Measures' })).not.toBeInTheDocument();
	});
});

describe('your own name (#113)', () => {
	/** A Kitchen of your own and one you share with Marie, as the server names them now. */
	const kitchensNaming = (you: string) => ({
		kitchens: [
			{
				id: 'k_home',
				name: 'Maison Batterman',
				cookbooks: [cookbookLabel()],
				nickname: null,
				members: [{ person_id: 'p_1', name: you }],
			},
			{
				id: 'k_shared',
				name: 'Supper Club',
				cookbooks: [cookbookLabel()],
				nickname: null,
				members: [
					{ person_id: 'p_1', name: you },
					{ person_id: 'p_2', name: 'Marie' },
				],
			},
		],
	});

	const signedIn = (over: Answers = {}): Answers => ({
		instance_status: {
			version: '0.1.0',
			setup_complete: true,
			password_minimum: 15,
			passkey_origin: null,
			demo: false,
		},
		list_sessions: { sessions: [] },
		list_passkeys: NO_PASSKEYS,
		list_access_keys: { access_keys: [] },
		...readsInAmerican,
		list_tags: { tags: [] },
		list_kitchens: kitchensNaming('Stéphane'),
		...over,
	});

	it('opens the menu with your name, which leads to the page that changes it (#469)', async () => {
		renderScreen(Settings, signedIn());

		const card = await screen.findByRole('link', { name: /Stéphane/ });
		expect(card).toHaveAttribute('href', '/settings/name');
		// It is the first thing on the menu.
		expect(screen.getAllByRole('link')[0]).toBe(card);
	});

	it('marks your own row in every Kitchen as you, on their page (#221)', async () => {
		renderScreen(Kitchens, signedIn());

		await screen.findByDisplayValue('Supper Club');
		const marked = screen.getAllByText('you');
		expect(marked).toHaveLength(2);
		for (const mark of marked) expect(mark.closest('li')?.textContent).toContain('Stéphane');
		const marieRow = screen.getByText('Marie').closest('li') as HTMLElement;
		expect(within(marieRow).queryByText('you')).not.toBeInTheDocument();
	});

	it('says what renaming reaches before anything is renamed', async () => {
		const { kamosu } = renderScreen(Name, signedIn());

		expect(await screen.findByLabelText('Your name')).toHaveValue('Stéphane');
		expect(
			screen.getByText('On every Version you saved, and the name you sign in with.'),
		).toBeInTheDocument();
		// Nothing to change until the name is changed.
		expect(screen.getByRole('button', { name: 'Change my name' })).toBeDisabled();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('rename_person');
	});

	it('renames you through the Operation, and says what it did', async () => {
		const { kamosu } = renderScreen(Name, signedIn({ rename_person: { name: 'Stéphane Dupont' } }));

		await fireEvent.input(await screen.findByLabelText('Your name'), {
			target: { value: 'Stéphane Dupont' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Change my name' }));

		expect(
			await screen.findByText(
				'Done. Your Versions now say Stéphane Dupont. Sign in with that name from now on.',
			),
		).toBeInTheDocument();
		expect(kamosu.calls).toContainEqual({
			operation: 'rename_person',
			input: { name: 'Stéphane Dupont' },
		});
		expect(screen.getByLabelText('Your name')).toHaveValue('Stéphane Dupont');
	});

	it('says why a name was refused, and keeps the field to try another', async () => {
		renderScreen(
			Name,
			signedIn({
				rename_person: {
					refuse: 'bad_request',
					message: 'somebody here already signs in as Marie; choose another name',
				},
			}),
		);

		await fireEvent.input(await screen.findByLabelText('Your name'), {
			target: { value: 'Marie' },
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Change my name' }));

		expect(await screen.findByRole('alert')).toHaveTextContent(
			'somebody here already signs in as Marie; choose another name',
		);
		expect(screen.getByLabelText('Your name')).toHaveValue('Marie');
		expect(screen.queryByText(/^Done\./)).not.toBeInTheDocument();
	});

	it('shows a stranger no name to change', async () => {
		const { kamosu } = renderScreen(Name, {
			instance_status: {
				version: '0.1.0',
				setup_complete: true,
				password_minimum: 15,
				passkey_origin: null,
				demo: false,
			},
			...anonymous,
		});

		await vi.waitFor(() =>
			expect(kamosu.calls.map((call) => call.operation)).toContain('get_person'),
		);
		expect(screen.queryByLabelText('Your name')).not.toBeInTheDocument();
	});
});

describe('telling your Sessions apart (#114)', () => {
	/** You are on the iPhone; also signed in on a laptop, and on something
	 * from before Sessions were named for their device. */
	const session = (id: string, name: string, current = false) => ({
		id,
		name,
		created_at: '2026-09-12T09:00:00Z',
		last_used_at: '2026-09-20T18:28:00Z',
		revoked: false,
		current,
	});
	const three = [
		session('s_laptop', 'Firefox · Linux'),
		session('s_phone', 'Safari · iPhone', true),
		session('s_old', 'this browser'),
	];

	const signedIn = (over: Answers = {}): Answers => ({
		instance_status: {
			version: '0.1.0',
			setup_complete: true,
			password_minimum: 15,
			passkey_origin: null,
			demo: false,
		},
		list_sessions: { sessions: three },
		list_passkeys: NO_PASSKEYS,
		list_access_keys: { access_keys: [] },
		...readsInAmerican,
		list_tags: { tags: [] },
		list_kitchens: { kitchens: [] },
		...over,
	});

	const rows = async () => {
		const heading = await screen.findByRole('heading', { name: 'Devices signed in' });
		const list = heading.nextElementSibling as HTMLElement;
		return within(list).getAllByRole('listitem');
	};

	it('puts the device in your hand first and marks it, and names every other one', async () => {
		renderScreen(SigningIn, signedIn());

		const [first, second, third] = await rows();
		expect(within(first).getByText('Safari · iPhone')).toBeInTheDocument();
		expect(within(first).getByText('This device')).toBeInTheDocument();
		expect(within(first).getByRole('button', { name: 'Sign out' })).toBeInTheDocument();
		expect(within(first).queryByRole('button', { name: 'End' })).not.toBeInTheDocument();

		expect(within(second).getByText('Firefox · Linux')).toBeInTheDocument();
		expect(within(second).queryByText('This device')).not.toBeInTheDocument();
		expect(within(second).getByRole('button', { name: 'End' })).toBeInTheDocument();

		// A Session from before names says what it is, rather than "this browser",
		// which would be a lie on every device but one.
		expect(within(third).getByText('An older sign-in, not named')).toBeInTheDocument();
		expect(within(third).queryByText('this browser')).not.toBeInTheDocument();
		expect(within(third).getByText(/^Signed in .* · Last used /)).toBeInTheDocument();
		expect(screen.getAllByText('This device')).toHaveLength(1);
	});

	it('ends an older Session on its own, and stays here', async () => {
		let ended = false;
		const { kamosu } = renderScreen(
			SigningIn,
			signedIn({
				list_sessions: () => ({ sessions: ended ? three.slice(0, 2) : three }),
				revoke_session: () => {
					ended = true;
					return { revoked: true, current: false };
				},
			}),
		);

		const old = (await rows())[2];
		await fireEvent.click(within(old).getByRole('button', { name: 'End' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'revoke_session',
			input: { session_id: 's_old' },
		});
		await vi.waitFor(async () => expect(await rows()).toHaveLength(2));
		expect(went).not.toHaveBeenCalled();
	});

	it('signs you out when the Session ended is the one in your hand', async () => {
		const { kamosu } = renderScreen(
			SigningIn,
			signedIn({ revoke_session: { revoked: true, current: true } }),
		);

		await fireEvent.click(within((await rows())[0]).getByRole('button', { name: 'Sign out' }));
		// It asks first, as the foot of the Settings menu does (#380).
		await fireEvent.click(
			within(await screen.findByRole('dialog')).getByRole('button', { name: 'Sign out' }),
		);

		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/'));
		expect(kamosu.calls).toContainEqual({
			operation: 'revoke_session',
			input: { session_id: 's_phone' },
		});
		// So the sidebar is gone before Home is drawn, not a moment after (#194).
		expect(signingIn.signedIn).toBe(false);
	});

	it('renames an older Session in place, starting from an empty name', async () => {
		let renamed = false;
		const { kamosu } = renderScreen(
			SigningIn,
			signedIn({
				list_sessions: () => ({
					sessions: renamed ? [three[0], three[1], session('s_old', 'Kitchen laptop')] : three,
				}),
				rename_session: () => {
					renamed = true;
					return { id: 's_old', name: 'Kitchen laptop' };
				},
			}),
		);

		await fireEvent.click(within((await rows())[2]).getByRole('button', { name: 'Rename' }));
		const field = screen.getByLabelText('Call it');
		expect(field).toHaveValue('');
		await fireEvent.input(field, { target: { value: 'Kitchen laptop' } });
		await fireEvent.click(within((await rows())[2]).getByRole('button', { name: 'Save' }));

		expect(kamosu.calls).toContainEqual({
			operation: 'rename_session',
			input: { session_id: 's_old', name: 'Kitchen laptop' },
		});
		expect(await screen.findByText('Kitchen laptop')).toBeInTheDocument();
		expect(screen.queryByText('An older sign-in, not named')).not.toBeInTheDocument();
		expect(screen.queryByLabelText('Call it')).not.toBeInTheDocument();
	});

	it('starts renaming a named Session from its name, and Keep it changes nothing', async () => {
		const { kamosu } = renderScreen(SigningIn, signedIn());

		await fireEvent.click(within((await rows())[0]).getByRole('button', { name: 'Rename' }));
		expect(screen.getByLabelText('Call it')).toHaveValue('Safari · iPhone');
		await fireEvent.click(screen.getByRole('button', { name: 'Keep it' }));

		expect(screen.queryByLabelText('Call it')).not.toBeInTheDocument();
		expect(screen.getByText('Safari · iPhone')).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('rename_session');
	});

	it('says why a rename was refused, and keeps the field open', async () => {
		renderScreen(
			SigningIn,
			signedIn({
				rename_session: {
					refuse: 'not_found',
					message: 'no live Session with that id belongs to this Person',
				},
			}),
		);

		await fireEvent.click(within((await rows())[1]).getByRole('button', { name: 'Rename' }));
		await fireEvent.input(screen.getByLabelText('Call it'), { target: { value: 'Work' } });
		await fireEvent.click(within((await rows())[1]).getByRole('button', { name: 'Save' }));

		expect(await screen.findByRole('alert')).toHaveTextContent(
			'no live Session with that id belongs to this Person',
		);
		expect(screen.getByLabelText('Call it')).toHaveValue('Work');
	});
});

// #131, screen choices 2 and 5: your Cookbook, a page of its own since #469,
// with the one-use link that makes it a Cookbook you write with somebody.
describe('your Cookbook (#131)', () => {
	const signedIn = (over: Answers = {}): Answers => ({
		instance_status: {
			version: '0.1.0',
			setup_complete: true,
			password_minimum: 15,
			passkey_origin: null,
			demo: false,
		},
		list_sessions: { sessions: [] },
		list_passkeys: NO_PASSKEYS,
		list_access_keys: { access_keys: [] },
		...readsInAmerican,
		list_kitchens: { kitchens: [] },
		...over,
	});
	/** The page, once its card has drawn. */
	const card = async () => {
		await screen.findByLabelText('Your Cookbook’s name');
		return document.body;
	};
	const TOGETHER = cookbookOf({
		authors: [
			{ person_id: 'p_1', name: 'Stéphane' },
			{ person_id: 'p_2', name: 'Camille' },
		],
		recipe_count: 99,
	});

	it('says who writes it and how many recipes, and is named after them until renamed', async () => {
		renderScreen(Cookbook, signedIn());
		const book = within(await card());
		const name = book.getByLabelText('Your Cookbook’s name');
		expect(name).toHaveValue('');
		expect(name).toHaveAttribute('placeholder', 'Stéphane’s');
		expect(book.getByText('Who can change your recipes.')).toBeInTheDocument();
		expect(book.getByText('87 recipes.')).toBeInTheDocument();
		// A Cookbook of one has nobody to leave.
		expect(book.queryByRole('button', { name: 'Leave this Cookbook' })).not.toBeInTheDocument();
	});

	it('renames it, and clears the name back to its authors’ with an empty box', async () => {
		const { kamosu } = renderScreen(
			Cookbook,
			signedIn({ rename_cookbook: cookbookOf({ name: 'Chez nous' }) }),
		);
		const book = within(await card());
		await fireEvent.input(book.getByLabelText('Your Cookbook’s name'), {
			target: { value: '  Chez nous ' },
		});
		await fireEvent.click(book.getByRole('button', { name: 'Save' }));
		await waitFor(() =>
			expect(kamosu.calls).toContainEqual({
				operation: 'rename_cookbook',
				input: { name: 'Chez nous' },
			}),
		);

		await fireEvent.input(book.getByLabelText('Your Cookbook’s name'), { target: { value: '' } });
		await fireEvent.click(book.getByRole('button', { name: 'Save' }));
		await waitFor(() =>
			expect(kamosu.calls).toContainEqual({ operation: 'rename_cookbook', input: { name: null } }),
		);
	});

	it('mints a one-use link on this instance, copies it, and cancels it', async () => {
		let minted = false;
		const { kamosu } = renderScreen(
			Cookbook,
			signedIn({
				invite_to_cookbook: () => {
					minted = true;
					return { invite_id: 'ci_1', secret: 'the-cookbook-secret' };
				},
				get_cookbook: () =>
					cookbookOf({
						invites: minted ? [{ invite_id: 'ci_1', created_at: '2026-09-24T08:00:00Z' }] : [],
					}),
				cancel_cookbook_invite: () => {
					minted = false;
					return { ended: true };
				},
			}),
		);
		const writeText = vi.fn(async () => {});
		Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });

		await fireEvent.click(
			within(await card()).getByRole('button', { name: 'Write it with someone' }),
		);
		const link = `${location.origin}/cookbook-invite/the-cookbook-secret`;
		expect(await screen.findByText(link)).toBeInTheDocument();
		expect(screen.getByText('Send this link to the person. It works once.')).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Copy' }));
		expect(writeText).toHaveBeenCalledWith(link);
		expect(await screen.findByRole('button', { name: 'Copied' })).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Cancel it' }));
		await waitFor(() =>
			expect(kamosu.calls).toContainEqual({
				operation: 'cancel_cookbook_invite',
				input: { invite_id: 'ci_1' },
			}),
		);
		await waitFor(() => expect(screen.queryByText(link)).not.toBeInTheDocument());
		Reflect.deleteProperty(navigator, 'clipboard');
	});

	it('offers to end a link minted on an earlier visit, which it cannot show again', async () => {
		const { kamosu } = renderScreen(
			Cookbook,
			signedIn({
				get_cookbook: cookbookOf({
					invites: [{ invite_id: 'ci_old', created_at: '2026-09-20T08:00:00Z' }],
				}),
				cancel_cookbook_invite: { ended: true },
			}),
		);
		const book = within(await card());
		expect(
			book.getByText('An invite to write it with you is waiting to be opened.'),
		).toBeInTheDocument();
		await fireEvent.click(book.getByRole('button', { name: 'Cancel it' }));
		await waitFor(() =>
			expect(kamosu.calls).toContainEqual({
				operation: 'cancel_cookbook_invite',
				input: { invite_id: 'ci_old' },
			}),
		);
	});

	it('asks before leaving a Cookbook written together, saying what each keeps', async () => {
		const { kamosu } = renderScreen(
			Cookbook,
			signedIn({ get_cookbook: TOGETHER, leave_cookbook: cookbookOf({ recipe_count: 99 }) }),
		);
		const book = within(await card());
		await fireEvent.click(book.getByRole('button', { name: 'Leave this Cookbook' }));

		const sheet = within(await screen.findByRole('dialog'));
		expect(sheet.getByRole('heading', { name: 'Leave Stéphane and Camille?' })).toBeInTheDocument();
		expect(
			sheet.getByText(
				'You take a copy of all 99 recipes, with their history. The others keep theirs.',
			),
		).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('leave_cookbook');

		await fireEvent.click(sheet.getByRole('button', { name: 'Leave' }));
		await waitFor(() =>
			expect(kamosu.calls.map((call) => call.operation)).toContain('leave_cookbook'),
		);
		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
	});

	it('asks before removing a Co-author, and removes them by the Operation', async () => {
		const { kamosu } = renderScreen(
			Cookbook,
			signedIn({ get_cookbook: TOGETHER, remove_cookbook_author: cookbookOf() }),
		);
		const camille = (await card()).querySelectorAll('li');
		const row = [...camille].find((li) => li.textContent?.includes('Camille')) as HTMLElement;
		await fireEvent.click(within(row).getByRole('button', { name: 'Remove' }));

		const sheet = within(await screen.findByRole('dialog'));
		expect(sheet.getByRole('heading', { name: 'Stop writing with Camille?' })).toBeInTheDocument();
		await fireEvent.click(sheet.getByRole('button', { name: 'Remove Camille' }));
		await waitFor(() =>
			expect(kamosu.calls).toContainEqual({
				operation: 'remove_cookbook_author',
				input: { person_id: 'p_2' },
			}),
		);
	});

	describe('a join waiting on answers (#135)', () => {
		type Join = GetCookbookOutput['joins'][number];
		/** Stéphane (you, p_1) said yes to Hélène's Invite; Hélène writes with Bob. */
		const joinOf = (over: Partial<Join> = {}): Join => ({
			join_id: 'cj_1',
			state: 'waiting',
			accepted_by: { person_id: 'p_1', name: 'Stéphane' },
			invited_by: { person_id: 'p_3', name: 'Hélène' },
			joining: { id: 'c_1', name: null, authors: TOGETHER.authors },
			into: {
				id: 'c_2',
				name: null,
				authors: [
					{ person_id: 'p_3', name: 'Hélène' },
					{ person_id: 'p_4', name: 'Bob' },
				],
			},
			together_recipes: 120,
			waiting_on: [
				{ person_id: 'p_2', name: 'Camille' },
				{ person_id: 'p_4', name: 'Bob' },
			],
			you: 'accepted',
			refused_by: null,
			refused_by_co_author: false,
			...over,
		});

		it('says whom it waits for, and lets the one who accepted take the yes back', async () => {
			const { kamosu } = renderScreen(
				Cookbook,
				signedIn({
					get_cookbook: { ...TOGETHER, joins: [joinOf()] },
					answer_cookbook_join: TOGETHER,
				}),
			);
			const book = within(await card());
			expect(book.getByText('One Cookbook with Hélène and Bob')).toBeInTheDocument();
			expect(book.getByText('Waiting for Camille and Bob to say yes.')).toBeInTheDocument();
			await fireEvent.click(book.getByRole('button', { name: 'Take my yes back' }));
			await waitFor(() =>
				expect(kamosu.calls).toContainEqual({
					operation: 'answer_cookbook_join',
					input: { join_id: 'cj_1', yes: false },
				}),
			);
			await waitFor(() =>
				expect(
					screen.queryByText('Waiting for Camille and Bob to say yes.'),
				).not.toBeInTheDocument(),
			);
		});

		it('offers anyone else in it the same no, as calling it off', async () => {
			renderScreen(
				Cookbook,
				signedIn({ get_cookbook: { ...TOGETHER, joins: [joinOf({ you: 'invited' })] } }),
			);
			const book = within(await card());
			expect(book.getByRole('button', { name: 'Call it off' })).toBeInTheDocument();
		});

		it('asks the question here too, to someone it waits on', async () => {
			const { kamosu } = renderScreen(
				Cookbook,
				signedIn({
					get_cookbook: { ...TOGETHER, joins: [joinOf({ you: 'asked' })] },
					answer_cookbook_join: TOGETHER,
				}),
			);
			const book = within(await card());
			expect(
				book.getByRole('heading', { name: 'Write one Cookbook with Hélène and Bob?' }),
			).toBeInTheDocument();
			await fireEvent.click(book.getByRole('button', { name: 'Write together' }));
			await waitFor(() =>
				expect(kamosu.calls).toContainEqual({
					operation: 'answer_cookbook_join',
					input: { join_id: 'cj_1', yes: true },
				}),
			);
		});

		it('tells the one who accepted who said no, and how to join anyway', async () => {
			renderScreen(
				Cookbook,
				signedIn({
					get_cookbook: {
						...TOGETHER,
						joins: [
							joinOf({
								state: 'refused',
								waiting_on: [],
								refused_by: { person_id: 'p_2', name: 'Camille' },
								refused_by_co_author: true,
							}),
						],
					},
				}),
			);
			const book = within(await card());
			expect(book.getByText('Camille said no, so nothing changed.')).toBeInTheDocument();
			expect(
				book.getByText(
					'To join anyway, leave this Cookbook first. You keep a copy of every recipe. Then open the same link again.',
				),
			).toBeInTheDocument();
		});

		it('gives no leaving advice where the no came from the other side', async () => {
			renderScreen(
				Cookbook,
				signedIn({
					get_cookbook: {
						...TOGETHER,
						joins: [
							joinOf({
								state: 'refused',
								waiting_on: [],
								refused_by: { person_id: 'p_4', name: 'Bob' },
							}),
						],
					},
				}),
			);
			const book = within(await card());
			expect(book.getByText('Bob said no, so nothing changed.')).toBeInTheDocument();
			expect(book.queryByText(/To join anyway/)).not.toBeInTheDocument();
		});
	});

	it('heads the Tags with your Cookbook, the only one whose words you rename', async () => {
		renderScreen(
			Tags,
			signedIn({ get_cookbook: cookbookOf({ name: 'Chez nous' }), list_tags: { tags: [] } }),
		);
		expect(await screen.findByRole('heading', { name: 'Chez nous' })).toBeInTheDocument();
	});
});
