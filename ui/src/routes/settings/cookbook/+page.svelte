<!--
	Your Cookbook (#131, ADR 0041, #469): who can change your recipes. One
	card, since a person writes exactly one Cookbook. The Kitchens, which only
	see the recipes, are a page of their own beside this.

	Inviting is the card's one main button, and the invite it makes is drawn
	as every invite is (`InviteLink`, #648), at the instance's public address
	when one is set. Renaming is a small word by the name. Removing a
	Co-author and leaving each ask first.
-->
<script lang="ts">
	import Refusal from '#lib/Refusal.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import Screen from '#lib/shell/Screen.svelte';
	import Confirm from '#lib/Confirm.svelte';
	import CookbookJoins from '#lib/CookbookJoins.svelte';
	import InviteLink from '#lib/InviteLink.svelte';
	import WaitingInvites from '#lib/WaitingInvites.svelte';
	import { cookbookCalled, cookbookPlainName, cookbookWhose } from '#lib/cookbook.js';
	import type { GetCookbookOutput, GetPersonOutput } from '#lib/api/catalogue.js';

	const kamosu = useKamosu();

	let me = $state<GetPersonOutput | undefined>(undefined);
	let cookbook = $state<GetCookbookOutput | undefined>(undefined);
	let failed = $state<string | undefined>(undefined);
	/**
	 * The link just minted. Held only on this page: the Core keeps a hash, so
	 * once the page is left the link cannot be shown again, only ended.
	 */
	let link = $state<{ inviteId: string; url: string } | undefined>(undefined);
	/** Whether the name is being changed. */
	let renaming = $state(false);
	let asking = $state<
		| { kind: 'leave' }
		| { kind: 'remove'; personId: string; name: string }
		| { kind: 'cancel'; inviteId: string }
		| undefined
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
			renaming = false;
		});

	/** Whether an invite is being made, so a second tap makes no second one. */
	let inviting = $state(false);

	const invite = () =>
		withError(async () => {
			if (inviting) return;
			inviting = true;
			const minted = await kamosu.inviteToCookbook().finally(() => (inviting = false));
			link = {
				inviteId: minted.invite_id,
				// The instance's public address, where one is set (#648); the
				// address this page is read at only where none is.
				url: minted.url ?? `${location.origin}/cookbook-invite/${minted.secret}`,
			};
			await load();
		});

	/** Cancel the invite just made, from its card, which asks first itself. */
	async function cancelMade(inviteId: string) {
		await kamosu.cancelCookbookInvite({ invite_id: inviteId });
		// The list read again first, so the invite just ended never shows in it.
		await load();
		link = undefined;
	}

	async function answer() {
		const asked = asking;
		if (!asked) return;
		busy = true;
		askFailed = undefined;
		try {
			if (asked.kind === 'cancel') {
				await kamosu.cancelCookbookInvite({ invite_id: asked.inviteId });
				await load();
			} else {
				cookbook =
					asked.kind === 'leave'
						? await kamosu.leaveCookbook()
						: await kamosu.removeCookbookAuthor({ person_id: asked.personId });
			}
			asking = undefined;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			askFailed = error.message;
		} finally {
			busy = false;
		}
	}
</script>

