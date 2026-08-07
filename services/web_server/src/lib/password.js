/**
 * Share-password generator.
 *
 * The design shows a readable, dash-segmented password (`k7-Fern-Ridge-92`)
 * rather than an opaque blob, because people have to copy this somewhere and
 * read it back: an unreadable password gets written down badly or not at all.
 *
 * Readable must not mean weak. This password derives the file's encryption key
 * through Argon2id, so it is the whole security of the drop. The shape is
 * 2 alnum + 4 words + 2 digits, drawn from `crypto.getRandomValues` with
 * rejection sampling so the modulo does not bias the low end of each range.
 *
 * The strength is `PASSWORD_ENTROPY_BITS` below, computed from the actual
 * alphabets rather than asserted in prose here: the figure that used to live in
 * this comment drifted from the code, and a wrong number in a crypto file is
 * worse than no number. Every one of those bits costs an attacker a full
 * Argon2id evaluation at libsodium's MODERATE limits (t=3, m=256 MiB, the
 * parameters `crypto.worker.js` and `chacha.js` actually pass), which puts
 * offline cracking out of reach at any realistic scale and far beyond what
 * people pick for themselves.
 */

// Short, unambiguous words: no homophones, nothing that looks alike in mono,
// nothing that reads as offensive when two land side by side. Deduplicated on
// load so a repeated entry shrinks the list honestly instead of quietly biasing
// the draw and inflating the entropy figure.
const WORDS = Array.from(
	new Set(
		'Able Acorn Adobe Agile Alder Alloy Amber Anchor Angle Anvil Apex Apple Arbor Arch Arrow Ash Aspen Atlas Aura Axis Badge Basin Beacon Beam Bear Beech Bell Berry Birch Bison Blade Bloom Bluff Bolt Bone Book Boulder Brace Branch Brass Brick Bridge Brook Buck Bulb Cabin Cable Cactus Cairn Camp Canvas Cape Cedar Chalk Chart Chime Cinder Clay Cliff Cloud Clover Coal Coast Cobalt Comet Compass Copper Coral Cove Crane Crate Creek Crest Crown Cube Cypress Dale Dawn Delta Denim Dial Dock Dome Dove Drift Dune Dusk Eagle Earth Echo Elder Elm Ember Ridge Falcon Fern Field Finch Flint Flute Foam Forge Fox Frost Garnet Gate Glade Glass Globe Gorge Granite Grove Gull Hail Harbor Hawk Hazel Hearth Heath Hedge Helm Hill Hollow Honey Hazel Ice Indigo Iris Iron Ivory Ivy Jade Jetty Junco Juniper Kelp Kettle Key Kiln Knoll Lace Lagoon Lake Lantern Lark Laurel Leaf Ledge Linen Lion Loam Lodge Loom Lotus Lumen Lynx Maple Marble Marsh Meadow Mesa Mica Mint Mist Moss Moth Mountain Nectar Nest Nickel Night Nomad North Oak Oasis Ochre Olive Onyx Opal Orbit Orchid Otter Owl Palm Pearl Pebble Pine Pillar Pilot Plain Plume Pond Poplar Prairie Quarry Quartz Quill Rain Raven Reed Reef Relay Rill River Rock Rope Rose Rust Sable Sage Sail Sand Sapling Scout Sedge Shale Shell Shore Signal Silver Slate Sleet Slope Smoke Snow Spark Spire Spring Spruce Stone Storm Stream Summit Sun Swift Sycamore Talon Teak Thicket Thorn Tide Timber Tin Topaz Torch Trail Tulip Tundra Vale Vault Vine Violet Wave Wharf Wheat Willow Wind Wolf Wood Wren Yarrow Yew Zenith Zinc'.split(' ')
	)
);

// No `l`/`o`, which are the characters people misread when copying by hand.
const ALNUM = 'abcdefghijkmnpqrstuvwxyz23456789';

/**
 * Entropy of a generated password, in bits, derived from the alphabets and shape
 * actually used above. Change `ALNUM`, `WORDS` or `generatePassword` and this
 * follows automatically, which is the point: the previous hand-written figure
 * claimed a word count and a set of Argon2id parameters that the code had since
 * moved away from.
 */
export const PASSWORD_ENTROPY_BITS =
	2 * Math.log2(ALNUM.length) + 4 * Math.log2(WORDS.length) + 2 * Math.log2(10);

function randomInts(count, max) {
	const out = [];
	// Rejection-sample so the modulo does not bias the low end of the range.
	const limit = Math.floor(0x100000000 / max) * max;
	const buf = new Uint32Array(count * 2);
	while (out.length < count) {
		crypto.getRandomValues(buf);
		for (const n of buf) {
			if (n < limit) {
				out.push(n % max);
				if (out.length === count) break;
			}
		}
	}
	return out;
}

/** `k7-Fern-Ridge-Slate-Cove-92`: readable, `PASSWORD_ENTROPY_BITS` of entropy. */
export function generatePassword() {
	const [a, b] = randomInts(2, ALNUM.length);
	const wordIdx = randomInts(4, WORDS.length);
	const [d1, d2] = randomInts(2, 10);
	const prefix = ALNUM[a] + ALNUM[b];
	const words = wordIdx.map((i) => WORDS[i]).join('-');
	return `${prefix}-${words}-${d1}${d2}`;
}
