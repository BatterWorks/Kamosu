/**
 * Whether this Person administers the instance, as `get_person` says (#533).
 */

import { describe, expect, it } from 'vitest';
import { asksWhetherAdministering } from './administering';
import type { KamosuClient } from '#lib/api/catalogue.js';

const answering = (person: unknown) =>
	({ getPerson: async () => person }) as unknown as KamosuClient;

describe('asking whether this Person administers the instance', () => {
	it('is a yes only when get_person says so', async () => {
		const operator = { person_id: 'p_1', name: 'Stéphane', is_operator: true };
		expect(await asksWhetherAdministering(answering(operator))).toEqual({ may: true });
		expect(await asksWhetherAdministering(answering({ ...operator, is_operator: false }))).toEqual({
			may: false,
		});
	});

	it('takes an answer kept from before get_person said it as no', async () => {
		// A phone offline after the upgrade hands back what it kept.
		const kept = { person_id: 'p_2', name: 'Camille' };
		expect(await asksWhetherAdministering(answering(kept))).toEqual({ may: false });
	});
});
