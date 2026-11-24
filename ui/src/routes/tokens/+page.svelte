<!--
	The tokens page (issue #80, moved here by #37).

	Every value on it is read out of the stylesheet the browser actually loaded,
	with `getComputedStyle`. That is the whole trick: the page cannot drift from
	what Kamosu looks like, because it is not a copy of the tokens — it is the
	tokens, read back. It was rendered from Rust while there was no app to put it
	in; now that there is, it is an ordinary route reading the same values the
	same way.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import Screen from '#lib/shell/Screen.svelte';
	import Section from '#lib/shell/Section.svelte';
	import ListRow from '#lib/ListRow.svelte';
	import NoticeCard from '#lib/NoticeCard.svelte';
	import {
		COLOUR_ORDER,
		RADIUS_ORDER,
		readToken,
		readTypeScale,
		readSpacing,
	} from '#lib/tokens.js';

	// Read once the document exists: this is what the browser resolved, fonts,
	// cascade and all.
	let colours = $state<{ name: string; value: string }[]>([]);
	let type = $state<ReturnType<typeof readTypeScale>>([]);
	let spacing = $state<{ name: string; value: string }[]>([]);
	let radii = $state<{ name: string; value: string }[]>([]);
	let layout = $state<{ name: string; value: string }[]>([]);

	$effect(() => {
		colours = COLOUR_ORDER.map((name) => ({ name, value: readToken(name) }));
		radii = RADIUS_ORDER.map((name) => ({ name, value: readToken(name) }));
		layout = [
			'--tile-w',
			'--hero-h',
			'--rule-w',
			'--noren-h',
			'--photo-thumb',
			'--row-picture',
		].map((name) => ({
			name,
			value: readToken(name),
		}));
		type = readTypeScale();
		spacing = readSpacing();
	});
</script>

<Screen title={m.tokens_title()} blurb={m.tokens_blurb()}>
	<Section heading={m.tokens_colour()}>
		<ul class="grid grid-cols-2 gap-2 wide:grid-cols-3">
			{#each colours as colour (colour.name)}
				<li class="overflow-hidden rounded-sm border border-rule bg-card">
					<div class="h-12" style="background-color: var({colour.name})"></div>
					<div class="p-2">
						<div class="text-read font-medium">{colour.name}</div>
						<div class="text-read text-ink-2">{colour.value}</div>
					</div>
				</li>
			{/each}
		</ul>
	</Section>

	<!--
		The shared pieces (#632), each in every state it has, named by the class
		or component a screen uses. The one place more than two main buttons
		stand together: these are specimens, not a screen's acts, and
		`data-specimens` keeps them out of the count every screen test ends on.
	-->
	<Section heading={m.tokens_pieces()} data-specimens>
		<div class="grid gap-2">
			<button type="button" class="main-button w-full">main-button</button>
			<div><button type="button" class="main-button">main-button</button></div>
			<button type="button" class="main-button w-full" disabled>main-button · disabled</button>
			<button type="button" class="main-button w-full" disabled aria-busy="true">
				main-button · aria-busy
			</button>
			<button type="button" class="second-button w-full">second-button</button>
			<button type="button" class="second-button w-full" disabled>second-button · disabled</button>
			<button type="button" class="second-button w-full" disabled aria-busy="true">
				second-button · aria-busy
			</button>
			<button type="button" class="second-button w-full loses">second-button loses</button>
			<!-- A few short choices of one setting, the chosen one filled (#653). -->
			<div class="segmented" role="group" aria-label="segmented">
				<button type="button" aria-pressed="true">segmented</button>
				<button type="button" aria-pressed="false">not chosen</button>
				<button type="button" aria-pressed="false">a third</button>
			</div>
			<button type="button" class="second-button w-full" aria-pressed="true">
				second-button · aria-pressed
			</button>
			<div class="flex gap-3">
				<button type="button" class="quiet-button text-accent">quiet-button</button>
				<button type="button" class="quiet-button text-support">quiet-button</button>
			</div>
		</div>

		<ul class="mt-6 list-rows">
			<li><ListRow href="#tokens" title="ListRow" line="line" /></li>
			<li><ListRow href="#tokens" title="ListRow" /></li>
		</ul>
		<ul class="mt-4 list-rows">
			<li>
				<ListRow href="#tokens" title="ListRow · an empty picture" line="line" picture={null} />
			</li>
			<li>
				<ListRow
					onclick={() => {}}
					title="ListRow · onclick · a title long enough to be cut after its second line, which is where a row stops"
					picture={null}
				/>
			</li>
		</ul>

		<div class="mt-6 grid gap-4">
			<NoticeCard title="NoticeCard · plain" actions={[{ label: 'second-button', act: () => {} }]}>
				<p>tone="plain"</p>
			</NoticeCard>
			<NoticeCard
				title="NoticeCard · waiting"
				tone="waiting"
				actions={[
					{ label: 'main-button', act: () => {}, main: true },
					{ label: 'second-button', act: () => {} },
				]}
			>
				<p>tone="waiting"</p>
			</NoticeCard>
			<NoticeCard title="NoticeCard · warning" tone="warning">
				<p>tone="warning"</p>
			</NoticeCard>
		</div>
	</Section>

	<Section heading={m.tokens_type()}>
		{#each type as step (step.name)}
			<div class="mb-4">
				<p
					style="font-size: var({step.name}); line-height: {step.lineHeight ||
						'normal'}; letter-spacing: {step.letterSpacing || 'normal'}"
				>
					{m.tokens_specimen()}
				</p>
				<p class="text-read text-ink-2">
					{step.name} · {step.size}{step.lineHeight
						? ` / ${step.lineHeight}`
						: ''}{step.letterSpacing ? ` · ${step.letterSpacing}` : ''}
				</p>
			</div>
		{/each}
		<p class="mb-2 font-display font-semibold">
			Zen Old Mincho sets what a person wrote — steps, titles, covers.
		</p>
		<p>Zen Kaku Gothic New sets the interface.</p>
	</Section>

	<Section heading={m.tokens_spacing()}>
		{#each spacing as step (step.name)}
			<div class="mb-3">
				<div class="text-read text-ink-2">{step.name} · {step.value}</div>
				<div class="mt-1 h-4 bg-accent" style="width: var({step.name})"></div>
			</div>
		{/each}
	</Section>

	<Section heading={m.tokens_shape()}>
		<div class="mb-4 flex items-end gap-4">
			{#each radii as radius (radius.name)}
				<div class="text-center">
					<div
						class="h-12 w-12 border-2 border-accent bg-card"
						style="border-radius: var({radius.name})"
					></div>
					<div class="mt-1 text-read text-ink-2">
						{radius.name.replace('--radius-', '')}<br />{radius.value}
					</div>
				</div>
			{/each}
		</div>
		<ul class="text-read text-ink-2">
			{#each layout as constant (constant.name)}
				<li>{constant.name} · {constant.value}</li>
			{/each}
		</ul>
	</Section>
</Screen>
