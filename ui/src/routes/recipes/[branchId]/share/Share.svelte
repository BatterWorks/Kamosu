<!--
	The share screen (#65, ADR 0026, ADR 0018 as amended by #648).

	Two levels of visibility and no third: this recipe is seen by its Kitchen, or
	by anyone holding its link. So there is no scale on this screen and nothing
	to set a point on: a link, or none.

	WHAT A READER OF THE LINK GETS IS SAID BEFORE THE LINK EXISTS, every time,
	in a panel *Make a link…* opens (option B, chosen on #648): the recipe and
	its photos, every earlier Version with its notes and who wrote them, the
	recipes it is made of (#50, ADR 0008), and that ending the link later
	cannot take back a copy already saved. The link is made only from that
	panel. The same words, every time, with no first-time special case (ADR
	0018): what changed on #648 is that they stand in a panel at body size,
	where the audit found the old standing line the palest text on the screen
	(S3).

	A Component travels with its parent as a Passenger, because a recipe that
	cannot tell you how to make its own dough is incomplete. It does not change
	the dough's own Visibility: the dough gets no page and no link of its own,
	and is read only through this one.

	THE LINK IS SHOWN FOR AS LONG AS IT LIVES (#171, ADR 0031 as amended), sent
	through the phone's share menu or copied (`LinkToSend`, #648). Sending the
	recipe to a second person is sending it again, never ending the link and
	minting a new one, which would break every copy already sent. Ending it is
	set apart at the foot, under its own heading, and asks first (U11). A link
	minted before Kamosu kept its address has none to show, and the screen
	says so and how to get one. A live link on an instance with no address,
	which only a Demo can hold, has none either, and the screen says the
	address is not set (#468).
-->
<script lang="ts">
	import { aDate } from '#lib/dates.js';
	import Refusal from '#lib/Refusal.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import Screen from '#lib/shell/Screen.svelte';
	import Section from '#lib/shell/Section.svelte';
	import SheetFrame from '#lib/SheetFrame.svelte';
	import Confirm from '#lib/Confirm.svelte';
	import LinkToSend from '#lib/LinkToSend.svelte';
	import NeedsServer from '#lib/offline/NeedsServer.svelte';
	import { Online, thisDevice, type Device } from '#lib/offline/device.svelte.js';
	import { BUNDLE, shareFile, sharesFiles, useFiles } from '#lib/api/files.js';
	import type { ExportBundleOutput, GetShareLinkOutput } from '#lib/api/catalogue.js';

	interface Props {
		branchId: string;
		/**
		 * Whether this is the app installed on an iPhone or iPad, where the
		 * recipe file goes to the share sheet rather than to an address (#156).
		 * A test hands in its own.
		 */
		device?: Pick<Device, 'installed' | 'apple'>;
	}

	let { branchId, device = thisDevice() }: Props = $props();

	const kamosu = useKamosu();
	const files = useFiles();
	/** Whether the file can be described or handed over at all (#76). */
	const online = new Online();

	let link = $state<GetShareLinkOutput | undefined>(undefined);
	/**
	 * The recipes this one is made of, which a link carries with it (#50). Read
	 * from the recipe rather than from the link, because it is true whether or
	 * not sharing is on — it is what turning it on would mean.
	 */
	let passengers = $state<string[]>([]);
	/**
	 * What the recipe file would hold, read when the screen opens (#65, #66).
	 *
	 * Read up front and not on the tap, because this screen says what an act
	 * means **before** you do it: the panel says what a link carries before it
	 * is made, and a file that announced itself only once it had gone would be
	 * the one thing here that breaks the habit (option C, chosen 20 September
	 * 2026). The photograph that has gone missing is the case that
	 * proves it — worth knowing beforehand, worth nothing after.
	 *
	 * It describes the **file** rather than the recipe, so the Components it
	 * names are the ones actually travelling in the zip.
	 */
	let file = $state<ExportBundleOutput | undefined>(undefined);
	let address = $state('');
	let working = $state(false);
	let failed = $state<string | undefined>(undefined);
	/** The panel that says what goes, open before the link is made (#648). */
	let before = $state(false);
	/** A refusal while the panel is open, said in the panel. */
	let beforeFailed = $state<string | undefined>(undefined);
	/** Ending the link, asked first (#648). */
	let ending = $state(false);
	let endFailed = $state<string | undefined>(undefined);
	/** Where taking the recipe file has got to, in the installed app on Apple (#156). */
	let saving = $state<'idle' | 'getting' | 'ready' | 'failed'>('idle');
	/**
	 * The recipe file fetched and waiting for the tap that shares it (#156).
	 * Set only while `saving` is `ready`, and dropped with the screen.
	 */
	let prepared: File | undefined;
	/** Which recipe the screen is on, so a file fetched for the last one is not kept. */
	let visit = 0;

	$effect(() => {
		let current = true;
		void (async () => {
			try {
				const read = await kamosu.getShareLink({ branch_id: branchId });
				if (current) link = read;

				const recipe = await kamosu.getRecipe({ branch_id: branchId });
				if (!current) return;
				// Named once each, in the order the recipe meets them. A
				// Component this instance does not hold has no name to give and
				// nothing to carry, so it is not promised.
				passengers = [
					...new Set(
						(recipe.versions.at(-1)?.components ?? [])
							.filter((component) => component.held && !component.stopped)
							.map((component) => component.title)
							.filter((title): title is string => Boolean(title)),
					),
				];
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = m.share_failed();
			}
		})();
		return () => {
			current = false;
		};
	});

	/**
	 * What the recipe file would hold, read on its own.
	 *
	 * Apart from the link's own read on purpose, twice over. The file has
	 * nothing to do with the link, so a failure here must not say "the link
	 * could not be changed" — and this read is the one that is *expected* to
	 * fail, because `export_bundle` is answered straight through and never kept
	 * on the phone (`#lib/offline/reads.js`). Off the network it simply does not
	 * arrive, and the block below says so rather than disappearing.
	 */
	$effect(() => {
		let current = true;
		const asked = branchId;
		// A file fetched or ready for the last recipe is that recipe's (#156).
		visit += 1;
		saving = 'idle';
		prepared = undefined;
		void (async () => {
			try {
				const described = await kamosu.exportBundle({ branch_id: asked });
				if (current) file = described;
			} catch {
				// Nothing to say here. Offline the block already says what it
				// is waiting for, and a server that cannot describe the file
				// cannot hand it over either.
				if (current) file = undefined;
			}
		})();
		return () => {
			current = false;
		};
	});

	/** Whether the instance already knows where it is reachable from outside. */
	const knowsAddress = $derived(Boolean(link?.public_address));

	/** Open the panel, once there is an address for the link to carry. */
	function askToMake() {
		if (!knowsAddress && !address.trim()) {
			failed = m.share_address_missing();
			return;
		}
		failed = undefined;
		beforeFailed = undefined;
		before = true;
	}

	async function make() {
		if (working) return;
		const offered = address.trim();
		working = true;
		beforeFailed = undefined;
		try {
			link = await kamosu.shareRecipe(
				knowsAddress ? { branch_id: branchId } : { branch_id: branchId, public_address: offered },
			);
			before = false;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			beforeFailed = error.message;
		} finally {
			working = false;
		}
	}

	async function end() {
		if (working) return;
		working = true;
		endFailed = undefined;
		try {
			link = await kamosu.endShareLink({ branch_id: branchId });
			ending = false;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			endFailed = error.message;
		} finally {
			working = false;
		}
	}

	/** The recipes travelling inside the file besides this one (#50, ADR 0008). */
	const carried = $derived(
		(file?.passengers ?? [])
			.map((passenger) => passenger.title)
			.filter((title): title is string => Boolean(title)),
	);

	/**
	 * Take the file.
	 *
	 * A plain navigation and not a fetch: the server answers the zip under a
	 * Content-Disposition, the Session cookie travels with the request, and the
	 * browser saves it without this screen going anywhere. Nothing is held in
	 * memory, which matters for a recipe whose photographs run to megabytes.
	 *
	 * In the app installed on an iPhone or iPad that address would open in the
	 * app's own window, which has no Share or Save to Files (#156). There the
	 * file is fetched here, and this same button then shares it (`shareIt`;
	 * option A, chosen 27 September 2026), as a Sheet is (#149). Holding
	 * the file until the second tap is the cost of that path.
	 */
	async function save() {
		if (!file) return;
		if (!sharesFiles(device, BUNDLE)) {
			window.location.assign(file.fetch_at);
			return;
		}
		const forVisit = visit;
		saving = 'getting';
		const leftBehind = () => closed || visit !== forVisit;
		try {
			const fetched = await files(file.fetch_at, BUNDLE);
			// Left while it was fetched: nothing is kept for a page not shown.
			if (leftBehind()) return;
			prepared = fetched;
			saving = 'ready';
		} catch {
			if (!leftBehind()) saving = 'failed';
		}
	}

	/** Set once this screen closes, so a file still being fetched is not kept. */
	let closed = false;
	$effect(() => () => {
		closed = true;
	});

	/**
	 * Hand the fetched file to the share sheet, straight from the tap with
	 * nothing awaited first (#149). Closing the share sheet keeps the file for
	 * another try; once it has gone somewhere, the next tap fetches afresh.
	 */
	function shareIt() {
		const held = prepared;
		if (!held) return;
		void shareFile(held).then((outcome) => {
			if (outcome === 'kept' || prepared !== held) return;
			prepared = undefined;
			saving = outcome === 'shared' ? 'idle' : 'failed';
		});
	}
</script>

{#snippet whatGoes()}
	<p class="text-body font-semibold text-ink">{m.share_reader_sees()}</p>
	<ul class="mt-2 list-disc pl-6 text-body text-ink">
		<li>{m.share_sees_recipe()}</li>
		<li>{m.share_sees_versions()}</li>
		{#if passengers.length}
			<li>
				{passengers.length === 1
					? m.share_sees_part_one({ recipes: passengers[0]! })
					: m.share_sees_parts({ recipes: passengers.join(', ') })}
			</li>
		{/if}
	</ul>
	<p class="mt-3 text-body text-ink">{m.share_ending_later()}</p>
{/snippet}

<Screen title={m.share_title()}>
	{#if failed}
		<p class="mt-4 text-read text-support" role="alert"><Refusal words={failed} /></p>
	{/if}

	{#if !link && !failed}
		<!-- Only while there is still something to wait for. A read that failed
		     has already said so above, and a screen that goes on saying
		     "Loading…" underneath it is telling the reader to keep waiting for
		     something that is not coming. -->
		<p class="mt-6 text-body text-ink-2">{m.loading()}</p>
	{:else if link?.shared}
		<p class="mt-6 text-body">{m.share_is_shared()}</p>
		{#if link.shared_by && link.created_at}
			<p class="mt-1 text-read text-ink-2">
				{m.share_made_by({ name: link.shared_by, when: aDate(link.created_at) })}
			</p>
		{/if}

		{#if link.url}
			<div class="mt-4"><LinkToSend url={link.url} /></div>
			<!-- What the friend opens: the page a stranger holding the link sees (S7). -->
			<a href={link.url} target="_blank" rel="noreferrer" class="quiet-button mt-3 text-accent">
				{m.share_see_it()}
			</a>
		{:else if !knowsAddress}
			<!-- A Demo given no address (#468): it refuses ending the link and
			     setting one, so the address is its operator's to give. -->
			<p class="mt-4 border-l-2 border-support-2 py-1 pl-4 text-read text-ink-2">
				{m.share_link_no_address()}
			</p>
		{:else}
			<!-- Minted before Kamosu kept a link's address: it still opens for
			     whoever holds it, but there is nothing here to copy (#171). -->
			<p class="mt-4 border-l-2 border-support-2 py-1 pl-4 text-read text-ink-2">
				{m.share_link_made_before()}
			</p>
		{/if}

		<div class="mt-8">{@render whatGoes()}</div>

		<Section heading={m.share_stop_heading()} class="mt-12">
			<p class="text-body text-ink">{m.share_stop_said()}</p>
			<button
				type="button"
				onclick={() => {
					endFailed = undefined;
					ending = true;
				}}
				class="mt-3 second-button w-full loses"
			>
				{m.share_end_ask()}
			</button>
		</Section>
	{:else if link}
		<p class="mt-6 text-body text-ink">{m.share_lead()}</p>

		<!-- A form, so Enter in the address opens the panel (#203). `novalidate`
		     keeps an address the Core would refuse a matter for the Core, whose
		     sentence says what is wrong with it. -->
		<form
			novalidate
			onsubmit={(event) => {
				event.preventDefault();
				askToMake();
			}}
		>
			{#if !knowsAddress}
				<!-- Asked once, at the first Share Link, and kept in the database,
				     never an environment variable (#65). -->
				<label class="mt-6 block text-body font-semibold text-ink" for="public-address">
					{m.share_address_question()}
				</label>
				<input
					id="public-address"
					type="url"
					bind:value={address}
					placeholder="https://kamosu.example.com"
					class="mt-2 w-full rounded-sm border border-rule bg-card p-3 text-body"
				/>
				<p class="mt-2 text-read text-ink-2">{m.share_address_once()}</p>
			{/if}

			<button type="submit" class="mt-6 main-button w-full">{m.share_link_make()}</button>
		</form>
	{/if}

	<!--
		The recipe file (#65, #66, ADR 0020), described before it is taken.

		Outside the link's own branches on purpose: the file has nothing to do
		with the link and is offered whether sharing is on or off. It is the one
		way a recipe leaves Kamosu whole, for a backup or for moving to another
		instance. Its words say another Kamosu takes it in and that a friend is
		sent the link (#648, the audit's S4): a zip is no way to hand a recipe
		to someone on a phone.

		It waits for the server, since the server is what builds it.
	-->
	{#if file || !online.current}
		<Section heading={m.share_file_heading()} class="mt-12">
			{#if file}
				<p class="text-body text-ink">{file.file_name}</p>
				<p class="mt-1 text-read text-ink-2">
					{m.share_file_holds({
						notes:
							file.notes.length === 1
								? m.share_file_notes_one()
								: m.share_file_notes({ count: file.notes.length }),
						photographs:
							file.photographs === 0
								? m.share_file_photographs_none()
								: file.photographs === 1
									? m.share_file_photographs_one()
									: m.share_file_photographs({ count: file.photographs }),
					})}
				</p>
				{#if carried.length}
					<p class="mt-2 text-read text-support-2">
						{carried.length === 1
							? m.share_file_carries_one({ recipes: carried[0] })
							: m.share_file_carries({ recipes: carried.join(', ') })}
					</p>
				{/if}
				{#if file.missing_photographs.length}
					<p class="mt-2 text-read text-support">
						{file.missing_photographs.length === 1
							? m.share_file_missing_one()
							: m.share_file_missing({ count: file.missing_photographs.length })}
					</p>
				{/if}
			{:else}
				<!-- Offline. `export_bundle` is never kept on the phone, so what
				     the file holds cannot be said here — but the button stays
				     where it is rather than vanishing from under the reader. -->
				<p class="text-read text-ink-2">{m.share_file_waiting()}</p>
			{/if}
			{#if saving === 'ready'}
				<!-- The file is already on the phone, so sharing it waits for
				     nothing (#156): the same button, no longer one that needs
				     the server. -->
				<button type="button" onclick={shareIt} class="mt-3 second-button w-full">
					{m.share_file_share()}
				</button>
			{:else}
				<NeedsServer
					label={saving === 'getting' ? m.share_file_getting() : m.share_file_save()}
					waiting={m.offline_waits_file()}
					onclick={save}
					busy={saving === 'getting'}
					shapeClass="mt-3 second-button w-full"
					lookClass=""
					idleClass=""
				/>
			{/if}
			{#if saving === 'failed'}
				<p class="mt-2 text-read text-support" role="alert">{m.share_file_failed()}</p>
			{/if}
		</Section>
	{/if}
</Screen>

{#if before}
	<!-- What goes, every time, before the link exists (#648, option B). -->
	<SheetFrame
		labelledby="before-title"
		class="overflow-y-auto border-t border-rule bg-ground px-gutter pt-6"
		onclose={() => {
			if (!working) before = false;
		}}
	>
		<h2 id="before-title" class="font-display text-list-title font-semibold text-ink">
			{m.share_before_title()}
		</h2>
		<div class="mt-3">{@render whatGoes()}</div>
		{#if beforeFailed}
			<p class="mt-3 text-body text-support" role="alert"><Refusal words={beforeFailed} /></p>
		{/if}
		<div class="mt-6 mb-4 flex flex-col gap-2">
			<button
				type="button"
				onclick={make}
				disabled={working}
				aria-busy={working ? 'true' : undefined}
				class="main-button w-full"
			>
				{working ? m.share_link_making() : m.share_make_the_link()}
			</button>
			<button
				type="button"
				onclick={() => (before = false)}
				disabled={working}
				class="second-button w-full"
			>
				{m.cookbook_invite_not_now()}
			</button>
		</div>
	</SheetFrame>
{/if}

{#if ending}
	<Confirm
		title={m.share_end_title()}
		consequence={m.share_end_said()}
		act={m.share_end_do()}
		cancelLabel={m.share_end_keep()}
		busy={working}
		failed={endFailed}
		run={end}
		cancel={() => (ending = false)}
	/>
{/if}
