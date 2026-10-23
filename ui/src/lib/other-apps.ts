/**
 * The apps whose export Kamosu reads (#412), each with where a cook finds its
 * export. The server recognises a file by what is in it; this list is what
 * the screens say about it, and holds exactly the apps `other_apps::READERS`
 * holds on the server. Each later ticket of #411 adds its app to both.
 */

import { m } from '$lib/paraglide/messages';

export interface OtherApp {
	/** The source kind its Imports are kept under. */
	kind: string;
	/** Its name as a cook knows it, which no language translates. */
	name: string;
	/** Where its export is, in a sentence. */
	where: () => string;
}

export const OTHER_APPS: OtherApp[] = [
	{ kind: 'mealie', name: 'Mealie', where: m.other_app_where_mealie },
	{ kind: 'paprika', name: 'Paprika', where: m.other_app_where_paprika },
	{ kind: 'tandoor', name: 'Tandoor', where: m.other_app_where_tandoor },
	{ kind: 'crouton', name: 'Crouton', where: m.other_app_where_crouton },
	// One reader reads the schema.org recipe files these four write, and each
	// lands under its own kind (#416).
	{
		kind: 'nextcloud_cookbook',
		name: 'Nextcloud Cookbook',
		where: m.other_app_where_nextcloud_cookbook,
	},
	{ kind: 'recipesage', name: 'RecipeSage', where: m.other_app_where_recipesage },
	{ kind: 'recipya', name: 'Recipya', where: m.other_app_where_recipya },
	{ kind: 'cookcli', name: 'CookCLI', where: m.other_app_where_cookcli },
	{ kind: 'bar_assistant', name: 'Bar Assistant', where: m.other_app_where_bar_assistant },
];

/** Where a cook asks for another app. */
export const ASK_FOR_AN_APP = 'https://github.com/BatterWorks/Kamosu/issues';

/** An app's name by its source kind, or nothing for a source no app is. */
export const appNamed = (kind: string | undefined): string | undefined =>
	OTHER_APPS.find((app) => app.kind === kind)?.name;

/** Every name, as a sentence lists them: "Mealie, Paprika and Crouton". */
export function appNames(): string {
	const names = OTHER_APPS.map((app) => app.name);
	return names.length < 2
		? (names[0] ?? '')
		: m.other_apps_list({ some: names.slice(0, -1).join(', '), last: names.at(-1)! });
}

/** The first few names and how many more, for one line on the Settings menu. */
export function appsInBrief(): string {
	const SHOWN = 4;
	const names = OTHER_APPS.map((app) => app.name);
	if (names.length <= SHOWN) return names.join(', ');
	return m.other_apps_and_more({
		some: names.slice(0, SHOWN).join(', '),
		count: names.length - SHOWN,
	});
}
