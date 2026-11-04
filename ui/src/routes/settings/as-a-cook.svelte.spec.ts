/**
 * Every Settings screen, opened by a cook (#533). Whether somebody is an
 * Operator is `get_person`'s to say. Before, each screen asked `list_accounts`
 * and read the refusal as "no", so a cook's browser logged a refused request on
 * every Settings visit, and the offline worker took it for an ended Session.
 */

import { describe, expect, it } from 'vitest';
import { renderScreen, type ScreenComponent } from '../../testing/render';
import type { Answers } from '#lib/api/stand-in.js';
import { CATALOGUE } from '#lib/api/catalogue.js';

const screens = {
	...import.meta.glob<{ default: ScreenComponent }>('./+page.svelte', { eager: true }),
	...import.meta.glob<{ default: ScreenComponent }>('./*/+page.svelte', { eager: true }),
	...import.meta.glob<{ default: ScreenComponent }>('../operator/+page.svelte', { eager: true }),
};

/** What only an Operator may ask, and so what a cook must never be sent to ask. */
const OPERATORS_ALONE = ['list_accounts', 'list_instance_security_record'];

/**
 * A cook, told who they are and refused everything else, which every screen
 * takes in its stride: what a screen asks before any answer comes back is the
 * question here, and whether to ask the Operator's questions is `get_person`'s
 * answer alone.
 */
const aCook: Answers = {
	...Object.fromEntries(CATALOGUE.map(({ name }) => [name, { refuse: 'unauthorized' }])),
	instance_status: {
		version: '0.1.0',
		setup_complete: true,
		password_minimum: 15,
		passkey_origin: null,
		demo: false,
	},
	get_person: { person_id: 'p_2', name: 'Camille', is_operator: false },
};

describe('Settings, as a cook (#533)', () => {
	it('has the menu, the 23 pages and the old Operator address to open', () => {
		expect(Object.keys(screens)).toHaveLength(25);
	});

	it.each(Object.entries(screens))(
		'%s asks nothing an Operator alone may ask',
		async (_, { default: page }) => {
			const { kamosu } = renderScreen(page, aCook);

			// Some pages ask nothing at all. Long enough for what the first
			// answers set off to be asked too.
			await new Promise((settle) => setTimeout(settle, 100));
			const asked = kamosu.calls.map((call) => call.operation);
			for (const operation of OPERATORS_ALONE) expect(asked).not.toContain(operation);
		},
	);
});
