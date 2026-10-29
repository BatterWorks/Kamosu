/**
 * An Operator's act on one of This instance's pages (#103, #469): whether
 * one is running, so a button cannot fire twice, and the refusal it met.
 *
 * A refusal is shown in the words it arrived in. The Core's refusals are
 * written to be read by a person ("'Stéphane' is the only Operator this
 * instance has…"), so paraphrasing one would lose the reason and keep only
 * the fact.
 */

import { OperationError } from '$lib/api/client';

export class Acting {
	said = $state<string | undefined>(undefined);
	working = $state(false);

	/** Run an act, then what follows it, saying a refusal of either. */
	act = async (run: () => Promise<void>, after?: () => Promise<void>) => {
		this.working = true;
		this.said = undefined;
		try {
			await run();
			await after?.();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			this.said = error.message;
		} finally {
			this.working = false;
		}
	};
}
