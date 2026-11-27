<!--
	Writing on the plan (#487): a few words on a day, or on any day of a week,
	"leftovers", "eating out", kept as written and never read.

	**Chosen on 31 October 2026**, option A of the prototype recorded on #487:
	opened from the + on a day or on the week's list, so the place is known.
	The words, then "or" and a way to choose a recipe instead, then "Put it on
	<day>", which waits until something is written.

	Its title is the name of the + that opened it, "Add to Tuesday 1
	December", in full ink, where a faint "ADD TO THE PLAN" stood above the
	day (option A of #717's prototype, 27 November 2026).
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { longest } from '#lib/longest.js';
	import SheetFrame from '#lib/SheetFrame.svelte';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import type { Where } from '#lib/plan-days.js';
	import { addTo, whereName } from '#lib/plan-where.js';

	interface Props {
		where: Where;
		onWritten: () => void;
		onClose: () => void;
	}

	let { where, onWritten, onClose }: Props = $props();

	const kamosu = useKamosu();
	const uid = $props.id();
	let words = $state('');
	let working = $state(false);
	let failed = $state(false);

	async function put() {
		if (!words.trim() || working) return;
		working = true;
		try {
			await kamosu.planWords({ words, ...where });
			onWritten();
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		} finally {
			working = false;
		}
	}
</script>

<SheetFrame
	labelledby="{uid}-title"
	class="flex flex-col bg-ground"
	onclose={onClose}
	onconfirm={() => void put()}
>
	<div class="min-h-0 flex-1 overflow-y-auto px-gutter pt-4 pb-4">
		<h3 id="{uid}-title" class="font-display text-title font-semibold">
			{addTo(where)}
		</h3>
		<label class="mt-4 block text-label text-ink-2 uppercase" for="{uid}-words"
			>{m.plan_write_something()}</label
		>
		<input
			id="{uid}-words"
			class="mt-2 block min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body text-ink"
			placeholder={m.plan_words_placeholder()}
			bind:value={words}
			maxlength={longest('plan_words', 'words')}
			data-sheet-focus
		/>
		<p class="mt-2 text-read text-ink-2">{m.plan_words_kept_short()}</p>
		<p class="mt-3 text-center text-read text-ink-2">{m.plan_or()}</p>
		<a href="/recipes" class="mt-2 second-button w-full">{m.plan_choose_recipe()}</a>
		{#if failed}
			<p class="mt-3 text-read text-support" role="alert">{m.plan_write_failed()}</p>
		{/if}
	</div>
	<div class="grid gap-2 border-t border-rule px-gutter py-3">
		<button
			type="button"
			class="main-button w-full"
			disabled={!words.trim() || working}
			onclick={() => void put()}
		>
			{m.plan_put_it_on({ where: whereName(where) })}
		</button>
		<button type="button" class="second-button w-full" onclick={onClose}>
			{m.plan_cancel()}
		</button>
	</div>
</SheetFrame>
