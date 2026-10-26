<!--
	Settings' menu (#469, option C of 26 October 2026): your name as a card,
	then four groups by whose a thing is, each row a name and a short line
	saying what is inside. About Kamosu and Sign out stand at the foot.

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
	import { m } from '$lib/paraglide/messages';
	import Screen from '$lib/shell/Screen.svelte';
	import { OperationError } from '$lib/api/client';
	import { useKamosu } from '$lib/kamosu';
	import { useRoom } from '$lib/room.svelte';
	import { Online } from '$lib/offline/device.svelte';
	import { asksWhetherAdministering } from '$lib/operator/administering';
	import { ABOUT, GROUPS, NAME_PAGE, type PageEntry } from './pages';
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
		kamosu
			.getPerson()
			.then((person) => {
				if (live) name = person.name;
			})
			.catch((error: unknown) => {
				// Not known, then no card: the rest of the menu stands either way.
				if (!(error instanceof OperationError)) throw error;
			});
		// Only a plain yes shows the instance's group. A Kamosu that could not
		// answer is not one this Person may administer *yet*.
		asksWhetherAdministering(kamosu).then((answer) => {
			if (live) operator = answer.may === true;
		});
		return () => {
			live = false;
		};
	});

	/**
	 * The row of the page open beside the menu, brought into the menu's own
	 * view: with 21 rows, This instance's pages are below the fold of the
	 * column on a tablet.
	 */
	function inView(row: HTMLElement) {
		row.scrollIntoView?.({ block: 'nearest' });
	}

	const groups = $derived(GROUPS.filter((group) => !group.operator || operator));
</script>

{#snippet chevron()}
	<svg
		viewBox="0 0 24 24"
		aria-hidden="true"
		class="h-6 w-6 shrink-0 text-ink-2"
		fill="none"
		stroke="currentColor"
		stroke-width="1.75"
		stroke-linecap="round"
		stroke-linejoin="round"
	>
		<path d="m9 5 7 7-7 7" />
	</svg>
{/snippet}

{#snippet row(entry: PageEntry)}
	{@const here = room.wide && current === entry.href}
	<li class="border-b border-rule">
		<a
			href={entry.href}
			aria-current={here ? 'page' : undefined}
			{@attach here ? inView : undefined}
			class={[
				'flex min-h-[52px] items-center gap-3 py-2 text-ink',
				room.wide && 'px-2',
				here && 'bg-ground-2 shadow-[inset_3px_0_0_var(--color-accent)]',
			]}
		>
			<span class="min-w-0 flex-1">
				<span class="block font-display text-line font-semibold">{entry.title()}</span>
				<span class="block text-read text-ink-2">{entry.inside()}</span>
			</span>
			{@render chevron()}
		</a>
	</li>
{/snippet}

{#snippet menu()}
	{#if name}
		{@const here = room.wide && current === NAME_PAGE}
		<a
			href={NAME_PAGE}
			aria-current={here ? 'page' : undefined}
			class={[
				'flex items-center gap-3 rounded-sm border border-rule text-ink',
				room.wide ? 'px-2 py-[10px]' : 'p-3',
				here ? 'bg-ground-2 shadow-[inset_3px_0_0_var(--color-accent)]' : 'bg-card',
			]}
		>
			<span class="min-w-0 flex-1">
				<span
					class={[
						'block font-display font-semibold break-words',
						room.wide ? 'text-line' : 'text-title',
					]}>{name}</span
				>
				<span class="block text-read text-ink-2"
					>{operator ? m.settings_card_operator() : m.settings_card_cook()}</span
				>
			</span>
			{@render chevron()}
		</a>
	{/if}

	{#each groups as group (group.heading())}
		<h2 class="mt-[28px] mb-1 text-label font-medium text-accent uppercase">
			{group.heading()}
		</h2>
		<ul class="border-t border-rule">
			{#each group.entries.filter((entry) => entry.offered?.() ?? true) as entry (entry.href)}
				{@render row(entry)}
			{/each}
		</ul>
	{/each}

	<ul class="mt-[28px] border-t border-rule">
		{@render row(ABOUT)}
	</ul>

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
		<div class="min-h-0 flex-1 overflow-y-auto px-gutter pb-6">
			<h2 class="flex min-h-12 items-center font-display text-title font-semibold">
				{m.settings_title()}
			</h2>
			<div class="mt-6">{@render menu()}</div>
		</div>
	</nav>
{:else}
	<Screen title={m.settings_title()} expandsFrom="settings">
		{@render menu()}
	</Screen>
{/if}

{#if signingOut}
	<SignOut onclose={() => (signingOut = false)} />
{/if}
