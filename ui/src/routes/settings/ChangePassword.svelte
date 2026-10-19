<!--
	Changing your own password (#370, choice A of 19 October 2026: two tick
	boxes under the fields). The prototype is
	`.dev/prototypes/370-passkeys/pages/password-a.phone.html`.

	The current password first, then the new one twice under the rule every
	new password meets. Signing out the other devices starts ticked, since a
	new password usually means the old one is suspected; ending the Access
	Keys starts unticked, since that stops every agent using one and a key
	never knew the password. This device stays signed in either way, and the
	passkeys are left alone.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { ChangePasswordOutput } from '$lib/api/catalogue';

	interface Props {
		/** How many of this Person's Sessions are signed in elsewhere. */
		otherDevices: number;
		/** How many live Access Keys this Person holds. */
		accessKeys: number;
		onChanged: (ended: ChangePasswordOutput) => void;
		onClose: () => void;
	}

	let { otherDevices, accessKeys, onChanged, onClose }: Props = $props();

	const kamosu = useKamosu();
	const uid = $props.id();

	/** The shortest password Kamosu accepts where one is set, as it says (#138). */
	let minimum = $state(0);
	$effect(() => {
		void kamosu.instanceStatus().then((status) => (minimum = status.password_minimum));
	});

	let current = $state('');
	let next = $state('');
	let again = $state('');
	let signOut = $state(true);
	let endKeys = $state(false);
	let busy = $state(false);
	let wrongCurrent = $state(false);
	let failed = $state<string | undefined>(undefined);

	async function change() {
		wrongCurrent = false;
		failed = undefined;
		// Counted as Kamosu counts it, and said in the reader's Language.
		if ([...next.trim()].length < minimum) {
			failed = m.account_password_too_short({ count: minimum });
			return;
		}
		if (next !== again) {
			failed = m.account_password_mismatch();
			return;
		}
		busy = true;
		try {
			onChanged(
				await kamosu.changePassword({
					current_password: current,
					new_password: next,
					sign_out_other_devices: signOut,
					end_access_keys: endKeys,
				}),
			);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			if (error.reason === 'wrong_password') wrongCurrent = true;
			else if (error.kind === 'busy' && error.retryAfterSeconds)
				failed = m.account_wait({ seconds: error.retryAfterSeconds });
			else failed = error.message;
		} finally {
			busy = false;
		}
	}

	const devicesSaid = $derived(
		otherDevices === 0
			? m.password_sign_out_count_none()
			: otherDevices === 1
				? m.password_sign_out_count_one()
				: m.password_sign_out_count({ count: otherDevices }),
	);
	const keysSaid = $derived(
		accessKeys === 0
			? m.password_end_keys_count_none()
			: accessKeys === 1
				? m.password_end_keys_count_one()
				: m.password_end_keys_count({ count: accessKeys }),
	);
</script>

<SheetFrame
	labelledby="{uid}-title"
	tall={78}
	class="overflow-y-auto border-t border-rule bg-ground px-gutter pt-6"
	onclose={() => {
		if (!busy) onClose();
	}}
>
	<h2 id="{uid}-title" class="font-display text-list-title font-semibold text-ink">
		{m.password_change()}
	</h2>
	<form
		class="mt-4 grid gap-4"
		onsubmit={(event) => {
			event.preventDefault();
			void change();
		}}
	>
		<div class="grid gap-1">
			<label class="grid gap-1 text-body text-ink">
				{m.password_current()}
				<input
					class="min-h-12 w-full rounded-sm border border-rule bg-card px-3"
					type="password"
					autocomplete="current-password"
					bind:value={current}
					required
					data-sheet-focus
				/>
			</label>
			{#if wrongCurrent}
				<p class="text-body text-support" role="alert">{m.password_wrong()}</p>
			{/if}
		</div>
		<div class="grid gap-1">
			<label class="grid gap-1 text-body text-ink">
				{m.password_new()}
				<input
					class="min-h-12 w-full rounded-sm border border-rule bg-card px-3"
					type="password"
					autocomplete="new-password"
					bind:value={next}
					required
					aria-describedby="{uid}-minimum"
				/>
			</label>
			<p id="{uid}-minimum" class="text-read text-ink-2">
				{m.password_new_hint({ count: minimum })}
			</p>
		</div>
		<label class="grid gap-1 text-body text-ink">
			{m.password_new_again()}
			<input
				class="min-h-12 w-full rounded-sm border border-rule bg-card px-3"
				type="password"
				autocomplete="new-password"
				bind:value={again}
				required
			/>
		</label>
		<label class="grid grid-cols-[auto_1fr] items-start gap-x-3 text-body text-ink">
			<input class="mt-1 size-[22px] accent-accent" type="checkbox" bind:checked={signOut} />
			{m.password_sign_out()}
			<span class="col-start-2 text-read text-ink-2">{devicesSaid}</span>
		</label>
		<label class="grid grid-cols-[auto_1fr] items-start gap-x-3 text-body text-ink">
			<input class="mt-1 size-[22px] accent-accent" type="checkbox" bind:checked={endKeys} />
			{m.password_end_keys()}
			<span class="col-start-2 text-read text-ink-2">{keysSaid}</span>
		</label>
		{#if failed}
			<p class="text-body text-support" role="alert">{failed}</p>
		{/if}
		<div class="mt-2 mb-4 flex flex-col gap-2">
			<button
				class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60"
				disabled={busy}
			>
				{m.password_do()}
			</button>
			<button
				type="button"
				class="min-h-12 rounded-sm px-4 text-body font-medium text-accent"
				disabled={busy}
				onclick={onClose}
			>
				{m.password_keep()}
			</button>
		</div>
		<p class="mb-4 text-read text-ink-2">{m.password_passkeys_stay()}</p>
	</form>
</SheetFrame>
