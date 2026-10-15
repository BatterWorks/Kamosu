/**
 * The ways a new recipe starts, written once (#174): a web page, a Bundle
 * somebody sent you (a Kamosu zip file, ADR 0020), a title you type yourself,
 * a whole recipe pasted as text (#175), and a recipe PDF (#176). On an
 * instance with a Runner a photo of a page is one too, and a photo, a PDF and
 * pasted text go to Kamosu's AI agent rather than to a draft sorted by hand
 * (#278).
 *
 * Two places offer them. The + beside the search box offers all three every
 * day, and the dead ends `AddOrImport` draws (a search that matched nothing,
 * an empty Home) offer them as full-width buttons. Each place holds its own
 * `Adding`, so one screen's work never quiets the other's buttons, but the acts
 * themselves live here and nowhere else. One act written twice is one act that
 * will eventually be two, which is how #93 moved the first two out of
 * `AddOrImport` and why #174 moved the third out after them.
 *
 * **Every act *does* the thing and ends on the recipe** (ADR 0027). A link and
 * a file are one recipe each and open it; a title *is* a recipe (#6), so
 * writing one opens it too. Nobody is asked where it goes: a recipe written or
 * brought in lands in its writer's own Cookbook (ADR 0041).
 */

import { goto } from '$app/navigation';
import { m } from '$lib/paraglide/messages';
import { useKamosu } from '$lib/kamosu';
import { useUpload, type Uploader } from '$lib/api/upload';
import { OperationError } from '$lib/api/client';
import { StillRunning, waitForJob } from '$lib/api/job';
import { receiveStaged } from '$lib/receive';
import type { ImportWebLinkOutput, KamosuClient, ReadPastedRecipeOutput } from '$lib/api/catalogue';
import { holdPaste, readPdf, type PageReading, type PastedDraft } from '$lib/pasted.svelte';
import { noteArrival } from '$lib/arrival.svelte';

/** `send` is handing a photo, a PDF or pasted text to Kamosu's AI agent (#278). */
export type Act = 'link' | 'file' | 'pdf' | 'picture' | 'write' | 'paste' | 'send';

/** A photo or a PDF chosen or dropped, shown once before it goes to the agent (#278). */
export interface ToSend {
	file: File;
	kind: 'picture' | 'pdf';
}

export class Adding {
	/** The act running now, or none. Every button offering an act quiets while one runs. */
	working = $state<Act | null>(null);
	/** Why the last act landed nothing, in the words it failed with. */
	failed = $state<string | undefined>(undefined);
	/**
	 * The wait ran out before the Job did (#117). Not a failure: the recipe
	 * still arrives, and the screen says so rather than what went wrong.
	 */
	stillGoing = $state(false);
	/**
	 * A PDF read and waiting to be checked (#176), or none. It is checked on
	 * the sheet a paste is checked on, and made the same way.
	 */
	fromPdf = $state<ReadPastedRecipeOutput | null>(null);
	/**
	 * A web page with no recipe data, read from its own text and waiting to
	 * be checked (#301), or none. Only an instance with no Runner answers
	 * one: it is checked on the sheet a paste is checked on, and its page
	 * goes with it as the recipe's Source.
	 */
	fromLink = $state<PageReading | null>(null);
	/**
	 * What an act is waiting for, said where it was asked (#204). A button
	 * says this in its own label and cannot be pressed; a file dropped while
	 * the server is out of reach has no label to read, so it is said here.
	 */
	waiting = $state<string | undefined>(undefined);
	/**
	 * Whether this instance has a Runner, so that a photo, a PDF and pasted
	 * text go to Kamosu's AI agent (#278), or nothing while that has not been
	 * read yet. Read from the server each time it matters, never from what
	 * the phone kept (`runner_status` is never cached), and `null` reads as
	 * no Runner: the + then offers what it offers on every instance.
	 */
	runner = $state<boolean | null>(null);
	/** A photo or a PDF waiting on the sheet that asks before it is sent, or none. */
	toSend = $state<ToSend | null>(null);

	#kamosu: KamosuClient;
	#upload: Uploader;

	constructor(kamosu: KamosuClient, upload: Uploader) {
		this.#kamosu = kamosu;
		this.#upload = upload;
	}

