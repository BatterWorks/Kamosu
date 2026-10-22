/**
 * Putting a picture on a cooking (#77, ADR 0005), the one way both the
 * cooking screen and the diary do it.
 *
 * The picture is kept on the phone first and named there, then added to the
 * cooking beside whatever it already holds (`add_photographs`), so a picture
 * taken with no network is as much a part of the cooking as one taken at home.
 * When it reaches the server it is remade at the door and named by its bytes
 * (ADR 0017); until then the outbox shows it from the phone.
 */

import type { KamosuClient, StartAttemptOutput } from '$lib/api/catalogue';
import { OperationError } from '$lib/api/client';
import type { Keeping } from './outbox';

export async function photographCooking(
	kamosu: KamosuClient,
	keeping: Keeping,
	attemptId: string,
	picture: Blob,
): Promise<{ photographs: StartAttemptOutput['photographs']; inADemo: boolean }> {
	const name = await keeping.keepPhotograph(picture);
	const edited = await kamosu.editAttempt({ attempt_id: attemptId, add_photographs: [name] });
	if (!(await isADemo(kamosu))) return { photographs: edited.photographs, inADemo: false };
	// A Demo keeps no photograph (#403), so the server left this one out. It
	// stands where it was put, drawn from the phone, until the screen next
	// reads the cooking, and the screen says why it goes.
	const photographs = edited.photographs.includes(name)
		? edited.photographs
		: [...edited.photographs, name];
	return { photographs, inADemo: true };
}

/**
 * Whether this is a Demo, which keeps no photograph (#403). An instance that
 * cannot be asked just now is taken for none.
 */
async function isADemo(kamosu: KamosuClient): Promise<boolean> {
	try {
		return (await kamosu.instanceStatus()).demo;
	} catch (error) {
		if (!(error instanceof OperationError)) throw error;
		return false;
	}
}

/** The one file a picker handed over, and the picker emptied for the next. */
export function pickedFile(event: Event): File | undefined {
	const input = event.currentTarget as HTMLInputElement;
	const file = input.files?.[0];
	input.value = '';
	return file;
}
