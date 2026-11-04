/**
 * Does this Person administer the instance? (#103)
 *
 * `get_person` says, as `is_operator` (#533). It used to be asked by calling
 * `list_accounts`, an Operator's Operation, and reading a refusal as "no":
 * every Settings screen a cook opened sent a request the Core refused, which
 * the browser logged as an error. `get_person` answers every Person, so a
 * cook is told "no" without being refused anything. A screen that needs the
 * accounts asks `list_accounts` itself, once the answer is yes.
 *
 * Written once because several screens ask it, and written twice it would be
 * two subtly different askings. It already was: two call sites treated *any*
 * refusal as "not an Operator", so an unreadable database showed a real
 * Operator an empty room and hid their own door. Three answers, not two, is
 * the fix, and it belongs in one place.
 */

import { OperationError } from '#lib/api/client.js';
import type { KamosuClient } from '#lib/api/catalogue.js';

export type Administering =
	/** They do. */
	| { may: true }
	/** They do not, or nobody is signed in to ask about. */
	| { may: false }
	/** Kamosu could not say. Not the same as "no", and never shown as one. */
	| { may: 'unknown'; failed: string };

export async function asksWhetherAdministering(kamosu: KamosuClient): Promise<Administering> {
	try {
		return { may: (await kamosu.getPerson()).is_operator };
	} catch (error) {
		if (!(error instanceof OperationError)) throw error;
		if (error.kind === 'unauthorized') return { may: false };
		return { may: 'unknown', failed: error.message };
	}
}