	/** Reading a web page is slow, so importing is a Job (ADR 0032). */
	link = (url: string) =>
		this.#run('link', async () => {
			const asked = await this.#kamosu.importWebLink({ url: url.trim() });
			const finished = await waitForJob(this.#kamosu, asked.job_id);
			const result = finished.result as ImportWebLinkOutput;
			// Nothing landed: the page was read from its own text, and waits
			// to be checked before anything is made of it (#301).
			if (result.to_check) {
				this.fromLink = result.to_check;
				return undefined;
			}
			// A Share Link arrives as the Bundle it serves (#169), so its Report
			// names the shared recipe as its subject beside the Passengers that
			// travelled with it. A web page's single row names no subject.
			const landed =
				result.arrived?.find((row) => row.subject) ?? result.arrived?.[0] ?? result.offered?.[0];
			if (landed) {
				await goto(`/recipes/${landed.branch_id}`);
				return undefined;
			}
			// The page was reached and held no recipe this instance could read.
			// Saying so is the whole answer; there is nothing to open.
			return result.unreadable?.[0]?.reason ?? m.recipes_import_unreadable();
		});

	/**
	 * A Bundle, sent as its own bytes and then named to the Operation.
	 *
	 * One recipe carrying photographs is megabytes, and base64 inside a JSON
	 * body would be a third larger again and would sit whole in the Job's
	 * stored input, so it travels to `POST /api/uploads` and `import_bundle` is
	 * asked with the id that answers (#93, ADR 0001).
	 */
	file = (file: File) =>
		this.#run('file', async () => receiveStaged(this.#kamosu, await this.#upload(file)));

	/**
	 * A recipe PDF (#176), sent as its own bytes the way a Bundle is and read
	 * by `read_recipe_pdf`, which saves nothing. What it made waits in
	 * `fromPdf` for the sheet a paste is checked on; nothing is made until
	 * that sheet's "Make this recipe".
	 */
	pdf = (file: File) =>
		this.#run('pdf', async () => {
			const answer = await readPdf(this.#kamosu, await this.#upload(file));
			if (typeof answer === 'string') return answer;
			this.fromPdf = answer;
			return undefined;
		});

	/**
	 * Read again whether this instance has a Runner (#278). A refusal leaves
	 * what was known: the + then draws what it drew.
	 */
	async readRunner(): Promise<boolean> {
		try {
			const status = await this.#kamosu.runnerStatus({});
			this.runner = status.has_runner;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
		}
		return this.runner === true;
	}

	/**
	 * A PDF was chosen or dropped. With a Runner it goes to the agent, a scan
	 * included, once the sheet has shown it; without one it is read into a
	 * draft as it always was (#176).
	 */
	pdfChosen = (file: File) =>
		this.#chosen(async () => {
			if (await this.readRunner()) this.offer({ file, kind: 'pdf' });
			else await this.pdf(file);
		});

	/**
	 * A photo of a page was chosen or dropped (#278). Only an instance with a
	 * Runner reads one, and a photo dropped on one without is refused in the
	 * words a drop is refused with.
	 */
	pictureChosen = (file: File) =>
		this.#chosen(async () => {
			if (await this.readRunner()) this.offer({ file, kind: 'picture' });
			else this.refuse(m.drop_recipes_refused());
		});

	/** A file chosen while whether there is a Runner is still being read. */
	#choosing = false;

	/**
	 * One file at a time: a second chosen or dropped while the first waits
	 * for the server's answer is let go, as one is while an act runs.
	 */
	async #chosen(decide: () => Promise<void>) {
		if (this.working || this.#choosing) return;
		this.#choosing = true;
		try {
			await decide();
		} finally {
			this.#choosing = false;
		}
	}

	/** Forget what the last act said, as a new choice starts. */
	quiet() {
		if (this.working) return;
		this.failed = undefined;
		this.waiting = undefined;
		this.stillGoing = false;
	}

	/** Show a photo or a PDF on the sheet that asks before it is sent. */
	offer(chosen: ToSend) {
		this.failed = undefined;
		this.waiting = undefined;
		this.stillGoing = false;
		this.toSend = chosen;
	}

	/**
	 * Hand a photo, a PDF or pasted text to Kamosu's AI agent (#278), and go
	 * to its Import, which says whether it waits or is being read (#277). A
	 * file handed over before is known by its contents (#271): a Job still on
	 * it is that same Import, and a recipe already made from it opens, saying
	 * it was read before.
	 */
	send = (handed: { text: string } | { file: File }) =>
		this.#run('send', async () => {
			const input =
				'text' in handed ? { text: handed.text } : { upload_id: await this.#upload(handed.file) };
			const answer = await this.#kamosu.importByAgent(input);
			this.toSend = null;
			if (answer.job_id) {
				await goto(`/imports/${answer.job_id}`);
				return undefined;
			}
			if (answer.branch_id) {
				noteArrival({
					branchId: answer.branch_id,
					title: answer.title ?? '',
					how: 'read_before',
					passengers: [],
				});
				await goto(`/recipes/${answer.branch_id}`);
			}
			return undefined;
		});

	/**
	 * A recipe needs only a title (#6), so the title *is* the recipe. A title of
	 * nothing but spaces is not one, and asks nothing.
	 */
	write = (title: string) =>
		this.#run('write', async () => {
			if (title.trim() === '') return undefined;
			const made = await this.#kamosu.createRecipe({ title: title.trim() });
			await goto(`/recipes/${made.branch_id}`);
			return undefined;
		});

	/**
	 * A pasted recipe, already read and checked (#175). The recipe is made from
	 * its title alone, like writing one, and opens on its writing screen with
	 * the lines filled in and NOT saved: they wait for Save, as a paste made
	 * on that screen does (#83). Nothing about the lines can be refused here,
	 * so only the title's refusal is this act's to say.
	 */
	paste = (title: string, draft: PastedDraft) =>
		this.#run('paste', async () => {
			if (title.trim() === '') return undefined;
			const made = await this.#kamosu.createRecipe({ title: title.trim() });
			holdPaste(made.branch_id, { ...draft, title: title.trim() });
			this.fromPdf = null;
			this.fromLink = null;
			await goto(`/recipes/${made.branch_id}`);
			return undefined;
		});

	/**
	 * Say why nothing was started, where an act would have said it (#204):
	 * something was dropped that is no recipe. Never over an act that is
	 * running, whose own answer is still to come.
	 */
	refuse(words: string) {
		if (this.working) return;
		this.stillGoing = false;
		this.waiting = undefined;
		this.failed = words;
	}

	/** Say what a dropped recipe waits for, while the server is out of reach (#204). */
	waitFor(words: string) {
		if (this.working) return;
		this.stillGoing = false;
		this.failed = undefined;
		this.waiting = words;
	}

	/**
	 * Every act starts and ends through here. An act answers the sentence to
	 * say where it was asked when nothing landed, or nothing once it has gone
	 * to the recipe.
	 *
	 * A Job that failed arrives as a plain Error from `waitForJob`, and a
	 * refused Operation as an OperationError: both are the act's to say, because
	 * a person who has just chosen a file or pasted a link is owed a sentence,
	 * never a silence (#93). An ended Share Link is the first kind: its Job
	 * fails in the share page's own words, which the reader is owed (#169).
	 */
	async #run(act: Act, perform: () => Promise<string | undefined>) {
		if (this.working) return;
		this.working = act;
		this.failed = undefined;
		this.waiting = undefined;
		this.stillGoing = false;
		try {
			this.failed = await perform();
		} catch (error) {
			if (!(error instanceof Error)) throw error;
			// Writing, pasting and sending are one Operation each and no Job
			// (an upload first, for a file), so only a
			// refusal is theirs to say; anything else is a Mistake for the
			// layers that catch those.
			// Kept here rather than in `write`, so every act leaves `working`
			// the same way.
			if (
				(act === 'write' || act === 'paste' || act === 'send') &&
				!(error instanceof OperationError)
			)
				throw error;
			if (error instanceof StillRunning) this.stillGoing = true;
			else this.failed = error.message;
		} finally {
			this.working = null;
		}
	}
}

/** An `Adding` on this screen's client and uploader. Call it while the component is being made. */
export function useAdding(): Adding {
	return new Adding(useKamosu(), useUpload());
}
