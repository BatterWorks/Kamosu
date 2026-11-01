<!--
	Your plan in your calendar (#491, chosen on 1 November 2026 from
	`.dev/prototypes/491-calendar/`, its `none-*`, `made-*`, `read-*` and
	`unread-*` pages): the Calendar Link a phone's calendar app reads the Meal
	Plan with (ADR 0059).

	The link is shown once, the moment it is made, with a way into an Apple
	calendar, a Copy button and a line for Google Calendar. Afterwards the page
	says only that a link is on, when it was made and when a calendar last read
	it, since Kamosu keeps no readable copy of it. The three plain-words points
	stand under it in every state.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { aMoment } from '$lib/dates';
	import { copyText } from '$lib/clipboard';
	import Refusal from '$lib/Refusal.svelte';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import type { GetCalendarLinkOutput } from '$lib/api/catalogue';

	const kamosu = useKamosu();

	/** The live link as Kamosu last said, `null` for none, `undefined` until read. */
	let live = $state<GetCalendarLinkOutput['calendar_link'] | undefined>(undefined);
	/** The link just made, shown this once. */
	let made = $state<string | undefined>(undefined);
	let copied = $state(false);
	let busy = $state(false);
	let failed = $state<string | undefined>(undefined);

	/** An Apple device subscribes from a `webcal:` link in one tap. */
	const apple = /iPhone|iPad|iPod|Macintosh/.test(navigator.userAgent);

	async function read() {
		try {
			live = (await kamosu.getCalendarLink({})).calendar_link;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		}
	}

	onMount(() => void read());

	async function run(work: () => Promise<void>) {
		if (busy) return;
		busy = true;
		failed = undefined;
		try {
			await work();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			busy = false;
		}
	}

	const make = () =>
		run(async () => {
			const link = await kamosu.makeCalendarLink({});
			made = link.url ?? `${location.origin}${link.path}`;
			copied = false;
			await read();
		});

	const stop = () =>
		run(async () => {
			await kamosu.endCalendarLink({});
			await read();
		});
</script>

<p class="mt-2 text-body">{m.calendar_link_intro()}</p>

{#if made}
	<div class="mt-4 rounded-sm border border-accent bg-card p-3" role="alert">
		<p class="text-body font-semibold text-ink">{m.calendar_link_once()}</p>
		<code class="mt-2 block rounded-sm bg-ground p-2 text-read break-all select-all">{made}</code>
		{#if apple}
			<a
				href={made.replace(/^https?:/, 'webcal:')}
				class="mt-3 flex min-h-12 items-center justify-center rounded-sm bg-accent px-4 font-semibold text-on-accent"
			>
				{m.calendar_link_add_apple()}
			</a>
		{/if}
		<button
			type="button"
			class={[
				'mt-3 min-h-12 w-full rounded-sm px-4 font-semibold',
				apple ? 'border border-accent bg-card text-accent' : 'bg-accent text-on-accent',
			]}
			onclick={async () => (copied = await copyText(made ?? ''))}
		>
			{copied ? m.calendar_link_copied() : m.calendar_link_copy()}
		</button>
		<p class="mt-3 text-read text-ink-2">
			<span class="font-semibold text-ink">{m.calendar_link_google_label()}</span>
			{m.calendar_link_google_steps()}
		</p>
		<button
			type="button"
			class="mt-3 block text-label text-accent underline"
			onclick={() => (made = undefined)}
		>
			{m.calendar_link_done()}
		</button>
	</div>
{:else if live}
	<div class="mt-4 rounded-sm border border-rule bg-card px-3 py-2">
		<p class="text-body text-ink">{m.calendar_link_on()}</p>
		<p class="text-read text-ink-2">
			{live.last_read_at
				? m.calendar_link_made_read({
						made: aMoment(live.made_at),
						read: aMoment(live.last_read_at),
					})
				: m.calendar_link_made_unread({ made: aMoment(live.made_at) })}
		</p>
	</div>
	<div class="mt-3 flex flex-wrap gap-3">
		<NeedsServer
			label={m.calendar_link_make_new()}
			waiting={m.calendar_link_waits()}
			disabled={busy}
			onclick={() => void make()}
			shapeClass="min-h-12 rounded-sm px-4 font-semibold disabled:opacity-60"
			lookClass="border border-accent bg-card text-accent"
		/>
		<NeedsServer
			label={m.calendar_link_stop()}
			waiting={m.calendar_link_waits()}
			disabled={busy}
			onclick={() => void stop()}
			shapeClass="min-h-12 rounded-sm px-4 font-semibold disabled:opacity-60"
			lookClass="border border-rule bg-card text-ink"
		/>
	</div>
	<p class="mt-2 text-read text-ink-2">{m.calendar_link_new_stops_old()}</p>
{/if}

{#if failed}
	<p class="mt-2 text-read text-support" role="alert"><Refusal words={failed} /></p>
{/if}

<div class="mt-4 grid gap-2 rounded-sm border border-rule bg-card p-3 text-read text-ink-2">
	<p>
		<span class="font-semibold text-ink">{m.calendar_link_who_label()}</span>
		{m.calendar_link_who()}
	</p>
	<p>
		<span class="font-semibold text-ink">{m.calendar_link_when_label()}</span>
		{m.calendar_link_when()}
	</p>
	<p>
		<span class="font-semibold text-ink">{m.calendar_link_google_needs_label()}</span>
		{m.calendar_link_google_needs()}
	</p>
</div>

{#if live === null && !made}
	<NeedsServer
		label={busy ? m.calendar_link_making() : m.calendar_link_make()}
		waiting={m.calendar_link_waits()}
		disabled={busy}
		onclick={() => void make()}
		shapeClass="mt-6 min-h-12 w-full rounded-sm px-4 font-semibold disabled:opacity-60"
		lookClass="bg-accent text-on-accent"
	/>
{/if}
