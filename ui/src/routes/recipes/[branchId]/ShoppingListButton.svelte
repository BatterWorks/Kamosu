<!--
	Onto the Shopping List (#73, ADR 0024), as one of the four acts under Start
	cooking (#649, option A). A button and not a link while the recipe is not
	on the list: adding is one act that finishes here. What it stores is the
	choosing — the Branch, at whatever Version it is on when the list is next
	read.

	Once the recipe is on the list it is a link to the list, filled, saying so
	(`en-short-A-phone.html?state=added`). Taking it back off is the Undo the
	page offers beside what was added (U12), and the list's own business after
	that; a second tap on the same button taking the recipe off again was the
	surprise the UX audit's U12 rules out.

	Whether the recipe is on the list is the recipe screen's to know, since the
	list's amount is what the page opens at (#109); this only changes it.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import type { Wanted } from '#lib/how-much.js';
	import ActIcon from './ActIcon.svelte';

	interface Props {
		branchId: string;
		/** What the page is scaled to, or null where it is as written. */
		scaledTo: Wanted;
		onTheList: boolean;
		/**
		 * It has just gone on the list from here, with how many of the list's
		 * lines came from it, so the page can say what went. Counted off the
		 * list's answer: a line naming another recipe puts that recipe's own
		 * lines on the list, never itself (#50).
		 */
		onAdded: (lines: number) => void;
	}

	let { branchId, scaledTo, onTheList = $bindable(), onAdded }: Props = $props();

	const kamosu = useKamosu();
	let adding = $state(false);

	async function add() {
		adding = true;
		try {
			// At the amount on screen (#109): what the errands scaler is for.
			const list = await kamosu.addToShoppingList(
				scaledTo ? { branch_id: branchId, shopping_yield: scaledTo } : { branch_id: branchId },
			);
			onTheList = true;
			onAdded(
				list.rows.flatMap((row) => row.lines).filter((line) => line.branch_id === branchId).length,
			);
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
		} finally {
			adding = false;
		}
	}
</script>

{#if onTheList}
	<a href="/shopping" class="main-button act-stack">
		<ActIcon name="onList" />{m.recipe_act_on_list()}
	</a>
{:else}
	<button
		type="button"
		disabled={adding}
		aria-busy={adding ? 'true' : undefined}
		class="second-button act-stack"
		onclick={add}
	>
		<ActIcon name="shopping" />{m.recipe_act_shopping()}
	</button>
{/if}
