/**
 * Every screen size live acceptance uses, by name (#216).
 *
 * The list is #192's: an iPad mini and an 11" iPad each way up, a 10" Android
 * tablet, a sideways phone, a MacBook 14" at its default setting and a large
 * monitor. #192 gives no numbers for the Android tablet or the monitor; theirs
 * are the ones live acceptance used from #196 to #205. The upright phone is
 * here too, because "the phone is unchanged" is the question most tickets end
 * on.
 *
 * Each size is `[width, height]` in CSS pixels. A name is part of a picture's
 * file name, so renaming one orphans every "before" picture already taken.
 */
export const SIZES = {
	phone: [390, 844],
	'phone-sideways': [850, 390],
	'mini-upright': [744, 1133],
	'mini-sideways': [1133, 744],
	'ipad-upright': [834, 1194],
	'ipad-sideways': [1194, 834],
	android: [800, 1280],
	macbook: [1512, 982],
	monitor: [2560, 1440],
};
