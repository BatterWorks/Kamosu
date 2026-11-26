/**
 * What a timer does when it runs out besides changing its chip (#681): three
 * short tones and a buzz, because a cook watching the pan does not see the
 * screen change, and all three graders of the first round found the end of a
 * timer silent.
 *
 * Sound is made through the Web Audio API, and iOS lets a page make sound only
 * from a context opened inside a tap. So `ready` is called in the tap that
 * starts the timer, and `ring` plays through what that tap opened. A browser
 * without either simply stays quiet: the chip still says the time is up.
 */
export interface Chime {
	/** Open the sound, inside the tap that starts a timer. */
	ready(): void;
	/** Sound and buzz once. */
	ring(): void;
}

export function chime(): Chime {
	let context: AudioContext | undefined;
	return {
		ready() {
			if (typeof AudioContext === 'undefined') return;
			try {
				context ??= new AudioContext();
				// Again on every tap, not only the first: iOS lets a page sound
				// from some taps and not others (a finger landing is not enough),
				// and a context opened on the wrong one stayed silent (review of
				// #681, unchecked on an iPhone).
				if (context.state === 'suspended') void context.resume();
			} catch {
				context = undefined;
			}
		},
		ring() {
			if ('vibrate' in navigator) navigator.vibrate([400, 200, 400, 200, 400]);
			if (!context) return;
			void context.resume();
			const start = context.currentTime;
			for (let tone = 0; tone < 3; tone++) {
				const at = start + tone * 0.5;
				const pitch = context.createOscillator();
				const level = context.createGain();
				pitch.frequency.value = 880;
				level.gain.setValueAtTime(0.0001, at);
				level.gain.exponentialRampToValueAtTime(0.4, at + 0.02);
				level.gain.exponentialRampToValueAtTime(0.0001, at + 0.35);
				pitch.connect(level).connect(context.destination);
				pitch.start(at);
				pitch.stop(at + 0.4);
			}
		},
	};
}
