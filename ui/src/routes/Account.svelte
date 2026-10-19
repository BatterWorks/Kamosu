<!--
	Becoming, or returning as, a Person: setup, login, an Invite and a recovery
	link, all four wearing the same form (#38).

	It is not a route of its own. `/` is **Home** once you are signed in, and
	this is what stands there until you are — so the one address in the tab bar
	is the one address you are ever sent to, whichever side of signing in you
	are on.

	An Invite or a recovery link is the exception: each opens a route of its own
	(`/invite/<secret>`, `/recover/<secret>`), which hands the link down as
	`link` and goes Home once this form has signed in (#126).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { useAuth } from '$lib/auth';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import { thisDeviceName } from '$lib/device-name';
	import { passkeysWorkHere } from '$lib/passkeys';
	import { heardWhetherSignedIn, signingIn } from '$lib/shell/signing-in.svelte';
	import type { Snippet } from 'svelte';

	interface Props {
		/**
		 * The Invite or recovery link this form was opened from, as the whole
		 * path Kamosu minted: `/invite/<secret>` or `/recover/<secret>`. Sent to
		 * the Core exactly so, since the Core strips the prefix itself. Absent
		 * on `/`, where the form is setup or login.
		 */
		link?: `/invite/${string}` | `/recover/${string}`;
		/**
		 * Told to the route above when this form has just succeeded, so it can
		 * ask Kamosu again and swap to Home. The route never watches a cookie or
		 * keeps a second idea of who is signed in: the only thing that ever says
		 * so is an Operation answering rather than refusing.
		 */
		onSignedIn?: () => void;
		/**
		 * A login asked for by another page rather than by `/`: the page a Share
		 * Link's *Import this recipe* opens (#170, choice 2). It
		 * brings its own heading, its own words on the button, and what it
		 * offers above and below the form. Worn only by a login: an instance
		 * not yet set up asks for its first Person whoever sent you.
		 */
		framing?: {
			title: string;
			blurb: string;
			submit: string;
			before: Snippet;
			after: Snippet;
		};
	}

	let { link, onSignedIn, framing: framingAsked }: Props = $props();

	const kamosu = useKamosu();
	const auth = useAuth();

	// The wide layout draws no sidebar beside this form (#194).
	$effect(() => {
		signingIn.showing = true;
		return () => {
			signingIn.showing = false;
		};
	});

	let setupComplete = $state<boolean | undefined>(undefined);
	/** The shortest password Kamosu accepts where one is set, as it says (#138). */
	let passwordMinimum = $state(0);
	/** Where a passkey signs in, if this instance offers passkeys at all (#370). */
	let passkeyOrigin = $state<string | null>(null);
	/**
	 * Whether this screen has given up learning the instance's state. Three
	 * states, not two: still asking, answered, and asking failed. Collapsing the
	 * first and the last is what left a browser reading "Loading Kamosu…"
	 * forever with no message and no button (#91).
	 */
	let stalled = $state(false);
	let name = $state('');
	let password = $state('');
	/** The password typed a second time, wherever one is set, so a slip is caught here. */
	let passwordAgain = $state('');
	let failed = $state<string | undefined>(undefined);
	/**
	 * Seconds until this name may be tried again, counted down in the line
	 * where a refusal shows, which it replaces until it reaches zero (#138).
	 */
	let waiting = $state(0);
	let busy = $state(false);
	const invite = $derived(link?.startsWith('/invite/') ? link : undefined);
	const recovery = $derived(link?.startsWith('/recover/') ? link : undefined);
	const mode = $derived(
		invite ? 'invite' : recovery ? 'recover' : setupComplete ? 'login' : 'first-person',
	);
	const framing = $derived(mode === 'login' ? framingAsked : undefined);
	/** What this form says, chosen once for the four forms and a framed login. */
	const words = $derived.by(() => {
		if (framing) return framing;
		switch (mode) {
			case 'invite':
				return {
					title: m.account_invite_title(),
					blurb: m.account_invite_blurb(),
					submit: m.account_invite_submit(),
				};
			case 'recover':
				return {
					title: m.account_recover_title(),
					blurb: m.account_recover_blurb(),
					submit: m.account_recover_submit(),
				};
			case 'login':
				return { title: m.account_login_title(), blurb: undefined, submit: m.account_login() };
			default:
				return {
					title: m.account_setup_title(),
					blurb: m.account_setup_blurb(),
					submit: m.account_create(),
				};
		}
	});
	/** Every form but login sets a password, and says how long it must be. */
	const settingAPassword = $derived(mode !== 'login');
	/**
	 * A passkey is offered at login, and only at the address it signs in at:
	 * Kamosu opened at its LAN address, or with no https:// name, keeps the
	 * form as it was (#370, choice A).
	 */
	const offerPasskey = $derived(mode === 'login' && passkeysWorkHere(passkeyOrigin));
	// A challenge waits for the press, so the press opens the device at once.
	$effect(() => {
		if (offerPasskey) auth.readyPasskey();
	});
	/** Why the last passkey did not sign in, said under its button. */
	let passkeyRefused = $state<string | undefined>(undefined);

	/** Which ask is the current one, so a slow answer to an abandoned one is dropped. */
	let latestAsk = 0;

	/**
	 * Learn whether this instance has been set up — the one question this screen
	 * asks before it can know which form to wear.
	 *
	 * Asked twice at most, and only one refusal is worth asking twice for. A
	 * browser whose Session has ended still holds its Session cookie,
	 * and holding it is exactly what makes Kamosu refuse even this Public
	 * Operation. That refusal is also what expires the cookie (#91), so the
	 * second ask arrives as a stranger and is answered: what a person sees is a
	 * moment of the loading title and then the sign-in form. Any other refusal
	 * the first ask has not fixed, so asking again would only be slower.
	 */
	async function learnInstanceState() {
		const mine = ++latestAsk;
		stalled = false;
		setupComplete = undefined;
		for (let attempt = 0; attempt < 2; attempt++) {
			try {
				const status = await kamosu.instanceStatus();
				if (mine === latestAsk) {
					setupComplete = status.setup_complete;
					passwordMinimum = status.password_minimum;
					passkeyOrigin = status.passkey_origin;
				}
				return;
			} catch (error) {
				// Anything at all, not refusals alone. Whatever went wrong, the
				// one thing this screen must never do again is go quiet (#91).
				if (error instanceof OperationError && error.kind === 'unauthorized' && attempt === 0)
					continue;
				if (mine === latestAsk) stalled = true;
				return;
			}
		}
	}

	$effect(() => {
		void learnInstanceState();
		// Leaving takes the current ask with it, so an answer that arrives after
		// this screen is gone writes nothing.
		return () => {
			latestAsk += 1;
		};
	});

	$effect(() => {
		if (waiting <= 0) return;
		const tick = setTimeout(() => (waiting -= 1), 1000);
		return () => clearTimeout(tick);
	});

	async function submit() {
		failed = undefined;
		waiting = 0;
		// Counted as Kamosu counts: characters, not UTF-16 units, of the password
		// with its outer spaces off. Refused here so it can be said in the
		// reader's Language; Kamosu refuses it too, in English, for a caller
		// that is not this screen.
		if (settingAPassword && [...password.trim()].length < passwordMinimum) {
			failed = m.account_password_too_short({ count: passwordMinimum });
			return;
		}
		if (settingAPassword && password !== passwordAgain) {
			failed = m.account_password_mismatch();
			return;
		}
		busy = true;
		try {
			await auth.authenticate(mode, {
				name: recovery ? undefined : name,
				password,
				// Named for the device it is on, so the Sessions list in Settings
				// can tell your phone from your laptop (#114).
				session_name: thisDeviceName(),
				link: invite ?? recovery,
			});
			if (mode === 'first-person' || mode === 'invite') setupComplete = true;
			heardWhetherSignedIn(true);
			onSignedIn?.();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			if (error.kind === 'busy' && error.retryAfterSeconds) waiting = error.retryAfterSeconds;
			else failed = error.message;
		} finally {
			busy = false;
			// A try refused for its wait was never checked, so what was typed
			// is kept for when the wait is over (#138).
			if (waiting === 0) {
				password = '';
				passwordAgain = '';
			}
		}
	}

	async function signInWithPasskey() {
		failed = undefined;
		passkeyRefused = undefined;
		busy = true;
		try {
			// The device's own prompt was closed: that is an answer, not a fault.
			const outcome = await auth.signInWithPasskey(thisDeviceName());
			// Ready for another press, whatever this one did.
			auth.readyPasskey();
			if (outcome === 'declined') return;
			heardWhetherSignedIn(true);
			onSignedIn?.();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			passkeyRefused =
				error.reason === 'start_again'
					? m.account_passkey_start_again()
					: error.kind === 'unauthorized'
						? m.account_passkey_refused()
						: error.message;
		} finally {
			busy = false;
		}
	}
</script>

{#if stalled}
	<Screen title={m.instance_unreachable()}>
		<button
			type="button"
			class="min-h-12 w-full rounded-sm bg-accent px-4 font-semibold text-on-accent"
			onclick={() => learnInstanceState()}
		>
			{m.account_try_again()}
		</button>
	</Screen>
{:else if setupComplete === undefined}
	<Screen title={m.account_loading()} />
{:else}
	<Screen title={words.title} blurb={words.blurb}>
		{@render framing?.before()}
		<form
			class="grid gap-4"
			onsubmit={(event) => {
				event.preventDefault();
				submit();
			}}
		>
			{#if !recovery}
				<label class="grid gap-1 text-body text-ink">
					{m.account_name()}
					<input
						class="min-h-12 w-full rounded-sm border border-rule bg-card px-3"
						bind:value={name}
						required
						autocomplete="username"
					/>
				</label>
			{/if}
			<div class="grid gap-1">
				<label class="grid gap-1 text-body text-ink">
					{m.account_password()}
					<input
						class="min-h-12 w-full rounded-sm border border-rule bg-card px-3"
						type="password"
						bind:value={password}
						required
						autocomplete={mode === 'login' ? 'current-password' : 'new-password'}
						aria-describedby={settingAPassword ? 'password-minimum' : undefined}
					/>
				</label>
				<!-- Said before anything is typed, where it is needed (#138, choice A). -->
				{#if settingAPassword}
					<p id="password-minimum" class="text-read text-ink-2">
						{m.account_password_minimum({ count: passwordMinimum })}
					</p>
				{/if}
			</div>
			{#if settingAPassword}
				<label class="grid gap-1 text-body text-ink">
					{m.account_password_again()}
					<input
						class="min-h-12 w-full rounded-sm border border-rule bg-card px-3"
						type="password"
						bind:value={passwordAgain}
						required
						autocomplete="new-password"
					/>
				</label>
			{/if}
			{#if waiting > 0}
				<p class="text-body text-accent" role="alert">{m.account_wait({ seconds: waiting })}</p>
			{:else if failed}
				<p class="text-body text-accent" role="alert">{failed}</p>
			{/if}
			<button
				class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60"
				disabled={busy}
			>
				{words.submit}
			</button>
		</form>
		{#if offerPasskey}
			<!-- Beside the password, never instead of it (#370, choice A). -->
			<div class="mt-gutter mb-4 flex items-center gap-3 text-read text-ink-2">
				<span class="flex-1 border-t border-rule"></span>
				{m.account_passkey_or()}
				<span class="flex-1 border-t border-rule"></span>
			</div>
			<button
				type="button"
				class="min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium text-accent disabled:opacity-60"
				disabled={busy}
				onclick={signInWithPasskey}
			>
				{m.account_passkey_sign_in()}
			</button>
			<p class="mt-2 text-read text-ink-2">{m.account_passkey_hint()}</p>
			{#if passkeyRefused}
				<p class="mt-3 text-body text-support" role="alert">{passkeyRefused}</p>
			{/if}
		{/if}
		<!-- For somebody who reached the sign-in form without knowing what
		     Kamosu is (#158): the story, at an address of its own. -->
		{#if framing}
			{@render framing.after()}
		{:else if mode === 'login'}
			<p class="mt-6 text-center">
				<a href="/about" class="text-body text-accent underline underline-offset-4">
					{m.story_what_is()}
				</a>
			</p>
		{/if}
	</Screen>
{/if}
