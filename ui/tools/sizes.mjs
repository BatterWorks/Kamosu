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

/**
 * The narrowest phone a screen is held to, which is not photographed: the
 * width of many Android phones, where the tab bar has the least room (#600).
 */
export const NARROWEST_PHONE = [360, 760];

/**
 * The sizes that stand for a screen with a mouse rather than a finger. Every
 * other one is a phone or a tablet, which `just ux-measure` opens as a touch
 * screen (#629). An iPad is wide and touch (#223).
 */
export const MOUSE_SIZES = ['macbook', 'monitor'];

/**
 * The two phones the UX audit judges comfort at (#627, decision 3), named by
 * their widths: an iPhone 17 and a Pixel 10. A list of their own, so `just
 * screens` keeps photographing the nine above.
 */
export const AUDIT_PHONES = {
	'phone-402': [402, 874],
	'phone-412': [412, 924],
};