<Screen title={m.settings_cookbook()} blurb={m.settings_cookbook_said()}>
	{#if cookbook}
		{@const book = cookbook}
		{@const alone = book.authors.length === 1}
		{@const waiting = book.invites.filter((each) => each.invite_id !== link?.inviteId)}
		{#if failed}
			<p class="mb-4 text-body text-accent" role="alert"><Refusal words={failed} /></p>
		{/if}
		{#if book.joins.length > 0}
			<!-- A join waiting on answers, or one somebody said no to (#135). -->
			<div class="mb-4 grid gap-3">
				<CookbookJoins cookbook={book} onAnswered={(answered) => (cookbook = answered)} />
			</div>
		{/if}
		<section class="mt-4 min-w-0 border border-rule bg-card p-4">
			<div class="flex items-baseline justify-between gap-3">
				<h2 class="min-w-0 font-display text-list-title font-semibold break-words text-ink">
					{cookbookCalled(book)}
				</h2>
				{#if !renaming}
					<button
						type="button"
						class="quiet-button shrink-0 text-accent"
						onclick={() => (renaming = true)}
					>
						{m.kitchen_rename()}
					</button>
				{/if}
			</div>
			{#if renaming}
				<!-- Blank until named: the placeholder is the name it goes by
				     meanwhile, its Co-authors', so clearing the box is how a name
				     of its own is given back. -->
				<form
					class="mt-3"
					onsubmit={(event) => {
						event.preventDefault();
						const input = event.currentTarget.elements.namedItem('name') as HTMLInputElement;
						rename(input.value);
					}}
				>
					<label class="block text-body font-semibold text-ink" for="cookbook-name">
						{m.cookbook_name_label()}
					</label>
					<div class="mt-2 flex gap-2">
						<input
							id="cookbook-name"
							name="name"
							class="min-h-12 min-w-0 flex-1 rounded-sm border border-rule bg-ground px-3 text-body text-ink"
							placeholder={cookbookWhose({ ...book, name: null })}
							value={book.name ?? ''}
						/>
						<button class="second-button shrink-0" type="submit">{m.kitchen_save()}</button>
					</div>
				</form>
			{/if}
			<p class="mt-1 text-read text-ink-2">
				{book.recipe_count === 1
					? m.cookbook_count_one()
					: m.cookbook_count({ count: book.recipe_count })}
			</p>

			{#if !alone}
				<p class="mt-4 text-body font-semibold text-ink">{m.cookbook_written_by()}</p>
				<ul class="mt-2">
					{#each book.authors as author (author.person_id)}
						<li
							class="flex min-h-12 items-center justify-between gap-3 border-b border-rule text-body text-ink"
						>
							<span class="min-w-0 break-words">
								{author.name}
								{#if author.person_id === me?.person_id}
									<span class="ml-1 text-label text-ink-2 uppercase">{m.kitchen_member_you()}</span>
								{/if}
							</span>
							{#if author.person_id !== me?.person_id}
								<button
									type="button"
									class="quiet-button shrink-0 text-support"
									onclick={() => {
										askFailed = undefined;
										asking = { kind: 'remove', personId: author.person_id, name: author.name };
									}}
								>
									{m.kitchen_remove_ask({ name: author.name })}
								</button>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}

			{#if link}
				{@const made = link}
				<InviteLink
					title={m.cookbook_invite_link_title()}
					url={made.url}
					after={m.cookbook_invite_gives()}
					cancel={() => cancelMade(made.inviteId)}
				/>
			{:else}
				<button
					type="button"
					class="mt-4 main-button w-full"
					disabled={inviting}
					aria-busy={inviting ? 'true' : undefined}
					onclick={invite}
				>
					{inviting ? m.share_link_making() : m.cookbook_invite_with()}
				</button>
			{/if}

			<!-- An invite made on an earlier visit cannot be shown again, since
			     only its hash is kept, but it can still be cancelled. -->
			<WaitingInvites
				invites={waiting}
				ask={(inviteId) => {
					askFailed = undefined;
					asking = { kind: 'cancel', inviteId };
				}}
			/>

			{#if !alone}
				<div class="mt-6 border-t border-rule pt-4">
					<button
						type="button"
						class="second-button loses"
						onclick={() => {
							askFailed = undefined;
							asking = { kind: 'leave' };
						}}
					>
						{m.cookbook_leave()}
					</button>
				</div>
			{/if}
		</section>
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
	{:else if asked.kind === 'cancel'}
		<Confirm
			title={m.invite_cancel_title()}
			consequence={m.invite_cancel_said()}
			act={m.invite_cancel_do()}
			cancelLabel={m.invite_cancel_keep()}
			{busy}
			failed={askFailed}
			run={answer}
			cancel={() => (asking = undefined)}
		/>
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
