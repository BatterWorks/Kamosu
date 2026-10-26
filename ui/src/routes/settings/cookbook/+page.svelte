<!--
	Your Cookbook (#131, ADR 0041, #469): who can change your recipes. One
	card, since a person writes exactly one Cookbook. The Kitchens, which only
	see the recipes, are a page of their own beside this.
-->
<script lang="ts">
	import Refusal from '$lib/Refusal.svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import Confirm from '$lib/Confirm.svelte';
	import CookbookJoins from '$lib/CookbookJoins.svelte';
	import { copyText } from '$lib/clipboard';
	import { cookbookPlainName, cookbookWhose } from '$lib/cookbook';
	import type { GetCookbookOutput, GetPersonOutput } from '$lib/api/catalogue';

	const kamosu = useKamosu();

	let me = $state<GetPersonOutput | undefined>(undefined);
	let cookbook = $state<GetCookbookOutput | undefined>(undefined);
	let failed = $state<string | undefined>(undefined);
	/**
	 * The link just minted. Held only on this page: the Core keeps a hash, so
	 * once the page is left the link cannot be shown again, only ended.
	 */
	let link = $state<{ inviteId: string; url: string } | undefined>(undefined);
	let copied = $state(false);
	let asking = $state<
		{ kind: 'leave' } | { kind: 'remove'; personId: string; name: string } | undefined
	>(undefined);
	let busy = $state(false);
	let askFailed = $state<string | undefined>(undefined);

	async function load() {
		try {
			const [person, book] = await Promise.all([kamosu.getPerson(), kamosu.getCookbook()]);
			me = person;
			cookbook = book;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			cookbook = undefined;
		}
	}

	$effect(() => {
		load();
	});

	async function withError(action: () => Promise<void>) {
		failed = undefined;
		try {
			await action();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		}
	}

	const rename = (name: string) =>
		withError(async () => {
			cookbook = await kamosu.renameCookbook({ name: name.trim() || null });
		});

	const invite = () =>
		withError(async () => {
			const minted = await kamosu.inviteToCookbook();
			copied = false;
			link = {
				inviteId: minted.invite_id,
				url: `${location.origin}/cookbook-invite/${minted.secret}`,
			};
			await load();
		});

	const cancelInvite = (inviteId: string) =>
		withError(async () => {
			await kamosu.cancelCookbookInvite({ invite_id: inviteId });
			if (link?.inviteId === inviteId) link = undefined;
			await load();
		});

	async function copyLink() {
		if (!link) return;
		// A browser that will not copy still shows the link to select by hand.
		copied = await copyText(link.url);
	}

	async function answer() {
		const asked = asking;
		if (!asked) return;
		busy = true;
		askFailed = undefined;
		try {
			cookbook =
				asked.kind === 'leave'
					? await kamosu.leaveCookbook()
					: await kamosu.removeCookbookAuthor({ person_id: asked.personId });
			asking = undefined;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			askFailed = error.message;
		} finally {
			busy = false;
		}
	}
</script>

<Screen title={m.settings_cookbook()}>
	{#if cookbook}
		{@const book = cookbook}
		<p class="mb-3 text-read text-ink-2">{m.settings_cookbook_said()}</p>
		{#if failed}
			<p class="mb-4 text-body text-accent" role="alert"><Refusal words={failed} /></p>
		{/if}
		{#if book.joins.length > 0}
			<!-- A join waiting on answers, or one somebody said no to (#135). -->
			<div class="mb-4 grid gap-3">
				<CookbookJoins cookbook={book} onAnswered={(answered) => (cookbook = answered)} />
			</div>
		{/if}
		<div class="min-w-0 rounded-sm border border-t-4 border-rule border-t-accent bg-card p-3">
			<!-- Blank until named: the placeholder is the name it goes by meanwhile,
			     its Co-authors', so clearing the box is how a name of its own is
			     given back. -->
			<form
				class="flex items-center gap-2"
				onsubmit={(event) => {
					event.preventDefault();
					const input = event.currentTarget.elements.namedItem('name') as HTMLInputElement;
					rename(input.value);
				}}
			>
				<label class="sr-only" for="cookbook-name">{m.cookbook_name_label()}</label>
				<input
					id="cookbook-name"
					name="name"
					class="min-h-12 min-w-0 flex-1 rounded-sm border border-rule bg-ground px-2 font-semibold text-ink"
					placeholder={cookbookWhose({ ...book, name: null })}
					value={book.name ?? ''}
				/>
				<button class="quiet-button shrink-0 text-accent" type="submit">
					{m.kitchen_save()}
				</button>
			</form>

			<h3 class="mt-3 mb-1 text-label text-ink-2 uppercase">{m.cookbook_written_by()}</h3>
			<ul class="grid gap-1">
				{#each book.authors as author (author.person_id)}
					<li class="flex items-center justify-between gap-3 text-body text-ink">
						<span>
							{author.name}
							{#if author.person_id === me?.person_id}
								<span class="ml-1 text-label text-ink-2 uppercase">{m.kitchen_member_you()}</span>
							{/if}
						</span>
						{#if author.person_id !== me?.person_id}
							<button
								type="button"
								class="quiet-button shrink-0 text-accent"
								onclick={() => {
									askFailed = undefined;
									asking = { kind: 'remove', personId: author.person_id, name: author.name };
								}}
							>
								{m.kitchen_remove()}
							</button>
						{/if}
					</li>
				{/each}
			</ul>
			<p class="mt-2 text-read text-ink-2">
				{book.recipe_count === 1
					? m.cookbook_count_one()
					: m.cookbook_count({ count: book.recipe_count })}
			</p>

			{#if link}
				{@const made = link}
				<div class="mt-3 rounded-sm border border-accent bg-ground p-3" role="status">
					<p class="text-read text-ink-2">{m.cookbook_invite_said()}</p>
					<code
						class="mt-2 block rounded-sm border border-dashed border-rule bg-card p-2 text-read break-all"
						>{made.url}</code
					>
					<div class="mt-2 flex flex-wrap items-center gap-4">
						<button type="button" class="quiet-button text-accent" onclick={copyLink}>
							{copied ? m.cookbook_invite_copied() : m.cookbook_invite_copy()}
						</button>
						<button
							type="button"
							class="quiet-button text-support"
							onclick={() => cancelInvite(made.inviteId)}
						>
							{m.cookbook_invite_cancel()}
						</button>
					</div>
				</div>
			{:else}
				<!-- A link minted on an earlier visit cannot be shown again, since
				     only its hash is kept, but it can still be ended. -->
				{#each book.invites as waiting (waiting.invite_id)}
					<div class="mt-3 flex flex-wrap items-center justify-between gap-3">
						<p class="text-read text-ink-2">{m.cookbook_invite_waiting()}</p>
						<button
							type="button"
							class="quiet-button shrink-0 text-support"
							onclick={() => cancelInvite(waiting.invite_id)}
						>
							{m.cookbook_invite_cancel()}
						</button>
					</div>
				{/each}
			{/if}

			<div class="mt-3 flex flex-wrap items-center justify-between gap-3">
				{#if !link}
					<button type="button" class="quiet-button text-accent" onclick={invite}>
						{m.cookbook_invite()}
					</button>
				{/if}
				{#if book.authors.length > 1}
					<button
						type="button"
						class="quiet-button text-support"
						onclick={() => {
							askFailed = undefined;
							asking = { kind: 'leave' };
						}}
					>
						{m.cookbook_leave()}
					</button>
				{/if}
			</div>
		</div>
	{/if}
</Screen>

{#if asking && cookbook}
	{@const asked = asking}
	{@const book = cookbook}
	{#if asked.kind === 'leave'}
		<Confirm
			title={m.cookbook_leave_title({ name: cookbookPlainName(book) })}
			consequence={m.cookbook_leave_said({ count: book.recipe_count })}
			act={m.cookbook_leave_do()}
			cancelLabel={m.cookbook_stay()}
			{busy}
			failed={askFailed}
			run={answer}
			cancel={() => (asking = undefined)}
		>
			<p class="text-read text-ink-2">{m.cookbook_leave_said_links()}</p>
		</Confirm>
	{:else}
		<Confirm
			title={m.cookbook_remove_title({ name: asked.name })}
			consequence={m.cookbook_remove_said({ name: asked.name, count: book.recipe_count })}
			act={m.cookbook_remove_do({ name: asked.name })}
			cancelLabel={m.cookbook_stay()}
			{busy}
			failed={askFailed}
			run={answer}
			cancel={() => (asking = undefined)}
		/>
	{/if}
{/if}
