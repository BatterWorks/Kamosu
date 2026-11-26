/**
 * The timer a Step offers — one tap, nothing typed and nothing stored
 * (ADR 0011).
 *
 * It is **the cook's, not the step's**: it lives here, beside the screen, so it
 * follows them to the next step rather than being cancelled by moving on. That
 * is why one is set at all — you start the rice timer and then get on with the
 * sauce.
 *
 * When it runs out it **says so and stays** rather than disappearing, because a
 * timer that vanishes at zero is one a cook who was looking at the pan never
 * sees. It is put away by hand, which is also how a second one is started.
 *
 * Nothing here is written to the server: how far through a timer is, is not
 * part of what a cooking is (ADR 0010 holds the Step, the ticks and the Yield,
 * and deliberately nothing else). The screen keeps the moment it ends on the
 * device itself, so a timer survives the cook looking at the recipe and coming
 * back, or the phone reloading the page: both lost it without a word until
 * round 1 of #681's climb. `resume` takes it back.
 */

export interface Timer {
	/** Seconds left, or null when nothing is running and nothing has just ended. */
	readonly remaining: number | null;
	/** True once it has reached zero and before the cook has put it away. */
	readonly rung: boolean;
	/** The moment it ends, in the clock's milliseconds, or null with none set. */
	readonly endsAt: number | null;
	/** Start (or restart) a timer of this many seconds. */
	start(seconds: number): void;
	/**
	 * Take back a timer set earlier, by the moment it ends. One that ended
	 * meanwhile says so and does not ring again: the cook was elsewhere.
	 */
	resume(endsAt: number): void;
	/** Put it away — stop a running one, or acknowledge one that has rung. */
	clear(): void;
	/** Stop the interval. The screen calls this as it goes away. */
	dispose(): void;
}

/**
 * @param now the clock, which a test replaces
 * @param onRing called once as a timer reaches zero, for the sound and the buzz
 *   the screen makes then: a cook looking at the pan does not see the chip change
 *   (#681)
 */
export function timer(now: () => number = Date.now, onRing: () => void = () => {}): Timer {
	let endsAt = $state<number | null>(null);
	let remaining = $state<number | null>(null);
	let ticking: ReturnType<typeof setInterval> | undefined;

	function tick() {
		if (endsAt === null) return;
		const left = Math.ceil((endsAt - now()) / 1000);
		remaining = left > 0 ? left : 0;
		if (remaining === 0) {
			stopTicking();
			onRing();
		}
	}

	function stopTicking() {
		clearInterval(ticking);
		ticking = undefined;
	}

	return {
		get remaining() {
			return remaining;
		},
		get rung() {
			return remaining === 0;
		},
		get endsAt() {
			return endsAt;
		},
		start(seconds: number) {
			stopTicking();
			endsAt = now() + seconds * 1000;
			remaining = seconds;
			ticking = setInterval(tick, 250);
		},
		resume(at: number) {
			stopTicking();
			endsAt = at;
			const left = Math.ceil((at - now()) / 1000);
			remaining = left > 0 ? left : 0;
			if (remaining > 0) ticking = setInterval(tick, 250);
		},
		clear() {
			stopTicking();
			endsAt = null;
			remaining = null;
		},
		dispose() {
			stopTicking();
		},
	};
}

/**
 * Where a cooking's timer is kept on the device: the moment it ends, under the
 * cooking's id, as the cook's own words are kept beside it (`as-cooked`). A
 * browser that keeps nothing keeps no timer, as before.
 */
const timerKey = (attemptId: string) => `kamosu.timer.${attemptId}`;

export function keepTimer(attemptId: string, endsAt: number | null): void {
	try {
		if (endsAt === null) localStorage.removeItem(timerKey(attemptId));
		else localStorage.setItem(timerKey(attemptId), String(endsAt));
	} catch {
		// Nothing kept: the timer lasts as long as the page, as it did.
	}
}

export function keptTimer(attemptId: string): number | null {
	try {
		const at = Number(localStorage.getItem(timerKey(attemptId)));
		return Number.isFinite(at) && at > 0 ? at : null;
	} catch {
		return null;
	}
}

/**
 * A running timer's clock — `4:07`, `1:02:30`. Digits rather than words on
 * purpose: it is read at a glance from across a kitchen, and the colon form is
 * the one every oven and phone already uses, in every Language.
 */
export function clock(seconds: number): string {
	const safe = Math.max(0, Math.floor(seconds));
	const hours = Math.floor(safe / 3600);
	const minutes = Math.floor((safe % 3600) / 60);
	const rest = safe % 60;
	const pad = (n: number) => String(n).padStart(2, '0');
	return hours > 0 ? `${hours}:${pad(minutes)}:${pad(rest)}` : `${minutes}:${pad(rest)}`;
}
