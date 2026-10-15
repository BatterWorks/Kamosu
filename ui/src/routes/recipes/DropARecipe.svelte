<!--
	A recipe dropped onto Recipes (ADR 0044, #204): a PDF, a Kamosu zip file
	(a Bundle, ADR 0020), or a link dragged from another tab. Each is the act
	its row under the + is, run on the + 's own `adding`, so a drop can do
	nothing a button could not, and no drop is the only way to do anything.

	The whole window is the place while Recipes is on screen. The sidebar is
	part of it: somebody aiming at the page and landing an inch to the left
	meant the same thing. So is the search box, which is under the cover with
	everything else by the time anything can be let go.

	**While something is held over it the page is covered** by a sheet of the
	paper with a dashed frame, and words in the middle say what letting go
	will do (`$lib/DropCover`). This was chosen on 7 October 2026 from two options drawn over
	the real Recipes screen (option 1, recorded on #204), over a bar across
	the top that left the recipes in view. Something Recipes cannot take turns
	it the alert colour and says so before it is let go.

	A browser says what kind of file is carried and keeps its name until the
	drop (`$lib/drop`), so the words name a PDF where the type says PDF and
	ask only for "a recipe" where the type is empty. What was dropped is
	judged again by its name once it is here.

	More than one file is refused like anything else that is no recipe: every
	row under the + takes one.

	**On an instance with a Runner a photo of a page is taken too** (#278), and
	a photo and a PDF go to Kamosu's AI agent, shown once on the sheet that
	asks before they are sent: the cover says so while they are held. Whether
	there is a Runner is read from the server when a drag arrives, and again
	when it is let go. With no Runner a photo is refused in words, as before.

	**Not while a sheet is open over the page**, the one a PDF is checked on
	included, and not while an act is running. A drop then is turned away
	like one on any other screen.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { DropZone, type Carried, type Dropped } from '$lib/drop.svelte';
	import { DRAWN } from '$lib/drop-drawings';
	import DropCover from '$lib/DropCover.svelte';
	import { underASheet } from '$lib/page';
	import type { Act, Adding } from '$lib/adding/adding.svelte';
	import { untrack } from 'svelte';
	import { ICON, waitsForServer } from '$lib/adding/sources';

	/** The four acts a drop can be. A photo only where there is a Runner (#278). */
	type DropAct = Extract<Act, 'pdf' | 'file' | 'link' | 'picture'>;
	/** What the page says while something is held over it. */
	type Held = DropAct | 'unsure' | 'photo' | 'no';

	interface Props {
		adding: Adding;
		/** The server is out of reach, so nothing dropped is sent (#76). */
		offline: boolean;
		/** Something was let go here, taken or refused. */
		ondrop: () => void;
	}

	let { adding, offline, ondrop }: Props = $props();

	/**
	 * Which act a file is, by its type and, once it is let go, its name. A
	 * Bundle is a zip, which Windows calls `application/x-zip-compressed`.
	 */
	function sourceOf(type: string, name = ''): Exclude<DropAct, 'link'> | undefined {
		if (type === 'application/pdf' || /\.pdf$/i.test(name)) return 'pdf';
		if (/^application\/(x-)?zip/.test(type) || /\.zip$/i.test(name)) return 'file';
		if (type.startsWith('image/') || /\.(jpe?g|png|gif|webp|heic|heif)$/i.test(name))
			return 'picture';
		return undefined;
	}

	/** Whether this instance has a Runner, as last read from the server (#278). */
	const runner = $derived(adding.runner === true);

	/** What the browser admits to, before the name is known. */
	function held(carried: Carried): Held {
		if (carried.link) return 'link';
		if (carried.files.length !== 1) return 'no';
		const type = carried.files[0];
		if (type === '') return 'unsure';
		const source = sourceOf(type);
		if (source === 'picture') return runner ? 'picture' : 'photo';
		return source ?? 'no';
	}

	/** The act to run on what was let go, or nothing where it is no recipe. */
	function actOn(dropped: Dropped): { source: DropAct; run: () => void } | undefined {
		if ('link' in dropped) {
			const url = dropped.link;
			if (!url || !/^https?:\/\//i.test(url)) return undefined;
			return { source: 'link', run: () => void adding.link(url) };
		}
		if (dropped.files.length !== 1) return undefined;
		const file = dropped.files[0];
		const source = sourceOf(file.type, file.name);
		if (source === 'pdf') return { source, run: () => void adding.pdfChosen(file) };
		// Refused without a Runner, once that has been read again (#278).
		if (source === 'picture') return { source, run: () => void adding.pictureChosen(file) };
		return source && { source, run: () => void adding[source](file) };
	}

	function take(dropped: Dropped) {
		ondrop();
		const act = actOn(dropped);
		const refused = runner ? m.drop_recipes_refused_agent() : m.drop_recipes_refused();
		if (!act || (offline && act.source === 'picture' && !runner)) adding.refuse(refused);
		else if (offline) adding.waitFor(waitsForServer(act.source, runner));
		else act.run();
	}

	const zone = new DropZone(take, () => adding.working === null && !underASheet());
	$effect(() => zone.listen(window));

	/** A drag arriving is when to ask whether there is a Runner, so the cover can say. */
	const dragging = $derived(zone.over !== null);
	$effect(() => {
		if (dragging && !offline) untrack(() => void adding.readRunner());
	});

	const showing = $derived(zone.over ? held(zone.over) : null);
	const refuses = $derived(showing === 'photo' || showing === 'no');

	const ICONS: Record<Held, string> = {
		link: ICON.link,
		file: ICON.file,
		pdf: ICON.pdf,
		picture: ICON.picture,
		unsure: DRAWN.unsure,
		photo: DRAWN.no,
		no: DRAWN.no,
	};

	/** The headline, and the sentence under it. */
	const WORDS = $derived.by((): Record<Held, [() => string, () => string]> => {
		const takes = runner ? m.drop_recipes_takes_agent : m.drop_recipes_takes;
		return {
			pdf: runner
				? [m.drop_recipes_agent_pdf, m.drop_recipes_agent_then]
				: [m.drop_recipes_pdf, m.drop_recipes_pdf_then],
			picture: [m.drop_recipes_picture, m.drop_recipes_agent_then],
			file: [m.drop_recipes_bundle, m.drop_recipes_bundle_then],
			link: [m.drop_recipes_link, m.drop_recipes_link_then],
			unsure: [m.drop_recipes_unsure, takes],
			photo: [m.drop_recipes_photo, takes],
			no: [m.drop_recipes_no, takes],
		};
	});

	/**
	 * Offline, a source says what its row under the + says and nothing more.
	 * A file the browser cannot name yet is not known to be one, and says so
	 * once it is let go.
	 */
	const waitsFor = $derived(
		offline &&
			(showing === 'pdf' || showing === 'file' || showing === 'link' || showing === 'picture')
			? waitsForServer(showing, runner)
			: null,
	);
</script>

{#if showing}
	<!--
		`inset-x-0` and not `inset-0`: beside the sidebar that is the class the
		rail moves over (`beside-rail` in app.css). Above the + and its list
		(z-30), and never beside a sheet, since none is open while this is.
	-->
	<DropCover
		class="fixed inset-x-0 inset-y-0 z-40"
		icon={ICONS[showing]}
		headline={waitsFor ?? WORDS[showing][0]()}
		sentence={waitsFor ? null : WORDS[showing][1]()}
		{refuses}
	/>
{/if}
