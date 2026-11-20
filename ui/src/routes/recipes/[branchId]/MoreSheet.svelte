<!--
	The rarer acts on a recipe (#649), behind More in the row under Start
	cooking: option A of the prototype chosen on 20 November 2026
	(`en-short-A-phone.html?state=more`). Each is a row of the shared list
	(`list-row`, #632) with its icon, its word and the arrow that says it
	opens something. Deleting stands apart under a rule, in beni, and asks
	once before it does anything (U11), as it did at the foot of the page.

	Every act here but the History is on the server's side of the line, so
	offline its row stays where it is, greyed, saying what it waits for, as
	`NeedsServer` draws a button (#76, ADR 0013).
-->
<script lang="ts">
	import { tick } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import SheetFrame from '#lib/SheetFrame.svelte';
	import { Online } from '#lib/offline/device.svelte.js';
	import type { Sheet } from './sheet.svelte';
	import ActIcon from './ActIcon.svelte';
	import type { ActIconName } from './act-icons';

	interface Props {
		branchId: string;
		/** Printing a Sheet, whose label says when one is being set or is ready to share. */
		paper: Sheet;
		/** The version's name may be set here, under this label (#134). */
		rename?: string;
		/** Deleting is offered: the recipe is one this reader writes. */
		deletes: boolean;
		onEdit: () => void;
		onTranslate: () => void;
		onPrint: () => void;
		onRename: () => void;
		onDelete: () => void;
		onClose: () => void;
	}

	let {
		branchId,
		paper,
		rename,
		deletes,
		onEdit,
		onTranslate,
		onPrint,
		onRename,
		onDelete,
		onClose,
	}: Props = $props();

	const uid = $props.id();
	const online = new Online();

	/**
	 * Close this sheet and do the act in the same tap: printing opens a tab or
	 * the share sheet, which a browser allows only inside the tap itself.
	 */
	const closing = (act: () => void) => () => {
		onClose();
		act();
	};

	/**
	 * Close this sheet, then do an act that opens a sheet of its own.
	 * The act waits until the caret is back on More, a frame after this sheet
	 * has gone (`SheetFrame`): a sheet the act opens then takes the caret
	 * from there and gives it back there, instead of losing it to More's
	 * return a frame after it opened.
	 */
	const closingFirst = (act: () => void) => async () => {
		onClose();
		await tick();
		requestAnimationFrame(() => requestAnimationFrame(act));
	};
</script>

{#snippet row(
	icon: ActIconName,
	label: string,
	waiting: string | null,
	act: () => void,
	disabled = false,
)}
	<li>
		{#if waiting !== null && !online.current}
			<button type="button" disabled class="list-row text-ink-2 opacity-55">
				<ActIcon name={icon} /><span class="min-w-0 flex-1 text-body">{waiting}</span>
			</button>
		{:else}
			<button type="button" class="list-row text-accent" {disabled} onclick={act}>
				<ActIcon name={icon} /><span class="min-w-0 flex-1 text-body font-medium">{label}</span>
				{@render arrow()}
			</button>
		{/if}
	</li>
{/snippet}

{#snippet arrow()}
	<svg
		viewBox="0 0 24 24"
		aria-hidden="true"
		class="h-[20px] w-[20px] shrink-0 text-ink-2"
		fill="none"
		stroke="currentColor"
		stroke-width="1.75"
		stroke-linecap="round"
		stroke-linejoin="round"><path d="m9 5 7 7-7 7" /></svg
	>
{/snippet}

<SheetFrame
	labelledby="{uid}-title"
	class="overflow-y-auto bg-ground px-gutter pt-6 pb-6"
	onclose={onClose}
>
	<h2 id="{uid}-title" class="mb-3 font-display text-list-title font-semibold text-ink">
		{m.recipe_more_title()}
	</h2>
	<ul class="list-rows">
		{@render row('edit', m.write_edit(), m.offline_waits_edit(), closing(onEdit))}
		<li>
			<a href="/recipes/{branchId}/thread" class="list-row text-accent" onclick={onClose}>
				<ActIcon name="history" /><span class="min-w-0 flex-1 text-body font-medium"
					>{m.recipe_the_thread()}</span
				>
				{@render arrow()}
			</a>
		</li>
		{@render row(
			'translate',
			m.recipe_act_translate(),
			m.offline_waits_edit(),
			closingFirst(onTranslate),
		)}
		{#if rename}
			{@render row('name', rename, m.offline_waits_version_name(), closingFirst(onRename))}
		{/if}
		{#if paper.printing === 'ready'}
			<!-- Already on the phone, so sharing it waits for nothing (#149). -->
			{@render row('print', m.recipe_share_sheet(), null, closing(paper.share))}
		{:else}
			{@render row(
				'print',
				paper.setting ? m.recipe_print_setting() : m.recipe_print_sheet(),
				m.offline_waits_print(),
				closing(onPrint),
				paper.setting,
			)}
		{/if}
	</ul>
	{#if deletes}
		<div class="mt-6 border-t border-rule pt-4">
			{#if online.current}
				<button type="button" class="second-button w-full loses" onclick={closingFirst(onDelete)}>
					<ActIcon name="delete" class="h-[20px] w-[20px] shrink-0" />{m.recipe_delete()}
				</button>
			{:else}
				<button type="button" disabled class="second-button w-full loses">
					{m.offline_waits_delete()}
				</button>
			{/if}
		</div>
	{/if}
</SheetFrame>
