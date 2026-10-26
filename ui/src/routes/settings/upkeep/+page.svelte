<!--
	Upkeep (#103, #166, #469): a page of This instance holding what was the
	Operator screen's Worklist, the things that pile up and want working
	through. Foods that look alike, Foods no recipe uses, the Photographs
	nothing shows, and reading every Ingredient Line again.
-->
<script lang="ts">
	import Refusal from '$lib/Refusal.svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { waitForJob } from '$lib/api/job';
	import Section from '$lib/shell/Section.svelte';
	import Confirm from '$lib/Confirm.svelte';
	import { Acting } from '$lib/operator/acting.svelte';
	import type {
		ListFoodsOutput,
		ListMergeSuggestionsOutput,
		PreviewFoodMergeOutput,
		RereadIngredientLinesOutput,
		SweepPhotographsOutput,
	} from '$lib/api/catalogue';
	import OperatorOnly from '../OperatorOnly.svelte';

	const kamosu = useKamosu();
	const acting = new Acting();

	/** Reading a library's lines is inference over every recipe: minutes on a small machine. */
	const LONG_ENOUGH = 30 * 60_000;

	type Suggestion = ListMergeSuggestionsOutput['suggestions'][number];
	type Food = ListFoodsOutput['foods'][number];

	let suggestions = $state<Suggestion[]>([]);
	let foods = $state<Food[]>([]);

	/**
	 * How many unused Foods are listed at once. A library can carry hundreds,
	 * and a phone showing all of them is a scroll nobody works through. The
	 * line under the list says the whole figure rather than letting the cap
	 * pass for the total.
	 */
	const SHOWN = 20;

	/** A Food nothing points at is the only kind `delete_food` will take. */
	const unused = $derived(foods.filter((food) => food.reading_count === 0));

	/** Each read caught on its own, so one that fails empties nothing else. */
	async function load() {
		const quietly = async (run: () => Promise<void>) => {
			try {
				await run();
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
			}
		};
		await Promise.all([
			quietly(async () => {
				suggestions = (await kamosu.listMergeSuggestions({})).suggestions;
			}),
			quietly(async () => {
				foods = (await kamosu.listFoods({})).foods;
			}),
		]);
	}

	type Asking =
		| { kind: 'delete-food'; food: Food }
		| { kind: 'sweep' }
		| { kind: 'merge'; preview: PreviewFoodMergeOutput };

	let asking = $state<Asking | undefined>(undefined);

	function close() {
		asking = undefined;
		acting.said = undefined;
	}

	// ── Foods ─────────────────────────────────────────────────────────────────

	/** A Food's name in whatever language it has one, never its id. */
	const naming = (food: { name: string | null; id: string }) => food.name ?? food.id;

	/**
	 * What to call one Food of a pair when the pair is being told apart. Two
	 * Foods can carry the same name: doubt makes a new Food rather than a merge
	 * (ADR 0022), and a Merge gives the survivor every name both held. The
	 * language tells them apart where it can, the id where it cannot.
	 */
	function telling(food: Food, other: Food): string {
		const name = naming(food);
		if (name !== naming(other)) return name;
		if (food.language && food.language !== other.language) return `${name} (${food.language})`;
		return `${name} (${food.id.slice(2, 8)})`;
	}

	/** Why Kamosu thinks two Foods are one thing, in words rather than a code. */
	const because = (reason: Suggestion['reason']) =>
		reason === 'arrived_as_one' ? m.operator_merge_arrived() : m.operator_merge_typed();

	const askMerge = (survivor: Food, absorbed: Food) =>
		acting.act(async () => {
			asking = {
				kind: 'merge',
				preview: await kamosu.previewFoodMerge({
					survivor_food_id: survivor.id,
					absorbed_food_id: absorbed.id,
				}),
			};
		});

	const merge = (preview: PreviewFoodMergeOutput) =>
		acting.act(
			async () => {
				await kamosu.mergeFood({
					survivor_food_id: preview.survivor.id,
					absorbed_food_id: preview.absorbed.id,
					// The figure the preview announced, said back. A Merge that does
					// not match it is refused: that is the safety net, so it is sent
					// from the preview and never recomputed.
					ingredient_lines: preview.ingredient_lines,
				});
			},
			async () => {
				await load();
				close();
			},
		);

	const deleteFood = (food: Food) =>
		acting.act(
			async () => {
				await kamosu.deleteFood({ food_id: food.id });
			},
			async () => {
				await load();
				close();
			},
		);

	// ── Photographs, and reading the library's lines ──────────────────────────

	let swept = $state<SweepPhotographsOutput | undefined>(undefined);
	let reread = $state<RereadIngredientLinesOutput | undefined>(undefined);
	let readingLines = $state(false);
	let allChanges = $state(false);

	const sweep = () =>
		acting.act(
			async () => {
				swept = await kamosu.sweepPhotographs({});
			},
			async () => close(),
		);

	/**
	 * Read every line in the library again (#166). The Foods it leaves with
	 * nothing pointing at them join the list above, so that is asked again.
	 */
	async function readLines() {
		readingLines = true;
		acting.said = undefined;
		try {
			const done = await waitForJob(kamosu, (await kamosu.rereadIngredientLines({})).job_id, {
				giveUpAfter: LONG_ENOUGH,
			});
			reread = done.result as RereadIngredientLinesOutput;
			allChanges = false;
			foods = (await kamosu.listFoods({})).foods;
		} catch (error) {
			if (!(error instanceof Error)) throw error;
			acting.said = error.message;
		} finally {
			readingLines = false;
		}
	}

	/** How many changes a report shows before "Show all". */
	const CHANGES_SHOWN = 6;

	type Reading = RereadIngredientLinesOutput['changed'][number]['before'];

	/** A Reading as one short phrase: "1½ lb · chicken thighs". */
	const measure = (reading: Reading) =>
		[reading?.amount, reading?.unit].filter((part) => part).join(' ');
</script>

<!--
	What reading every line again did (#166, chosen 27 September 2026): the
	counts first, then the lines themselves, a few at a time. A change names
	its recipe, the line as written, and the Reading before and after, so an
	Operator can see a line the reader now reads worse.
-->
{#snippet reading(reading: Reading)}
	{#if measure(reading)}{`${measure(reading)} · `}{/if}
	{#if reading?.target}
		<span class="font-semibold">{reading.target}</span>
	{:else}
		<span class="text-ink-2 italic">{m.operator_lines_no_food()}</span>
	{/if}
{/snippet}

{#snippet report(done: RereadIngredientLinesOutput)}
	{@const recipes = new Set(done.changed.map((change) => change.branch_id)).size}
	<div class="mt-3" role="status">
		<p class="text-body text-ink">
			{done.changed.length === 0 && done.older_versions_changed === 0
				? m.operator_lines_unchanged({ kept: done.kept_by_hand })
				: m.operator_lines_changed({
						count: done.changed.filter((change) => change.on_head).length,
						recipes,
						older: done.older_versions_changed,
						kept: done.kept_by_hand,
					})}
		</p>
		{#if done.emptied_foods.length > 0}
			<p class="mt-2 text-read text-ink-2">
				{done.emptied_foods.length === 1
					? m.operator_lines_emptied_one({ heading: m.operator_unused_heading() })
					: m.operator_lines_emptied({
							count: done.emptied_foods.length,
							heading: m.operator_unused_heading(),
						})}
			</p>
		{/if}
		{#if done.changed.length > 0}
			<ul class="mt-2">
				{#each allChanges ? done.changed : done.changed.slice(0, CHANGES_SHOWN) as change, i (i)}
					<li class="border-b border-rule py-2 last:border-b-0">
						<span class="block text-read text-ink-2">{change.title}</span>
						<span class="block text-body break-words text-ink">{change.line}</span>
						<span class="block text-read text-ink"
							>{@render reading(change.before)} <span class="text-ink-2">→</span>
							{@render reading(change.after)}</span
						>
						{#if !change.on_head}
							<span class="block text-read text-ink-2"
								>{m.operator_lines_older_only({ count: change.older_versions })}</span
							>
						{:else if change.older_versions > 0}
							<span class="block text-read text-ink-2"
								>{change.older_versions === 1
									? m.operator_lines_older_one()
									: m.operator_lines_older({ count: change.older_versions })}</span
							>
						{/if}
					</li>
				{/each}
			</ul>
			{#if !allChanges && done.changed.length > CHANGES_SHOWN}
				<button
					type="button"
					onclick={() => (allChanges = true)}
					class="min-h-12 text-read text-accent underline"
				>
					{m.operator_lines_show_all({ count: done.changed.length })}
				</button>
			{/if}
		{/if}
	</div>
{/snippet}

<OperatorOnly title={m.settings_upkeep()} onallowed={load}>
	{#if acting.said && !asking}
		<p class="mt-4 text-body text-support" role="alert"><Refusal words={acting.said} /></p>
	{/if}

	<Section heading={m.operator_merge_heading()}>
		{#if suggestions.length === 0}
			<div class="rounded-sm bg-ground-2 p-6 text-center">
				<p class="font-display text-list-title font-semibold text-ink">
					{m.operator_merge_empty_title()}
				</p>
				<p class="mt-1 text-body text-ink-2">{m.operator_merge_empty()}</p>
			</div>
		{:else}
			<ul class="overflow-hidden rounded-sm border border-rule bg-card">
				{#each suggestions as suggestion (suggestion.foods[0].id + suggestion.foods[1].id)}
					<li class="border-b border-rule p-3 last:border-b-0">
						<p class="text-body font-medium text-ink">
							{telling(suggestion.foods[0], suggestion.foods[1])} ·
							{telling(suggestion.foods[1], suggestion.foods[0])}
						</p>
						<p class="mt-1 text-read text-ink-2">{because(suggestion.reason)}</p>
						<div class="mt-2 flex flex-wrap gap-2">
							{#each suggestion.foods as keep, index (keep.id)}
								<button
									type="button"
									disabled={acting.working}
									onclick={() => askMerge(keep, suggestion.foods[index === 0 ? 1 : 0])}
									class="min-h-12 rounded-sm border border-accent px-3 text-read font-medium
									text-accent disabled:opacity-60"
								>
									{m.operator_merge_keep({
										food: telling(keep, suggestion.foods[index === 0 ? 1 : 0]),
									})}
								</button>
							{/each}
						</div>
					</li>
				{/each}
			</ul>
		{/if}
	</Section>

	<Section heading={m.operator_unused_heading()}>
		{#if unused.length === 0}
			<p class="text-body text-ink-2">{m.operator_unused_empty()}</p>
		{:else}
			<ul class="overflow-hidden rounded-sm border border-rule bg-card">
				{#each unused.slice(0, SHOWN) as food (food.id)}
					<li class="flex items-center gap-3 border-b border-rule p-3 last:border-b-0">
						<span class="min-w-0 flex-1">
							<span class="block truncate text-body text-ink">{naming(food)}</span>
							<span class="block text-read text-ink-2">{m.operator_unused_none()}</span>
						</span>
						<button
							type="button"
							disabled={acting.working}
							onclick={() => (asking = { kind: 'delete-food', food })}
							class="min-h-12 shrink-0 rounded-sm border border-support px-3 text-read
							font-medium text-support disabled:opacity-60"
						>
							{m.operator_delete()}
						</button>
					</li>
				{/each}
			</ul>
			<p class="mt-2 text-read text-ink-2">
				{unused.length > SHOWN
					? m.operator_unused_capped({ shown: SHOWN, count: unused.length })
					: m.operator_unused_kept({ count: unused.length })}
			</p>
		{/if}
	</Section>

	<Section heading={m.operator_photographs_heading()}>
		<p class="mb-3 text-read text-ink-2">
			{swept
				? m.operator_photographs_swept({ swept: swept.swept, kept: swept.referenced })
				: m.operator_photographs_blurb()}
		</p>
		<button
			type="button"
			disabled={acting.working}
			onclick={() => (asking = { kind: 'sweep' })}
			class="min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium
			text-accent disabled:opacity-60"
		>
			{m.operator_photographs_sweep()}
		</button>
	</Section>

	<Section heading={m.operator_lines_heading()}>
		<p class="mb-3 text-read text-ink-2">{m.operator_lines_blurb()}</p>
		<button
			type="button"
			disabled={readingLines}
			onclick={readLines}
			class="min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium
			text-accent disabled:opacity-60"
		>
			{readingLines ? m.operator_working() : m.operator_lines_run()}
		</button>
		{#if reread}
			{@render report(reread)}
		{/if}
	</Section>
</OperatorOnly>

{#if asking?.kind === 'merge'}
	{@const preview = asking.preview}
	<Confirm
		title={m.operator_merge_title({
			absorbed: naming(preview.absorbed),
			survivor: naming(preview.survivor),
		})}
		count={preview.ingredient_lines}
		counting={m.operator_merge_counting({
			absorbed: naming(preview.absorbed),
			survivor: naming(preview.survivor),
		})}
		consequence={m.operator_merge_consequence({
			absorbed: naming(preview.absorbed),
			survivor: naming(preview.survivor),
		})}
		act={m.operator_merge_do()}
		busy={acting.working}
		failed={acting.said}
		run={() => merge(preview)}
		cancel={close}
	>
		{#if preview.cup_weight_conflict}
			<p class="text-read text-ink-2">{m.operator_merge_cup_weight()}</p>
		{/if}
	</Confirm>
{:else if asking?.kind === 'sweep'}
	<Confirm
		title={m.operator_sweep_title()}
		consequence={m.operator_sweep_consequence()}
		act={m.operator_photographs_sweep()}
		busy={acting.working}
		failed={acting.said}
		run={sweep}
		cancel={close}
	/>
{:else if asking?.kind === 'delete-food'}
	{@const food = asking.food}
	<Confirm
		title={m.operator_food_title({ food: naming(food) })}
		consequence={m.operator_food_consequence()}
		act={m.operator_delete()}
		busy={acting.working}
		failed={acting.said}
		run={() => deleteFood(food)}
		cancel={close}
	/>
{/if}
