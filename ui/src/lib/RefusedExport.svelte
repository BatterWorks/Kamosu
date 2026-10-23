<!--
	An export Kamosu turned away whole (#412, option B of 23 October 2026):
	nothing imported and the file not kept. The card says why in the screen's
	own words, since the reason is data, and what to do next. For a file no
	app made, that is the apps Kamosu reads, where to ask for another, and the
	ways one recipe at a time always comes in. Those rows open the + on
	Recipes, whose PDF and photo rows open the phone's picker on the next tap:
	a browser opens a picker only for a tap. The photo row is there only where
	a Runner is, as under the +.

	Drawn under the import place, and on the Report of a refused import.
	The prototype is `.dev/prototypes/412-import-place/b-refused-phone.html`.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { ICON } from '$lib/adding/sources';
	import { ASK_FOR_AN_APP, appNames } from '$lib/other-apps';
	import type { ImportFromAnotherAppOutput } from '$lib/api/catalogue';

	type Refused = NonNullable<ImportFromAnotherAppOutput['refused']>;

	interface Props {
		refused: Refused;
		/** The file's own name, where this screen saw it sent. */
		file?: string;
		/** The card's ground: white on the page, the page's ground inside a card. */
		panel?: string;
	}

	let { refused, file, panel = 'bg-card' }: Props = $props();

	const kamosu = useKamosu();
	let runner = $state(false);

	$effect(() => {
		if (refused.reason !== 'unrecognised') return;
		let current = true;
		kamosu
			.runnerStatus({})
			.then((status) => {
				if (current) runner = status.has_runner;
			})
			.catch((error: unknown) => {
				// Not known, then the photo row is not offered.
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});

	const app = $derived(refused.app ?? '');
	const said = $derived(
		refused.reason === 'untested_shape'
			? [m.import_other_refused_untested({ app }), m.import_other_refused_untested_body({ app })]
			: refused.reason === 'nested_too_deep'
				? [m.import_other_refused_nested(), m.import_other_refused_nested_body()]
				: [
						m.import_other_refused_unrecognised(),
						m.import_other_refused_unrecognised_body({ apps: appNames() }),
					],
	);

	/** The ask's sentence, cut where its link goes, so it is translated whole. */
	const ask = $derived(
		m.import_other_refused_ask({ link: '\u0000' }).split('\u0000') as [string, string],
	);

	const ways = $derived([
		{ label: m.import_other_way_pdf(), href: '/recipes?add' },
		...(runner ? [{ label: m.import_other_way_picture(), href: '/recipes?add' }] : []),
		{ label: m.import_other_way_paste(), href: '/recipes?add=paste' },
	]);
</script>

<div class={['border-l-3 border-support px-3 py-3 break-words', panel]} role="alert">
	{#if file}
		<p class="flex items-center gap-2 text-read text-ink-2">
			<svg
				viewBox="0 0 24 24"
				aria-hidden="true"
				class="h-[18px] w-[18px] shrink-0"
				fill="none"
				stroke="currentColor"
				stroke-width="1.5"
				stroke-linecap="round"
				stroke-linejoin="round"><path d={ICON.file} /></svg
			>
			<span class="min-w-0">{file}</span>
		</p>
	{/if}
	<p class="mt-1 text-body font-semibold">{said[0]}</p>
	<p class="mt-2 text-read">{said[1]}</p>
	{#if refused.reason === 'unrecognised'}
		<p class="mt-2 text-read">
			{ask[0]}<a href={ASK_FOR_AN_APP} class="text-accent underline"
				>{m.import_other_ask_github()}</a
			>{ask[1]}
		</p>
	{/if}
</div>
{#if refused.reason === 'unrecognised'}
	<p class="mt-4 text-read">{m.import_other_one_recipe()}</p>
	{#each ways as way (way.label)}
		<a
			href={way.href}
			class="mt-3 flex min-h-12 items-center justify-between gap-3 rounded-sm border border-rule bg-card px-3 text-body font-medium text-ink"
		>
			{way.label}
			<span aria-hidden="true" class="text-ink-2">›</span>
		</a>
	{/each}
{/if}
