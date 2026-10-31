<!--
	Writing on the plan (#487): a few words on a day, or on any day of a week,
	"leftovers", "eating out", kept as written and never read.

	**Chosen on 31 October 2026**, option A of the prototype recorded on #487:
	opened from the + on a day or on the week's list, so the place is known.
	The words, then "or" and a way to choose a recipe instead, then "Put it on
	<day>", which waits until something is written.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { Where } from '$lib/plan-days';
	import { whereHeading, whereName } from '$lib/plan-where';

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
		<p class="text-label text-support-2 uppercase">{m.plan_add_label()}</p>
		<h3 id="{uid}-title" class="mt-1 font-display text-line font-semibold">
			{whereHeading(where)}
		</h3>
		<label class="mt-4 block text-label text-ink-2 uppercase" for="{uid}-words"
			>{m.plan_write_something()}</label
		>
		<input
			id="{uid}-words"
			class="mt-2 block min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body text-ink"
			placeholder={m.plan_words_placeholder()}
			bind:value={words}
			data-sheet-focus
		/>
		<p class="mt-2 text-read text-ink-2">{m.plan_words_kept_short()}</p>
		<p class="mt-3 text-center text-read text-ink-2">{m.plan_or()}</p>
		<a
			href="/recipes"
			class="mt-2 flex min-h-12 items-center justify-center rounded-sm border border-accent bg-card px-3 font-semibold text-accent"
			>{m.plan_choose_recipe()}</a
		>
		{#if failed}
			<p class="mt-3 text-read text-support" role="alert">{m.plan_write_failed()}</p>
		{/if}
	</div>
	<div class="grid gap-2 border-t border-rule px-gutter py-3">
		<button
			type="button"
			class="min-h-12 rounded-sm bg-accent px-4 font-display font-semibold text-on-accent disabled:opacity-40"
			disabled={!words.trim() || working}
			onclick={() => void put()}
		>
			{m.plan_put_it_on({ where: whereName(where) })}
		</button>
		<button type="button" class="min-h-12 text-read text-accent underline" onclick={onClose}>
			{m.plan_cancel()}
		</button>
	</div>
</SheetFrame>
