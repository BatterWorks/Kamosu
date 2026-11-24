<!--
	Settings' menu (#469, option C of 26 October 2026): your name as a card,
	then groups by whose a thing is, each row a name and a short line saying
	what is inside. About Kamosu and Sign out stand at the foot.

	#653 (option C of 24 November 2026) put Language first under the card,
	named in every Language with a globe, and the people you cook with before
	you; the rows are `ListRow`s and the groups `Section`s, each line in full
	ink. The prototype is `.dev/prototypes/653-settings/menu-C.html`.

	**On the phone it is the whole screen at `/settings`**, reached through the
	gear in the header (#218), and each row opens its page. **On the wide
	layout it stays in a column beside whichever page is open** (ADR 0044's
	list beside what is open, #199), the page it is showing marked.

	*This instance* is drawn for an Operator alone. Whether this Person is one
	is learnt by asking something only an Operator may ask, and a refusal
	leaves the group out, as every Operator surface is left out (ADR 0040).

	Sign out is set apart at the foot (#380, option A of 20 October 2026).
	Signing out needs the server, so offline it waits for it, greyed.

	On the phone the gear and this screen's title carry the same
	`view-transition-name`, so the gear grows into the page rather than the
	screen swapping (ADR 0012). The wide layout has no gear to start from.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import Screen from '#lib/shell/Screen.svelte';
	import { OperationError } from '#lib/api/client.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { useRoom } from '#lib/room.svelte.js';
	import { Online } from '#lib/offline/device.svelte.js';
	import ListRow from '#lib/ListRow.svelte';
	import Section from '#lib/shell/Section.svelte';
	import { ABOUT, GROUPS, LANGUAGE, NAME_PAGE, type PageEntry } from './pages';
	import SignOut from './SignOut.svelte';

	interface Props {
		/** The page open beside the menu on the wide layout, marked as current. */
		current?: string;
	}

	let { current }: Props = $props();

	const kamosu = useKamosu();
	const room = useRoom();
	const online = new Online();

	let name = $state<string | undefined>(undefined);
	let operator = $state(false);
	let signingOut = $state(false);

	// Asked again whenever the page beside it changes on the wide layout,
	// where the menu stays put: a name just changed, or an Operator just
	// stood down, shows at once.
	$effect(() => {
		void current;
		let live = true;
		// One question says both who this is and whether they administer the
		// instance (#533). Only a plain yes shows the instance's group: a
		// Kamosu that could not answer is not one this Person may administer
		// *yet*.
		kamosu
			.getPerson()
			.then((person) => {
				if (!live) return;
				name = person.name;
				operator = person.is_operator === true;
			})
			.catch((error: unknown) => {
				// Not known, then no card: the rest of the menu stands either way.
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			live = false;
		};
	});

	/** The menu itself, where the row of the page open beside it is looked for. */
	let list = $state<HTMLElement | undefined>(undefined);

	/**
	 * The row of the page open beside the menu, brought into the menu's own
	 * view: with 22 rows, This instance's pages are below the fold of the
	 * column on a tablet.
	 */
	$effect(() => {
		void current;
		void operator;
		list?.querySelector<HTMLElement>('[aria-current="page"]')?.scrollIntoView?.({
			block: 'nearest',
		});
	});

	const groups = $derived(GROUPS.filter((group) => !group.operator || operator));
</script>

{#snippet row(entry: PageEntry)}
	<ListRow
		href={entry.href}
		title={entry.title()}
		line={entry.inside()}
		current={room.wide && current === entry.href}
		inkLine
	/>
{/snippet}

{#snippet globe()}
	<svg
		viewBox="0 0 24 24"
		aria-hidden="true"
		class="h-[26px] w-[26px] shrink-0 text-accent"
		fill="none"
		stroke="currentColor"
		stroke-width="1.6"
		stroke-linecap="round"
		stroke-linejoin="round"
	>
		<circle cx="12" cy="12" r="9" />
		<path
			d="M3 12h18M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9c-2.5-2.6-3.8-5.6-3.8-9S9.5 5.6 12 3z"
		/>
	</svg>
{/snippet}

{#snippet menu()}
	{#if name}
		{@const here = room.wide && current === NAME_PAGE}
		<!-- Your name as a card: a row of the list drawn larger, as #469 chose it. -->
		<a
			href={NAME_PAGE}
			aria-current={here ? 'page' : undefined}
			class={[
				'list-row rounded-sm border border-rule',
				room.wide ? 'px-2' : 'px-3',
				!here && 'bg-card',
			]}
		>
			<span class="min-w-0 flex-1">
				<span
					class={[
						'block font-display font-semibold break-words',
						room.wide ? 'text-line' : 'text-title',
					]}>{name}</span
				>
				<span class="block text-read text-ink"
					>{operator ? m.settings_card_operator() : m.settings_card_cook()}</span
				>
			</span>
			<svg
				viewBox="0 0 24 24"
				aria-hidden="true"
				class="h-[20px] w-[20px] shrink-0 text-ink-2"
				fill="none"
				stroke="currentColor"
				stroke-width="1.75"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="m9 5 7 7-7 7" />
			</svg>
		</a>
	{/if}

	<!-- Language first, named in every Language at once (#653): the person
	     who needs another language is the least able to read this list. -->
	<div class="mt-4 list-rows">
		<ListRow
			href={LANGUAGE.href}
			title={LANGUAGE.title()}
			line={LANGUAGE.inside()}
			icon={globe}
			current={room.wide && current === LANGUAGE.href}
			inkLine
		/>
	</div>

	{#each groups as group (group.heading())}
		<Section heading={group.heading()}>
			{#each group.entries.filter((entry) => entry.offered?.() ?? true) as entry (entry.href)}
				{@render row(entry)}
			{/each}
		</Section>
	{/each}

	<div class="mt-8 list-rows">
		{@render row(ABOUT)}
	</div>

	<div class="mt-3">
		<button
			type="button"
			disabled={!online.current || signingOut}
			onclick={() => (signingOut = true)}
			class={[
				'mt-1 flex min-h-12 items-center gap-[10px] font-display text-line font-semibold',
				online.current ? 'text-accent' : 'text-ink-2 opacity-55',
			]}
		>
			<svg
				viewBox="0 0 24 24"
				aria-hidden="true"
				class="h-[22px] w-[22px] shrink-0"
				fill="none"
				stroke="currentColor"
				stroke-width="1.5"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4M10 17l5-5-5-5M15 12H3" />
			</svg>
			{online.current ? m.sign_out() : m.offline_waits_sign_out()}
		</button>
	</div>
{/snippet}

{#if room.wide}
	<!-- The list beside what is open draws no Screen of its own; the page
	     beside it names the tab. -->
	<nav class="list-column" aria-label={m.settings_title()}>
		<div bind:this={list} class="min-h-0 flex-1 overflow-y-auto px-gutter pb-6 [&_.list-row]:px-2">
			<h2 class="flex min-h-12 items-center font-display text-title font-semibold">
				{m.settings_title()}
			</h2>
			<div class="mt-6">{@render menu()}</div>
		</div>
	</nav>
{:else}
	<Screen title={m.settings_title()} expandsFrom="settings">
		<div bind:this={list}>{@render menu()}</div>
	</Screen>
{/if}

{#if signingOut}
	<SignOut onclose={() => (signingOut = false)} />
{/if}
