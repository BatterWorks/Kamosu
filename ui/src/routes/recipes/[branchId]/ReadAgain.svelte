<!--
	Asking for Kamosu's AI agent to read this recipe again (#279): option 2,
	"a panel of its own above the acts, which asks in place", as chosen on
	15 October 2026 (the decision is a comment on #279; the pages are
	`2-<state>.phone.html` in `.dev/prototypes/279-read-again/`).

	**It is drawn only where `can_read_recipe_again` says it can be read
	again, or that it needs the file handed over again**, read from the server
	each time (never from the phone, `reads.ts`). Everyone else sees nothing
	here: someone who sees the recipe through a Kitchen, an instance with no
	Runner, a recipe with nothing to read again, one received and never
	changed, and one already read again, which a recipe is once (#279). The
	page draws none of it while an agent's Job stands on the recipe, since the
	card above Cook this says so.

	**It asks in place.** Reading again takes an agent minutes and costs
	whoever runs it, so the first press says so and asks; the second sends.
	Where Kamosu has deleted what the recipe was read from (#270), the press
	asks for the photo or the PDF again, shows it once as the + does (#278),
	or takes the text pasted again.

	Offline, every button says what it waits for, as the other acts do.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { useUpload } from '$lib/api/upload';
	import { OperationError } from '$lib/api/client';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import { Online } from '$lib/offline/device.svelte';
	import { ICON } from '$lib/adding/sources';
	import type { CanReadRecipeAgainOutput } from '$lib/api/catalogue';

	interface Props {
		branchId: string;
		/** Moved when the recipe is read again from the server, to ask again too. */
		asOf: number;
		/** Said with the Job once one is asked for. */
		asked: (jobId: string) => void;
	}

	let { branchId, asOf, asked }: Props = $props();

	const kamosu = useKamosu();
	const upload = useUpload();
	const uid = $props.id();
	const online = new Online();

	let answer = $state<CanReadRecipeAgainOutput | undefined>(undefined);
	$effect(() => {
		void asOf;
		const id = branchId;
		let current = true;
		kamosu
			.canReadRecipeAgain({ branch_id: id })
			.then((read) => {
				if (current) answer = read;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				// Out of reach: offer nothing rather than something it cannot vouch for.
				if (current) answer = undefined;
			});
		return () => {
			current = false;
		};
	});

	/** Where the panel is: at rest, asking, or taking what was read again. */
	type Step = 'rest' | 'asking' | 'choosing' | 'pasting';
	let step = $state<Step>('rest');
	let chosen = $state<File | null>(null);
	let pasted = $state('');
	let sending = $state(false);
	let said = $state<string | undefined>(undefined);

	// Another recipe opened: start again.
	$effect(() => {
		void branchId;
		step = 'rest';
		chosen = null;
		pasted = '';
		said = undefined;
	});

	const offered = $derived(
		answer !== undefined && (answer.can_read_again || answer.reason === 'needs_the_file'),
	);
	const needsTheFile = $derived(answer?.reason === 'needs_the_file');
	const original = $derived(answer?.original ?? null);

	/** The site, as the page names it under the title: no scheme, no www. */
	const site = $derived.by(() => {
		if (!answer?.link) return '';
		try {
			return new URL(answer.link).hostname.replace(/^www\./, '');
		} catch {
			return answer.link;
		}
	});

	const lede = $derived(
		original === 'picture'
			? m.read_again_lede_picture()
			: original === 'pdf'
				? m.read_again_lede_pdf()
				: original === 'pasted_text'
					? m.read_again_lede_pasted_text()
					: m.read_again_lede_link({ site }),
	);

	const gone = $derived(
		original === 'picture'
			? m.read_again_gone_picture()
			: original === 'pdf'
				? m.read_again_gone_pdf()
				: m.read_again_gone_pasted_text(),
	);

	/** The photo as the browser holds it, let go when it is put away. */
	const shown = $derived(chosen && original === 'picture' ? URL.createObjectURL(chosen) : null);
	$effect(() => {
		const url = shown;
		return () => {
			if (url) URL.revokeObjectURL(url);
		};
	});

	let field = $state<HTMLInputElement | undefined>(undefined);

	function press() {
		said = undefined;
		if (!needsTheFile) step = 'asking';
		else if (original === 'pasted_text') step = 'pasting';
		else step = 'choosing';
	}

	function back() {
		if (sending) return;
		step = 'rest';
		chosen = null;
		pasted = '';
		said = undefined;
	}

	async function send() {
		sending = true;
		said = undefined;
		try {
			const handed =
				step === 'pasting' ? { text: pasted } : chosen ? { upload_id: await upload(chosen) } : {};
			const answered = await kamosu.readRecipeAgain({ branch_id: branchId, ...handed });
			step = 'rest';
			chosen = null;
			pasted = '';
			asked(answered.job_id);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			said = error.message;
		} finally {
			sending = false;
		}
	}

	const PRIMARY =
		'min-h-12 w-full rounded-sm px-4 font-display text-body font-semibold disabled:opacity-60';
	const SECOND = 'min-h-12 w-full rounded-sm border border-rule px-4 text-body disabled:opacity-40';
</script>

{#if offered}
	<section
		class="mx-gutter mt-6 rounded-sm border border-rule bg-card px-3 py-3"
		aria-labelledby="{uid}-heading"
	>
		<h2 id="{uid}-heading" class="text-label font-normal text-ink-2 uppercase">
			{m.read_again_heading()}
		</h2>
		<p class="mt-1 text-read text-ink-2">{lede} {m.read_again_keeps()}</p>

		{#if step === 'rest'}
			<NeedsServer
				label={m.read_again_do()}
				waiting={m.read_again_waits()}
				onclick={press}
				shapeClass="mt-3 {SECOND}"
				lookClass="text-accent"
				idleClass="text-ink-2 opacity-55"
			/>
		{:else}
			<div class="mt-3 border-t border-rule pt-1">
				{#if step === 'asking'}
					<p class="mt-2 text-read">{m.read_again_cost()}</p>
				{:else if step === 'choosing' && !chosen}
					<p class="mt-2 text-read">{gone}</p>
					<p class="mt-2 text-read">{m.read_again_keeps()} {m.read_again_cost()}</p>
				{:else if step === 'choosing' && chosen}
					<div class="mt-3">
						{#if shown}
							<img
								src={shown}
								alt={m.send_photo_alt()}
								class="block max-h-[30vh] w-full rounded-sm bg-ground-2 object-contain"
							/>
						{/if}
						<p class="mt-1 text-read break-words text-ink-2">{chosen.name}</p>
					</div>
					<p class="mt-2 text-read">{m.read_again_keeps()} {m.read_again_cost()}</p>
				{:else if step === 'pasting'}
					<p class="mt-2 text-read">{gone}</p>
					<label class="sr-only" for="{uid}-pasted">{m.read_again_paste_label()}</label>
					<textarea
						id="{uid}-pasted"
						rows="6"
						class="mt-3 block w-full rounded-sm border border-rule bg-card p-2 text-body text-ink"
						bind:value={pasted}></textarea>
					<p class="mt-2 text-read">{m.read_again_keeps()} {m.read_again_cost()}</p>
				{/if}

				{#if said}
					<p class="mt-2 text-read text-support" role="alert">{said}</p>
				{/if}

				{#if step === 'asking'}
					<div class="mt-3 grid grid-cols-2 gap-2">
						<NeedsServer
							label={sending ? m.read_again_sending() : m.read_again_do()}
							waiting={m.read_again_waits()}
							disabled={sending}
							onclick={send}
							shapeClass={PRIMARY}
							lookClass="bg-accent text-on-accent"
						/>
						<button type="button" class="{SECOND} text-accent" disabled={sending} onclick={back}
							>{m.read_again_no()}</button
						>
					</div>
				{:else if step === 'choosing' && !chosen}
					<input
						type="file"
						accept={original === 'pdf' ? '.pdf,application/pdf' : 'image/*'}
						class="hidden"
						aria-hidden="true"
						tabindex="-1"
						aria-label={original === 'pdf'
							? m.read_again_choose_pdf()
							: m.read_again_choose_picture()}
						bind:this={field}
						onchange={(event) => {
							const file = event.currentTarget.files?.[0];
							// Cleared at once, so choosing the same file twice is two choices.
							event.currentTarget.value = '';
							if (file) chosen = file;
						}}
					/>
					<div class="mt-3 grid gap-2">
						<!-- Not NeedsServer: the chosen page draws a camera beside the words. -->
						{#if online.current}
							<button
								type="button"
								class="{PRIMARY} flex items-center justify-center gap-2 bg-accent text-on-accent"
								onclick={() => field?.click()}
							>
								{#if original === 'picture'}
									<svg
										viewBox="0 0 24 24"
										aria-hidden="true"
										class="h-6 w-6"
										fill="none"
										stroke="currentColor"
										stroke-width="1.5"
										stroke-linecap="round"
										stroke-linejoin="round"><path d={ICON.picture} /></svg
									>
								{/if}
								{original === 'pdf' ? m.read_again_choose_pdf() : m.read_again_choose_picture()}
							</button>
						{:else}
							<button
								type="button"
								disabled
								class="{PRIMARY} border border-rule text-ink-2 opacity-55"
								>{m.read_again_waits()}</button
							>
						{/if}
						<button type="button" class="{SECOND} text-accent" onclick={back}
							>{m.read_again_back()}</button
						>
					</div>
				{:else}
					<div class="mt-3 grid gap-2">
						<NeedsServer
							label={sending ? m.read_again_sending() : m.read_again_send()}
							waiting={m.read_again_waits()}
							disabled={sending || (step === 'pasting' && pasted.trim() === '')}
							onclick={send}
							shapeClass={PRIMARY}
							lookClass="bg-accent text-on-accent"
						/>
						{#if step === 'choosing'}
							<button
								type="button"
								class="{SECOND} text-accent"
								disabled={sending}
								onclick={() => (chosen = null)}>{m.read_again_choose_another()}</button
							>
						{:else}
							<button type="button" class="{SECOND} text-accent" disabled={sending} onclick={back}
								>{m.read_again_back()}</button
							>
						{/if}
					</div>
				{/if}
			</div>
		{/if}
	</section>
{/if}
