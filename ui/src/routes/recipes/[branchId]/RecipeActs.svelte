<!--
	What a cook does with a recipe, on its first screen (#649): option A of the
	prototype chosen on 20 November 2026 (`en-short-A-phone.html`). Under Start
	cooking, one row of four acts, each its icon over its word: Plan, Shopping
	list, Share and More. More opens the rarer ones (`MoreSheet`).

	They stood in a grid of six at the foot of the page before, after the whole
	method, where three of the UX audit's twelve tasks had to scroll and hunt
	for them (P2 of 19 November 2026, rated 3 by all three evaluators).

	**What the last act did is said under the row, in one card** (#654, the
	prototype's `planned.html` and `listed.html`, chosen on 27 November 2026):
	what happened, a way to see it, and Undo, the same for planning as for the
	shopping list, where the cook is looking (U12). Undo takes back that one
	act alone and leaves a line saying so. The card shows the last act only, so
	two never stand together (U7). Plan stays Plan once used: pressing it again
	plans the recipe once more, and the sheet shows the days it is already on.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import NoticeCard from '#lib/NoticeCard.svelte';
	import { said, type Wanted, type Written, type YieldNouns } from '#lib/how-much.js';
	import { weekOfWhere, type Where } from '#lib/plan-days.js';
	import { whereName } from '#lib/plan-where.js';
	import ActIcon from './ActIcon.svelte';
	import PlanThis from './PlanThis.svelte';
	import ShoppingListButton from './ShoppingListButton.svelte';

	/**
	 * What "Plan this" plans: the Branch the page has read, its title, what it
	 * makes as written and the nouns it is said in.
	 */
	type Plans = { branchId: string; title: string; written: Written; nouns?: YieldNouns };

	interface Props {
		branchId: string;
		/**
		 * What Plan plans, once the page has read this recipe, and null until
		 * then: walking to another recipe holds the last one's words while the
		 * next is read, and the sheet must name the recipe it plans.
		 */
		plans: Plans | null;
		scaledTo: Wanted;
		onTheList: boolean;
		onMore: () => void;
	}

	let { branchId, plans, scaledTo, onTheList = $bindable(), onMore }: Props = $props();

	const kamosu = useKamosu();

	/** "Plan this" is open. */
	let planning = $state(false);

	/**
	 * What the last act from this page did, while it has not been left since.
	 * The page keys this component on its recipe, so walking to another recipe
	 * never carries one recipe's Undo onto the next.
	 */
	type Said =
		| { kind: 'listed'; lines: number }
		| { kind: 'unlisted' }
		| { kind: 'planned'; where: Where; wanted: Wanted; id: string | null }
		| { kind: 'unplanned'; where: Where };
	let last = $state<Said | null>(null);
	let undoing = $state(false);
	/** Undo was refused: said under the card, which stays, until the next act. */
	let undoFailed = $state(false);

	/** A new act from the row: what it did replaces whatever was said before. */
	function did(act: Said) {
		last = act;
		undoFailed = false;
	}

	async function undo() {
		const act = last;
		if (!act || undoing) return;
		undoing = true;
		undoFailed = false;
		try {
			let undone: Said | null = null;
			if (act.kind === 'listed') {
				await kamosu.removeFromShoppingList({ branch_id: branchId });
				onTheList = false;
				undone = { kind: 'unlisted' };
			} else if (act.kind === 'planned' && act.id) {
				await kamosu.removePlannedMeal({ planned_meal_id: act.id });
				undone = { kind: 'unplanned', where: act.where };
			}
			// An act made while Undo was on its way is the one to say.
			if (undone && last === act) last = undone;
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
			if (last === act) undoFailed = true;
		} finally {
			undoing = false;
		}
	}
</script>

<div class="mx-gutter mt-3 grid grid-cols-4 gap-2">
	<button
		type="button"
		class="second-button act-stack"
		disabled={plans === null}
		onclick={() => (planning = true)}
	>
		<ActIcon name="plan" />{m.recipe_act_plan()}
	</button>
	<ShoppingListButton
		{branchId}
		{scaledTo}
		bind:onTheList
		onAdded={(lines) => did({ kind: 'listed', lines })}
	/>
	<a href="/recipes/{branchId}/share" class="second-button act-stack">
		<ActIcon name="share" />{m.recipe_act_share()}
	</a>
	<button type="button" class="second-button act-stack" aria-haspopup="dialog" onclick={onMore}>
		<ActIcon name="more" />{m.recipe_act_more()}
	</button>
</div>

{#snippet happened(words: string, see: string, seeHref: string, undoable: boolean)}
	<NoticeCard tone="waiting" class="mx-gutter mt-3">
		<p class="text-body">{words}</p>
		<div class="mt-2 flex flex-wrap gap-2">
			<a href={seeHref} class="quiet-button text-accent">{see}</a>
			{#if undoable}
				<button
					type="button"
					class="quiet-button text-accent"
					disabled={undoing}
					aria-busy={undoing ? 'true' : undefined}
					onclick={undo}>{m.recipe_added_undo()}</button
				>
			{/if}
		</div>
		{#if undoFailed}
			<p class="mt-2 text-read text-support" role="alert">{m.plan_write_failed()}</p>
		{/if}
	</NoticeCard>
{/snippet}

{#snippet undone(words: string)}
	<p class="mx-gutter mt-3 text-read text-accent" role="status">{words}</p>
{/snippet}

{#if last?.kind === 'listed' && onTheList}
	{@render happened(
		m.recipe_added_to_list({ count: last.lines }),
		m.recipe_added_see_list(),
		'/shopping',
		true,
	)}
{:else if last?.kind === 'planned'}
	{@const amount = said(last.wanted, plans?.nouns)}
	{@render happened(
		amount
			? m.plan_planned_for_amount({ day: whereName(last.where), amount })
			: m.plan_planned_for({ day: whereName(last.where) }),
		m.plan_see(),
		`/plan?week=${weekOfWhere(last.where)}`,
		last.id !== null,
	)}
{:else if last?.kind === 'unlisted'}
	{@render undone(m.recipe_list_undone())}
{:else if last?.kind === 'unplanned'}
	{@render undone(m.plan_undone({ day: whereName(last.where) }))}
{/if}

{#if planning && plans}
	<PlanThis
		branchId={plans.branchId}
		title={plans.title}
		written={plans.written}
		nouns={plans.nouns}
		onPlanned={(where, wanted, id) => {
			planning = false;
			did({ kind: 'planned', where, wanted, id });
		}}
		onClose={() => (planning = false)}
	/>
{/if}
