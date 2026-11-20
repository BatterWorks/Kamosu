<!--
	What a cook does with a recipe, on its first screen (#649): option A of the
	prototype chosen on 20 November 2026 (`en-short-A-phone.html`). Under Start
	cooking, one row of four acts, each its icon over its word: Plan, Shopping
	list, Share and More. More opens the rarer ones (`MoreSheet`).

	They stood in a grid of six at the foot of the page before, after the whole
	method, where three of the UX audit's twelve tasks had to scroll and hunt
	for them (P2 of 19 November 2026, rated 3 by all three evaluators).

	Once the recipe goes on the Shopping List from here the page says how much
	went, with an Undo, in the place the cook is looking (U12).
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import NoticeCard from '#lib/NoticeCard.svelte';
	import type { Wanted } from '#lib/how-much.js';
	import ActIcon from './ActIcon.svelte';
	import ShoppingListButton from './ShoppingListButton.svelte';

	interface Props {
		branchId: string;
		scaledTo: Wanted;
		onTheList: boolean;
		onPlan: () => void;
		onMore: () => void;
	}

	let { branchId, scaledTo, onTheList = $bindable(), onPlan, onMore }: Props = $props();

	const kamosu = useKamosu();

	/**
	 * How many lines went on the list from this page, while it has not been
	 * left since. The page keys this component on its recipe, so walking to
	 * another recipe never carries one recipe's Undo onto the next.
	 */
	let added = $state<number | null>(null);
	let undoing = $state(false);

	async function undo() {
		undoing = true;
		try {
			await kamosu.removeFromShoppingList({ branch_id: branchId });
			onTheList = false;
			added = null;
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
		} finally {
			undoing = false;
		}
	}
</script>

<div class="mx-gutter mt-3 grid grid-cols-4 gap-2">
	<button type="button" class="second-button act-stack" onclick={onPlan}>
		<ActIcon name="plan" />{m.recipe_act_plan()}
	</button>
	<ShoppingListButton {branchId} {scaledTo} bind:onTheList onAdded={(lines) => (added = lines)} />
	<a href="/recipes/{branchId}/share" class="second-button act-stack">
		<ActIcon name="share" />{m.recipe_act_share()}
	</a>
	<button type="button" class="second-button act-stack" aria-haspopup="dialog" onclick={onMore}>
		<ActIcon name="more" />{m.recipe_act_more()}
	</button>
</div>

{#if added !== null && onTheList}
	<NoticeCard tone="waiting" class="mx-gutter mt-3">
		<div class="flex items-center gap-3">
			<p class="min-w-0 flex-1 text-body">{m.recipe_added_to_list({ count: added })}</p>
			<button
				type="button"
				class="quiet-button text-accent"
				disabled={undoing}
				aria-busy={undoing ? 'true' : undefined}
				onclick={undo}>{m.recipe_added_undo()}</button
			>
		</div>
	</NoticeCard>
{/if}
