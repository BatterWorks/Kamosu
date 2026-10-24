<!--
	Import from another app (#412, option B of 23 October 2026): a page of its
	own on the Settings menu, in place of the Crouton button that sat on This
	instance. The prototype is `.dev/prototypes/412-import-place/b-place-phone.html`
	and its states.

	The cook brings the export their old app makes, dropped on the place or
	picked with its button, and Kamosu works out which app made it. The file
	goes to `POST /api/uploads` first (ADR 0001), and `import_from_another_app`
	is asked with the id that answers. **The page waits for the file to be
	recognised before it moves to the Report**: a file turned away is said
	here, under the place, so the cook can try another without going back.
	Recognised is the Job counting its recipes, or ending. A cook who leaves
	while it looks is not moved anywhere; the import carries on, and is under
	Brought in.

	The place takes any one file (ADR 0044): which app made it is judged on
	the server by what is in it, never here by its name. Under it, the apps
	Kamosu reads, each with where its export is, and where to ask for another.
	Only apps Kamosu reads today are named.

	A Mealie backup of several households is not refused but asked about
	(#418): the sheet in `WhichHousehold.svelte` rises over the page, and the
	same upload, which the server keeps for it, is imported again naming the
	household chosen. Another file brought meanwhile puts the question away.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import Refusal from '$lib/Refusal.svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { StillRunning, waitForJob } from '$lib/api/job';
	import { useUpload } from '$lib/api/upload';
	import Section from '$lib/shell/Section.svelte';
	import { FilePlace, type FileKind } from '$lib/drop.svelte';
	import { drawnFor } from '$lib/drop-drawings';
	import { ICON } from '$lib/adding/sources';
	import DropCover from '$lib/DropCover.svelte';
	import RefusedExport from '$lib/RefusedExport.svelte';
	import WhichHousehold from '$lib/WhichHousehold.svelte';
	import { ASK_FOR_AN_APP, OTHER_APPS } from '$lib/other-apps';
	import type { ImportFromAnotherAppOutput } from '$lib/api/catalogue';

	interface Props {
		/** How long to wait between reads of the Job. Tests shorten it. */
		every?: number;
	}

	let { every = 400 }: Props = $props();

	const kamosu = useKamosu();
	const upload = useUpload();

	/** What the place is doing with a file, and its size while it is sent. */
	let working = $state<{ sending: string } | 'looking' | undefined>(undefined);
	let failed = $state<string | undefined>(undefined);
	let refused = $state<
		{ answer: NonNullable<ImportFromAnotherAppOutput['refused']>; file: string } | undefined
	>(undefined);
	let asking = $state<
		| {
				households: NonNullable<ImportFromAnotherAppOutput['which_household']>['households'];
				uploadId: string;
				file: string;
		  }
		| undefined
	>(undefined);
	let lastJob = $state<string | undefined>(undefined);
	/** Whether the page is still open: one left while it looks moves nobody. */
	let open = true;
	$effect(() => () => {
		open = false;
	});

	/** Either way in brings a library: the Crouton button's Imports are this place's too. */
	const IMPORTS = ['import_from_another_app', 'import_crouton'];

	$effect(() => {
		let current = true;
		kamosu
			.listJobs()
			.then((answer) => {
				const last = answer.jobs.find((job) => IMPORTS.includes(job.operation));
				if (current) lastJob = last?.id;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});

	const megabytes = (bytes: number) => `${Math.max(1, Math.round(bytes / 1_000_000))} MB`;

	function choose(event: Event & { currentTarget: HTMLInputElement }) {
		const file = event.currentTarget.files?.[0];
		event.currentTarget.value = '';
		if (file) void bring(file);
	}

	async function bring(file: File) {
		failed = undefined;
		refused = undefined;
		asking = undefined;
		working = { sending: megabytes(file.size) };
		try {
			await look(await upload(file), file.name);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			working = undefined;
		}
	}

	/** Bring in the household the cook chose, from the upload kept for it. */
	async function chooseHousehold(household: string) {
		const { uploadId, file } = asking!;
		asking = undefined;
		failed = undefined;
		working = 'looking';
		try {
			await look(uploadId, file, household);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			working = undefined;
		}
	}

	/** Ask for the upload to be imported, and wait until its file is recognised. */
	async function look(uploadId: string, fileName: string, household?: string) {
		const asked = await kamosu.importFromAnotherApp({
			upload_id: uploadId,
			...(household === undefined ? {} : { mealie_household: household }),
		});
		working = 'looking';
		// This import is now the last, so its link leads back to a question
		// put away with Not now.
		lastJob = asked.job_id;
		let recognised = false;
		const job = await waitForJob(kamosu, asked.job_id, {
			every,
			giveUpAfter: 60_000,
			whileWaiting: (read) => {
				recognised = read.progress.total !== null;
			},
			stopped: () => recognised || !open,
			// A failure is the Report's to say, as for any import.
			answersWhenFailed: () => true,
		}).catch((error: unknown) => {
			// Still waiting for its turn: the Report shows the wait.
			if (error instanceof StillRunning) return undefined;
			throw error;
		});
		if (!open) return;
		const answer = job?.status === 'completed' ? (job.result as ImportFromAnotherAppOutput) : null;
		if (answer?.refused) {
			refused = { answer: answer.refused, file: fileName };
			return;
		}
		if (answer?.which_household) {
			asking = { households: answer.which_household.households, uploadId, file: fileName };
			return;
		}
		await goto(`/imports/${asked.job_id}`);
	}

	/** Any one file: which app made it is the server's to say, by what is in it. */
	const anyFile: FileKind = () => true;

	const place = new FilePlace(
		anyFile,
		(file) => void bring(file),
		() => working === undefined,
	);

	const SAYS = {
		ok: [m.drop_export, m.drop_export_why],
		unsure: [m.drop_export, m.drop_export_why],
		no: [m.drop_export_one, m.drop_export_one_why],
	} as const;

	/** The ask's sentence, cut where its link goes, so it is translated whole. */
	const missing = $derived(
		m.import_other_missing({ link: '\u0000' }).split('\u0000') as [string, string],
	);
</script>

<p class="text-read text-ink-2">{m.import_other_blurb()} {m.import_other_again()}</p>

<div
	class="relative mt-4 flex flex-col items-center gap-2 rounded-md border-2 border-dashed border-rule bg-card px-4 py-6 text-center"
	{@attach place.listen}
>
	{#if place.held}
		<DropCover
			class="absolute inset-0 z-10 rounded-md"
			icon={drawnFor(place.held, ICON.file)}
			headline={SAYS[place.held][0]()}
			sentence={SAYS[place.held][1]()}
			refuses={place.held === 'no'}
		/>
	{/if}
	<svg
		viewBox="0 0 24 24"
		aria-hidden="true"
		class="h-8 w-8 text-ink-2"
		fill="none"
		stroke="currentColor"
		stroke-width="1.5"
		stroke-linecap="round"
		stroke-linejoin="round"><path d={ICON.file} /></svg
	>
	<p class="text-body font-semibold">{m.import_other_drop_here()}</p>
	<p class="text-read text-ink-2">{m.import_other_or()}</p>
	<label
		class="relative inline-flex min-h-12 cursor-pointer items-center rounded-sm bg-accent px-4 font-semibold text-on-accent has-disabled:opacity-60"
	>
		{working === 'looking'
			? m.import_other_looking()
			: working
				? m.import_other_sending({ size: working.sending })
				: m.import_other_choose()}
		<input type="file" class="sr-only" disabled={working !== undefined} onchange={choose} />
	</label>
</div>
{#if place.refused}
	<p class="mt-2 text-read text-support" role="alert">{m.drop_export_refused()}</p>
{/if}
{#if failed}
	<p class="mt-2 border-l-3 border-support bg-card px-3 py-2 text-read" role="alert">
		<Refusal words={failed} />
	</p>
{/if}
{#if refused}
	<div class="mt-4">
		<RefusedExport refused={refused.answer} file={refused.file} />
	</div>
{/if}
{#if asking}
	<WhichHousehold
		households={asking.households}
		file={asking.file}
		choose={(household) => void chooseHousehold(household)}
		close={() => (asking = undefined)}
	/>
{/if}
{#if lastJob}
	<p class="mt-3 text-read">
		<a href="/imports/{lastJob}" class="text-accent underline">{m.import_other_last()}</a>
	</p>
{/if}

<Section heading={m.import_other_apps()}>
	<ul class="border-t border-rule">
		{#each OTHER_APPS as app (app.kind)}
			<li class="border-b border-rule py-3">
				<span class="block text-body font-medium">{app.name}</span>
				<span class="block text-read text-ink-2">{app.where()}</span>
			</li>
		{/each}
	</ul>
	<p class="mt-3 text-read text-ink-2">
		{missing[0]}<a href={ASK_FOR_AN_APP} class="text-accent underline"
			>{m.import_other_ask_github()}</a
		>{missing[1]}
	</p>
</Section>
