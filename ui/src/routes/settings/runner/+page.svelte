<!--
	Runner and House Notes (#275, #344, #469): a page of This instance. A
	waiting Job that no agent answers links here (#277).

	Both reads are never kept on the phone (`reads.ts`): a kept answer would
	say a Runner is there after its Key was revoked, or silent while it works.
-->
<script lang="ts">
	import Refusal from '$lib/Refusal.svelte';
	import { howLongAgo } from '$lib/dates';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Section from '$lib/shell/Section.svelte';
	import Confirm from '$lib/Confirm.svelte';
	import RunnerKeySecret from '$lib/operator/RunnerKeySecret.svelte';
	import HouseNotes from '$lib/operator/HouseNotes.svelte';
	import { firstLine, savedLine } from '$lib/operator/house-notes';
	import { Acting } from '$lib/operator/acting.svelte';
	import type {
		GetHouseNotesOutput,
		ListRunnerKeysOutput,
		RunnerStatusOutput,
	} from '$lib/api/catalogue';
	import OperatorOnly from '../OperatorOnly.svelte';

	const kamosu = useKamosu();
	const acting = new Acting();

	type RunnerKey = ListRunnerKeysOutput['runner_keys'][number];

	let runnerKeys = $state<RunnerKey[]>([]);
	let status = $state<RunnerStatusOutput | undefined>(undefined);
	let houseNotes = $state<GetHouseNotesOutput | undefined>(undefined);
	let name = $state('');

	async function loadRunner() {
		const [keys, answer] = await Promise.all([kamosu.listRunnerKeys({}), kamosu.runnerStatus({})]);
		runnerKeys = keys.runner_keys.filter((key) => !key.revoked);
		status = answer;
	}

	async function load() {
		const quietly = async (run: () => Promise<void>) => {
			try {
				await run();
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
			}
		};
		await Promise.all([
			quietly(loadRunner),
			quietly(async () => {
				houseNotes = await kamosu.getHouseNotes({});
			}),
		]);
	}

	/**
	 * Past this a Runner is quiet rather than idle. An idle one asks every ten
	 * seconds, and one reading a recipe asks again when its Job ends, which its
	 * time limit puts at most half an hour away.
	 */
	const QUIET_AFTER = 30 * 60_000;

	const quiet = (at: string) => Date.now() - new Date(at).getTime() > QUIET_AFTER;

	type Asking =
		| { kind: 'revoke'; key: RunnerKey }
		| { kind: 'secret'; name: string; secret: string }
		| { kind: 'house-notes'; notes: GetHouseNotesOutput };

	let asking = $state<Asking | undefined>(undefined);

	function close() {
		asking = undefined;
		acting.said = undefined;
	}

	const mint = () =>
		acting.act(async () => {
			const minted = await kamosu.mintRunnerKey({ name: name.trim() });
			asking = { kind: 'secret', name: minted.name, secret: minted.secret };
			name = '';
		}, loadRunner);

	const revoke = (key: RunnerKey) =>
		acting.act(
			async () => {
				await kamosu.revokeRunnerKey({ runner_key_id: key.id });
			},
			// Closed first: the key is revoked even if the list then fails to
			// come back, and the sheet must not offer to revoke it twice.
			async () => {
				close();
				await loadRunner();
			},
		);
</script>

<OperatorOnly title={m.settings_runner()} onallowed={load}>
	{#if acting.said && !asking}
		<p class="mt-4 text-body text-support" role="alert"><Refusal words={acting.said} /></p>
	{/if}

	<Section heading={m.operator_runner_heading()}>
		<p class="mb-3 text-read text-ink-2">{m.operator_runner_what()}</p>
		{#if status}
			{@const asked = status.last_asked_at}
			<div class="rounded-sm border border-rule bg-card p-3">
				{#if asked === null}
					<p class="text-body text-ink">{m.operator_runner_never()}</p>
					<p class="mt-1 text-read text-ink-2">
						{runnerKeys.length > 0
							? m.operator_runner_never_keyed()
							: m.operator_runner_never_unkeyed()}
					</p>
				{:else if !status.has_runner}
					<!-- One asked once, and every key since is revoked: no Runner
					     can ask now, so the card says what would let one. -->
					<p class="text-body text-ink">{m.operator_runner_quiet({ when: howLongAgo(asked) })}</p>
					<p class="mt-1 text-read text-ink-2">{m.operator_runner_never_unkeyed()}</p>
				{:else if quiet(asked)}
					<p class="text-body text-ink">{m.operator_runner_quiet({ when: howLongAgo(asked) })}</p>
					<p class="mt-1 text-read text-ink-2">{m.operator_runner_quiet_more()}</p>
				{:else}
					<p class="text-body text-ink">{m.operator_runner_asked({ when: howLongAgo(asked) })}</p>
					<p class="mt-1 text-read text-ink-2">{m.operator_runner_asked_more()}</p>
				{/if}
			</div>
		{/if}
		<ul class="mt-3 overflow-hidden rounded-sm border border-rule bg-card">
			{#each runnerKeys as key (key.id)}
				<li class="flex items-center gap-3 border-b border-rule p-3 last:border-b-0">
					<span class="min-w-0 flex-1">
						<span class="block text-body break-words text-ink">{key.name}</span>
						<span class="block text-read text-ink-2"
							>{key.last_used_at
								? m.access_last_used({ when: howLongAgo(key.last_used_at) })
								: m.access_never_used()}</span
						>
					</span>
					<button
						type="button"
						onclick={() => (asking = { kind: 'revoke', key })}
						class="min-h-12 shrink-0 rounded-sm border border-support px-3 text-read
						font-medium text-support disabled:opacity-60"
					>
						{m.operator_runner_revoke()}
					</button>
				</li>
			{:else}
				<li class="p-3 text-body text-ink-2">{m.operator_runner_keys_empty()}</li>
			{/each}
		</ul>
		<div class="mt-3 rounded-sm border border-rule bg-card p-3">
			<form
				onsubmit={(event) => {
					event.preventDefault();
					void mint();
				}}
			>
				<label class="block text-read text-ink-2">
					{m.operator_runner_name()}
					<input
						bind:value={name}
						required
						class="mt-1 min-h-12 w-full rounded-sm border border-rule bg-ground px-3 text-body"
					/>
				</label>
				<button
					type="submit"
					disabled={acting.working || name.trim() === ''}
					class="mt-2 min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium
					text-accent disabled:opacity-60"
				>
					{m.operator_runner_mint()}
				</button>
			</form>
		</div>
	</Section>

	{#if houseNotes}
		{@const notes = houseNotes}
		<!-- The agent's House Notes (#344): what is written, and who saved them
		     when, then the button that opens the sheet that writes them. -->
		<Section heading={m.operator_notes_heading()}>
			<p class="mb-3 text-read text-ink-2">{m.operator_notes_said()}</p>
			<div class="rounded-sm border border-rule bg-card p-3">
				{#if notes.notes}
					<p class="truncate text-body text-ink">{firstLine(notes.notes)}</p>
					<p class="text-read text-ink-2">{savedLine(notes)}</p>
				{:else}
					<p class="text-body text-ink-2">{m.operator_notes_none_row()}</p>
				{/if}
			</div>
			<button
				type="button"
				onclick={() => (asking = { kind: 'house-notes', notes })}
				class="mt-3 min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium
				text-accent"
			>
				{m.operator_notes_write()}
			</button>
		</Section>
	{/if}
</OperatorOnly>

{#if asking?.kind === 'house-notes'}
	<HouseNotes
		notes={asking.notes}
		onsaved={(saved) => {
			houseNotes = saved;
			asking = { kind: 'house-notes', notes: saved };
		}}
		ondone={close}
	/>
{:else if asking?.kind === 'secret'}
	<RunnerKeySecret name={asking.name} secret={asking.secret} ondone={close} />
{:else if asking?.kind === 'revoke'}
	{@const key = asking.key}
	<Confirm
		title={m.operator_runner_revoke_title({ name: key.name })}
		consequence={m.operator_runner_revoke_consequence()}
		act={m.operator_runner_revoke()}
		busy={acting.working}
		failed={acting.said}
		run={() => revoke(key)}
		cancel={close}
	/>
{/if}
