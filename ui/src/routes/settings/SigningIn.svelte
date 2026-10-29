<!--
	Signing in (#469): everything about getting in, on one page. Your
	password, your passkeys (#370) and the devices signed in (ADR 0031),
	since people look for "change my password" next to those two.

	It is `/settings/signing-in`, and also what the wide layout opens beside
	the menu at `/settings`.
-->
<script lang="ts">
	import { aDate, aMoment } from '$lib/dates';
	import Refusal from '$lib/Refusal.svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import { Online } from '$lib/offline/device.svelte';
	import Passkeys from './Passkeys.svelte';
	import ChangePassword from './ChangePassword.svelte';
	import SignOut from './SignOut.svelte';
	import type { ChangePasswordOutput, ListSessionsOutput } from '$lib/api/catalogue';

	const kamosu = useKamosu();

	type Session = ListSessionsOutput['sessions'][number];

	let signedIn = $state(false);
	let sessions = $state<Session[]>([]);
	/** How many Access Keys there are, which changing the password may end. */
	let accessKeys = $state(0);

	async function load() {
		try {
			const [sessionsAnswer, keysAnswer] = await Promise.all([
				kamosu.listSessions(),
				kamosu.listAccessKeys(),
			]);
			sessions = sessionsAnswer.sessions.filter((session) => !session.revoked);
			accessKeys = keysAnswer.access_keys.filter((key) => !key.revoked).length;
			signedIn = true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			signedIn = false;
		}
	}

	$effect(() => {
		load();
	});

	// --- Password (#370) ---------------------------------------------------

	/** The change-password sheet is open. */
	let changingPassword = $state(false);
	/** What changing the password just did, said under its button until the page is left. */
	let passwordChanged = $state<string | undefined>(undefined);

	function passwordWasChanged(ended: ChangePasswordOutput) {
		changingPassword = false;
		passwordChanged = [
			m.password_done(),
			ended.ended_sessions === 1
				? m.password_done_sessions_one()
				: ended.ended_sessions > 1
					? m.password_done_sessions({ count: ended.ended_sessions })
					: undefined,
			ended.ended_access_keys === 1
				? m.password_done_keys_one()
				: ended.ended_access_keys > 1
					? m.password_done_keys({ count: ended.ended_access_keys })
					: undefined,
		]
			.filter(Boolean)
			.join(' ');
		void load();
	}

	// --- Devices signed in (#114) ------------------------------------------

	function whenLastUsed(lastUsedAt: string | null): string {
		return lastUsedAt ? m.access_last_used({ when: aMoment(lastUsedAt) }) : m.access_never_used();
	}

	/**
	 * The one in your hand first, the rest newest first as the Core answers
	 * them. Ending the one in your hand signs you out here, so it is marked
	 * and its button says so.
	 */
	const sessionsShown = $derived(
		[...sessions].sort((a, b) => Number(b.current) - Number(a.current)),
	);

	/**
	 * What every Session was called before a Session was named for its device.
	 * Kamosu cannot name such a row after the fact, so it says plainly what it
	 * is until its owner renames it.
	 */
	const UNNAMED_SESSION = 'this browser';

	let editing = $state<string | undefined>(undefined);
	let draftName = $state('');
	let renaming = $state(false);
	let renameError = $state<string | undefined>(undefined);

	function startRenaming(session: Session) {
		editing = session.id;
		draftName = session.name === UNNAMED_SESSION ? '' : session.name;
		renameError = undefined;
	}

	async function rename(sessionId: string) {
		renaming = true;
		renameError = undefined;
		try {
			await kamosu.renameSession({ session_id: sessionId, name: draftName });
			editing = undefined;
			await load();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			renameError = error.message;
		} finally {
			renaming = false;
		}
	}

	const online = new Online();

	/** The Session in your hand, while signing out of it asks first (#380). */
	let signingOutOf = $state<string | undefined>(undefined);

	async function end(session: Session) {
		// Ending the Session in your hand is signing out, asked as the foot of
		// the Settings menu asks it: what this device wrote offline goes first.
		if (session.current) {
			signingOutOf = session.id;
			return;
		}
		await kamosu.revokeSession({ session_id: session.id });
		await load();
	}
</script>

<Screen title={m.settings_signing_in()}>
	{#if signedIn}
		<Section heading={m.settings_password()}>
			<button
				type="button"
				class="min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium text-accent"
				onclick={() => {
					passwordChanged = undefined;
					changingPassword = true;
				}}
			>
				{m.password_change()}
			</button>
			{#if passwordChanged}
				<p role="status" class="mt-3 border-l-3 border-accent bg-card px-3 py-2 text-read text-ink">
					{passwordChanged}
				</p>
			{/if}
		</Section>

		<Passkeys />

		<Section heading={m.settings_sessions()}>
			{#if sessions.length === 0}
				<p class="text-body text-ink-2">{m.access_sessions_empty()}</p>
			{:else}
				<ul class="grid gap-2">
					{#each sessionsShown as session (session.id)}
						<li
							class={[
								'rounded-sm border bg-card px-3 py-2',
								session.current ? 'border-accent' : 'border-rule',
							]}
						>
							{#if editing === session.id}
								<form
									class="grid gap-2"
									onsubmit={(event) => {
										event.preventDefault();
										rename(session.id);
									}}
								>
									<label class="grid gap-1 text-read text-ink-2">
										{m.sessions_name_label()}
										<input
											class="min-h-12 w-full rounded-sm border border-rule bg-ground px-3 text-body text-ink"
											bind:value={draftName}
											required
											{@attach (node) => node.focus()}
										/>
									</label>
									{#if renameError}
										<p class="text-body text-accent" role="alert">
											<Refusal words={renameError} />
										</p>
									{/if}
									<div class="flex flex-wrap items-center gap-4">
										<button
											class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60"
											disabled={renaming}
										>
											{m.sessions_name_save()}
										</button>
										<button
											type="button"
											class="quiet-button text-accent"
											onclick={() => (editing = undefined)}
										>
											{m.you_name_keep()}
										</button>
									</div>
								</form>
							{:else}
								<div class="flex items-center justify-between gap-3">
									<div class="min-w-0">
										<p class="text-body break-words text-ink">
											{#if session.name === UNNAMED_SESSION}
												<span class="text-ink-2 italic">{m.sessions_unnamed()}</span>
											{:else}
												{session.name}
											{/if}
											{#if session.current}
												<span class="ml-1 text-label font-semibold text-accent uppercase"
													>{m.sessions_this_device()}</span
												>
											{/if}
										</p>
										<p class="text-read text-ink-2">
											{m.sessions_signed_in({
												when: aDate(session.created_at),
											})} · {whenLastUsed(session.last_used_at)}
										</p>
									</div>
									<div class="flex shrink-0 items-center gap-3">
										{#if session.current && !online.current}
											<!-- Signing out needs the server, as at the foot of the menu (#380). -->
											<button type="button" class="quiet-button text-ink-2 opacity-55" disabled>
												{m.offline_waits_sign_out()}
											</button>
										{:else}
											<button
												type="button"
												class="quiet-button text-accent"
												onclick={() => end(session)}
											>
												{session.current ? m.sessions_sign_out() : m.access_end()}
											</button>
										{/if}
										<button
											type="button"
											class="quiet-button text-accent"
											onclick={() => startRenaming(session)}
										>
											{m.sessions_rename()}
										</button>
									</div>
								</div>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</Section>
	{/if}
</Screen>

{#if changingPassword}
	<ChangePassword
		otherDevices={sessions.filter((session) => !session.current).length}
		{accessKeys}
		onChanged={passwordWasChanged}
		onClose={() => (changingPassword = false)}
	/>
{/if}

{#if signingOutOf}
	<SignOut sessionId={signingOutOf} onclose={() => (signingOutOf = undefined)} />
{/if}
