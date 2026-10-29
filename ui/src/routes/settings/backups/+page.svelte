<!-- Backups (#103, ADR 0039, #469): a page of This instance. -->
<script lang="ts">
	import Refusal from '$lib/Refusal.svelte';
	import { aDate } from '$lib/dates';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { waitForJob } from '$lib/api/job';
	import type { ListBackupsOutput } from '$lib/api/catalogue';
	import OperatorOnly from '../OperatorOnly.svelte';

	const kamosu = useKamosu();

	/**
	 * A Backup copies the database and every Photograph: minutes on a small
	 * machine, so the wait suits that rather than the default that suits the
	 * shortest Job in Kamosu.
	 */
	const LONG_ENOUGH = 30 * 60_000;

	let backups = $state<ListBackupsOutput['backups']>([]);
	let taking = $state(false);
	let saying = $state('');
	let said = $state<string | undefined>(undefined);

	async function load() {
		try {
			backups = (await kamosu.listBackups({})).backups;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			said = error.message;
		}
	}

	async function take() {
		taking = true;
		saying = '';
		said = undefined;
		try {
			await waitForJob(kamosu, (await kamosu.takeBackup({})).job_id, {
				giveUpAfter: LONG_ENOUGH,
				whileWaiting: (job) => {
					if (job.progress.message) saying = job.progress.message;
				},
			});
			backups = (await kamosu.listBackups({})).backups;
		} catch (error) {
			// A Job that failed arrives as a plain Error from waitForJob; a
			// refused Operation as an OperationError. Both are shown as what
			// they say.
			if (!(error instanceof Error)) throw error;
			said = error.message;
		} finally {
			taking = false;
			saying = '';
		}
	}

	/** Megabytes, to one place. An archive is never small enough for bytes. */
	const weight = (bytes: number) => `${(bytes / 1_000_000).toFixed(1)} MB`;
</script>

<OperatorOnly title={m.settings_backups()} onallowed={load}>
	<p class="mb-3 text-read text-ink-2">{m.operator_backups_blurb()}</p>
	{#if said}
		<p class="mb-3 text-body text-support" role="alert"><Refusal words={said} /></p>
	{/if}
	<ul class="overflow-hidden rounded-sm border border-rule bg-card">
		{#each backups as backup (backup.name)}
			<li class="flex items-center gap-3 border-b border-rule p-3 last:border-b-0">
				<span class="min-w-0 flex-1">
					<span class="block text-body text-ink">{aDate(backup.taken_at)}</span>
					<span class="block text-read text-ink-2">{backup.slot} · {weight(backup.size_bytes)}</span
					>
				</span>
				<a
					href="/api/backups/{backup.name}"
					class="min-h-12 shrink-0 content-center rounded-sm border border-accent px-3 text-read
					font-medium text-accent"
				>
					{m.operator_backup_get()}
				</a>
			</li>
		{:else}
			<li class="p-3 text-body text-ink-2">{m.operator_backups_empty()}</li>
		{/each}
	</ul>
	<button
		type="button"
		disabled={taking}
		onclick={take}
		class="mt-3 min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium
		text-accent disabled:opacity-60"
	>
		{taking ? saying || m.operator_working() : m.operator_backup_take()}
	</button>
</OperatorOnly>
