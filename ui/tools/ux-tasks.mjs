/**
 * The tasks of the UX audit (#630), each declared once for `just ux-walk`
 * (#639): its goal in plain words, as an evaluator reads it; its starting
 * screen; its shortest path, each step one tap or one typed entry; and words
 * its finish line shows.
 *
 * A step names what it taps the way a person sees it, by the control's words
 * or its label for a screen reader, never by a selector, so a path breaks
 * loudly when the interface changes under it. `partial` lets the words be part
 * of a longer name, for a control whose name holds a date. `opens` marks a
 * tap that opens a menu, so the Depth table says the next control was behind
 * one.
 *
 * The everyday tasks come first, then the occasional ones. Each walks a fresh
 * copy of the Sample Library (`sample-library/story.json`), signed in as its
 * first cook.
 */

/** The cook every task is walked as: the Sample Library's first Person. */
export const FIRST_COOK = 'Camille';

/**
 * The recipe page the link task adds. The instance fetches it through its
 * own guarded fetch, which takes public addresses alone (ADR 0033), so this
 * is a real page on the web, fetched on every run. Chosen because the dev
 * library imported pages from this site without trouble.
 */
export const LINK = 'https://www.recipetineats.com/chicken-tikka-masala/';

/** @type {import('./ux-walk.mjs').Task[]} */
export const TASKS = [
	{
		id: 'find-search',
		goal: 'Find the recipe for crispy Parmesan potatoes by searching for it.',
		start: '/',
		steps: [
			{ tap: 'Recipes' },
			{ type: 'parmesan', into: 'Search recipes' },
			{ tap: 'Crispy Parmesan potatoes' },
		],
		finish: 'Cook this',
	},
	{
		id: 'find-browse',
		goal: 'Find the Orange Cake recipe without typing anything.',
		start: '/',
		steps: [{ tap: 'Recipes' }, { tap: 'Orange Cake' }],
		finish: 'Cook this',
	},
	{
		id: 'find-tag',
		goal: 'Show only the recipes tagged Dessert.',
		start: '/',
		steps: [{ tap: 'Recipes' }, { tap: 'Tags', opens: true }, { tap: 'Dessert' }],
		finish: 'Tags: Dessert',
	},
	{
		id: 'read',
		goal: 'Open the Sunday roast chicken recipe and read how to make it.',
		start: '/',
		steps: [{ tap: 'Recipes' }, { tap: 'Sunday roast chicken' }],
		finish: 'Cook this',
	},
	{
		id: 'share',
		goal: 'Share this recipe with a friend, so they can read it.',
		start: { recipe: 'Sunday roast chicken' },
		steps: [{ tap: 'Share this recipe' }, { tap: 'Make a link…' }, { tap: 'Make the link' }],
		finish: 'Anyone with this link can read this recipe.',
	},
	{
		id: 'cook',
		goal: 'Start cooking this recipe.',
		start: { recipe: 'Sunday roast chicken' },
		steps: [{ tap: 'Cook this' }, { tap: 'Start cooking' }],
		finish: 'Pause',
	},
	{
		id: 'shopping',
		goal: "Put this recipe's ingredients on the shopping list.",
		start: { recipe: 'Sunday roast chicken' },
		steps: [{ tap: 'Add to shopping list' }],
		finish: 'On your shopping list',
	},
	{
		id: 'plan',
		goal: 'Plan this recipe for tomorrow.',
		start: { recipe: 'Sunday roast chicken' },
		steps: [
			{ tap: 'Plan this' },
			{ tap: 'Tomorrow', partial: true },
			{ tap: 'Plan it for', partial: true },
		],
		finish: 'Planned for',
	},
	{
		id: 'link',
		goal: `Add the recipe at ${LINK} to your recipes.`,
		start: '/',
		steps: [
			{ tap: 'Recipes' },
			{ tap: 'Add a recipe', opens: true },
			{ tap: 'From a link', partial: true },
			{ type: LINK, into: "The recipe's web address" },
			{ tap: 'Import from a link' },
		],
		finish: 'Chicken Tikka Masala',
	},
	{
		id: 'language',
		goal: 'Change the language the app is shown in to French.',
		start: '/',
		steps: [
			{ tap: 'Settings' },
			{ tap: 'Language and measures', partial: true },
			{ tap: 'Français' },
		],
		finish: 'Langue et mesures',
		occasional: true,
	},
	{
		id: 'invite',
		goal: 'Invite a friend into your Family kitchen.',
		start: '/',
		steps: [
			{ tap: 'Settings' },
			{ tap: 'Kitchens', partial: true },
			{ tap: 'Invite someone to', partial: true },
		],
		finish: 'Works once, for the first person who opens it, until you cancel it.',
		occasional: true,
	},
	{
		id: 'old-version',
		goal: 'Find what the Orange Cake recipe said before its last change.',
		start: { recipe: 'Orange Cake' },
		steps: [{ tap: 'History' }, { tap: 'Mum read it over', partial: true }],
		finish: 'Older Version',
		occasional: true,
	},
];
