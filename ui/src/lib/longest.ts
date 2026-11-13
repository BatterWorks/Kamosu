/**
 * The most characters a field takes, as its Operation declares it (#538).
 *
 * Read from the Catalogue rather than written here, so a box on a screen and
 * the Core that checks what it sends cannot disagree. A box that stops at the
 * limit matters most offline: a write the server refuses later is dropped
 * from the outbox, and its words with it.
 *
 * A browser counts `maxlength` in UTF-16 units, so a character outside the
 * basic plane counts twice there and once at the server. The box can only
 * stop short of the limit, never let past it.
 */

import { CATALOGUE, type OperationName } from '#lib/api/catalogue.js';

export function longest(operation: OperationName, field: string): number | undefined {
	const declaration = CATALOGUE.find((each) => each.name === operation);
	const properties = (declaration?.input_schema as { properties?: Record<string, unknown> })
		.properties;
	const declared = properties?.[field] as { maxLength?: number } | undefined;
	return declared?.maxLength;
}
