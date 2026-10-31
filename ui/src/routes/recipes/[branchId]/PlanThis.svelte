<!--
	"Plan this" (#486, #487): put the recipe on a day of your Meal Plan, or on
	a week with no day yet, at a Yield.

	**Chosen on 31 October 2026**, sheet 2 of the prototype recorded on #486:
	the days ahead as a list, today first, each saying what is already planned
	on it, with a heading at each week that stays in view while its days
	scroll. #487's option A put a row for any day of each week at the top of
	its days, and "Show 3 more weeks" at the foot (`PlanPicker`). Below the
	list, the recipe page's own "how much" picker, starting at the recipe as
	written.

	It plans the Branch on screen, so a cook choosing between a recipe's
	versions plans the one they are reading.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import HowMuch from '$lib/HowMuch.svelte';
	import PlanPicker from '$lib/PlanPicker.svelte';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { whereTarget } from '$lib/plan-where';
	import type { Wanted, Written } from '$lib/how-much';
	import type { Where } from '$lib/plan-days';

	interface Props {
		branchId: string;
		title: string;
		written: Written;
		/** Planned: where, at what. */
		onPlanned: (where: Where, wanted: Wanted) => void;
		onClose: () => void;
	}

	let { branchId, title, written, onPlanned, onClose }: Props = $props();

	const kamosu = useKamosu();
	const uid = $props.id();

	let chosen = $state<Where | null>(null);
	let wanted = $state<Wanted>(null);
	let working = $state(false);
	let failed = $state(false);

	async function plan() {
		if (!chosen || working) return;
		working = true;
		try {
			await kamosu.planRecipe({
				branch_id: branchId,
				...chosen,
				...(wanted ? { planned_yield: wanted } : {}),
			});
			onPlanned(chosen, wanted);
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		} finally {
			working = false;
		}
	}
</script>

<SheetFrame labelledby="{uid}-title" tall={88} class="flex flex-col bg-ground" onclose={onClose}>
	<div class="min-h-0 flex-1 overflow-y-auto px-gutter pt-4 pb-4">
		<p class="text-label text-support-2 uppercase">{m.plan_this()}</p>
		<h3 id="{uid}-title" class="mt-1 font-display text-line font-semibold">{title}</h3>
		<PlanPicker {chosen} onchoose={(where) => (chosen = where)} />
		<div class="mt-4 border-t border-rule pt-3">
			<HowMuch {written} {wanted} room="page" onchoose={(next) => (wanted = next)} />
		</div>
		{#if failed}
			<p class="mt-3 text-read text-support" role="alert">{m.plan_write_failed()}</p>
		{/if}
	</div>
	<div class="grid gap-2 border-t border-rule px-gutter py-3">
		<button
			type="button"
			class="min-h-12 rounded-sm bg-accent px-4 font-display font-semibold text-on-accent disabled:opacity-40"
			disabled={!chosen || working}
			onclick={() => void plan()}
		>
			{chosen ? m.plan_it_for({ day: whereTarget(chosen) }) : m.plan_choose_a_day()}
		</button>
		<button type="button" class="min-h-12 text-read text-accent underline" onclick={onClose}>
			{m.plan_cancel()}
		</button>
	</div>
</SheetFrame>
