<!--
	Share from your phone (#452, option B of 24 October 2026): a page of its
	own on the Settings menu, which sets a phone up to add recipes from any
	app's Share menu (ADR 0055). The prototype is
	`.dev/prototypes/452-share/setup-b-start-phone.html` and its states.

	**On an iPhone** the way in is an Apple Shortcut, so the page lays the
	setup out as three numbered steps from the start: choose what the agent
	reads, make the setup code, install the Shortcut. The code is an Add-only
	key and this phone's address in one string, shown once inside step 2 as a
	new key is shown, and never again. Without a Runner the first step says
	what works without an agent instead of asking.

	**On an Android phone** Kamosu itself is in the Share menu once installed,
	signed in as it is, so there is nothing to set up: the page says so and
	asks the agent question for this phone's Session.

	Anything else, a computer, is shown the iPhone's steps: a code made there
	reaches a phone through the clipboard the two share.
-->
<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Refusal from '$lib/Refusal.svelte';
	import OneTimeSecret from '$lib/OneTimeSecret.svelte';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import { thisDevice } from '$lib/offline/device.svelte';
	import { SHORTCUT_URL, phoneName, setupCode, whichPhone } from '$lib/sharing';

	const kamosu = useKamosu();
	const uid = $props.id();

	const phone = whichPhone(navigator.userAgent, thisDevice().apple);
	const device = phoneName(navigator.userAgent);

	/** Whether this instance has a Runner, or nothing until it has been read. */
	let runner = $state<boolean | null>(null);
	/** Where what is shared goes: everything to the agent, or only what Kamosu cannot read. */
	let everything = $state(false);
	/** The code just made, shown this once. */
	let code = $state<string | undefined>(undefined);
	let making = $state(false);
	let failed = $state<string | undefined>(undefined);

	onMount(() => {
		kamosu
			.runnerStatus({})
			.then((status) => (runner = status.has_runner))
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
			});
		// Android keeps the choice with its Session.
		if (phone === 'android')
			kamosu
				.getAgentChoice({})
				.then((choice) => (everything = choice.everything_to_agent))
				.catch((error: unknown) => {
					if (!(error instanceof OperationError)) throw error;
				});
	});

	async function make() {
		if (making) return;
		making = true;
		failed = undefined;
		try {
			const key = await kamosu.mintAccessKey({
				name: device,
				add_only: true,
				// Asked only where there is a Runner, and kept off where there is none.
				everything_to_agent: runner === true && everything,
			});
			code = setupCode(location.origin, key.secret);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			making = false;
		}
	}

	async function chooseForThisPhone(on: boolean) {
		everything = on;
		failed = undefined;
		try {
			await kamosu.setAgentChoice({ everything_to_agent: on });
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		}
	}
</script>

<!-- In step 1 the step's own title asks, so the legend is for a screen reader alone. -->
{#snippet question(name: string, onchoose: (on: boolean) => void, asks: boolean)}
	<fieldset class="grid gap-2">
		<legend class={asks ? 'mb-1 text-body text-ink' : 'sr-only'}>{m.share_agent_question()}</legend>
		<label class="grid grid-cols-[auto_1fr] items-start gap-x-2 text-body text-ink">
			<input
				class="mt-1"
				type="radio"
				{name}
				checked={!everything}
				onchange={() => onchoose(false)}
			/>
			{m.share_only_unread()}
			<span class="col-start-2 text-read text-ink-2">{m.share_only_unread_note()}</span>
		</label>
		<label class="grid grid-cols-[auto_1fr] items-start gap-x-2 text-body text-ink">
			<input
				class="mt-1"
				type="radio"
				{name}
				checked={everything}
				onchange={() => onchoose(true)}
			/>
			{m.share_everything()}
			<span class="col-start-2 text-read text-ink-2">{m.share_everything_note()}</span>
		</label>
	</fieldset>
{/snippet}

{#snippet step(number: number, title: string, now: boolean, body: Snippet)}
	<li class={['rounded-sm border bg-card p-3', now ? 'border-accent' : 'border-rule']}>
		<h2 class="flex items-baseline gap-2">
			<span
				aria-hidden="true"
				class={[
					'grid h-[26px] w-[26px] shrink-0 place-items-center round-mark border text-read',
					now ? 'border-accent bg-accent text-on-accent' : 'border-rule text-ink-2',
				]}>{number}</span
			>
			<span class="font-display text-line font-semibold">{title}</span>
		</h2>
		<div class="mt-2">{@render body()}</div>
	</li>
{/snippet}

{#snippet noRunner()}
	<p class="text-body">{m.share_no_runner_intro()}</p>
	<ul class="mt-2 list-disc pl-6 text-body">
		<li>{m.share_no_runner_link()}</li>
		<li>{m.share_no_runner_page()}</li>
		<li>{m.share_no_runner_files()}</li>
	</ul>
{/snippet}

{#snippet choose()}
	{@render question(`${uid}-agent`, (on) => (everything = on), false)}
{/snippet}

{#snippet makeCode()}
	{#if code}
		<p class="mb-2 text-read text-ink-2">{m.share_copy_note({ device })}</p>
		<OneTimeSecret
			secret={code}
			once={m.share_code_once()}
			copy={m.share_code_copy()}
			copied={m.share_code_copied()}
			dismiss={m.share_done()}
			ondismiss={() => (code = undefined)}
			onCard
			focus
		/>
	{:else}
		<p class="text-read text-ink-2">{m.share_make_note({ device })}</p>
		<NeedsServer
			label={making ? m.share_making() : m.share_make()}
			waiting={m.share_waits()}
			disabled={making || runner === null}
			onclick={() => void make()}
			shapeClass="mt-3 min-h-12 w-full rounded-sm px-4 font-semibold disabled:opacity-60"
			lookClass="bg-accent text-on-accent"
		/>
	{/if}
	{#if failed}
		<p class="mt-2 text-read text-support" role="alert"><Refusal words={failed} /></p>
	{/if}
{/snippet}

{#snippet install()}
	<p class="text-read text-ink-2">{m.share_install_note()}</p>
	{#if SHORTCUT_URL}
		<a
			href={SHORTCUT_URL}
			class="mt-3 flex min-h-12 items-center justify-center rounded-sm border border-accent bg-card px-4 font-semibold text-accent"
		>
			{m.share_get_shortcut()}
		</a>
	{:else}
		<p class="mt-3 text-read text-ink-2">{m.share_no_shortcut_yet()}</p>
	{/if}
{/snippet}

{#if phone === 'android'}
	<p class="mt-2 text-body">{m.share_android()}</p>
	{#if runner}
		<div class="mt-3">
			{@render question(`${uid}-phone`, (on) => void chooseForThisPhone(on), true)}
		</div>
		<p class="mt-2 text-read text-ink-2">{m.share_android_kept()}</p>
	{:else if runner === false}
		<div class="mt-3">{@render noRunner()}</div>
	{/if}
	{#if failed}
		<p class="mt-2 text-read text-support" role="alert"><Refusal words={failed} /></p>
	{/if}
{:else}
	<p class="mt-2 text-body">{runner === false ? m.share_what_short() : m.share_what()}</p>
	<ol class="mt-4 grid gap-3">
		{#if runner === false}
			{@render step(1, m.share_step_works(), false, noRunner)}
		{:else}
			{@render step(1, m.share_step_choose(), !code, choose)}
		{/if}
		{@render step(2, code ? m.share_step_copy() : m.share_step_make(), !!code, makeCode)}
		{@render step(3, m.share_step_install(), false, install)}
	</ol>
{/if}
