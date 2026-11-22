// Same rule as crates/glaude-core/src/bidi.rs: a block is RTL when at
// least 30% of its strong letters are RTL. Used live in the composer.

import type { Dir } from "./types";

const RTL_RE = /[֐-ࣿיִ-﷿ﹰ-﻿]/u;
const LETTER_RE = /\p{L}/u;

export function detectDir(text: string): Dir {
	let rtl = 0;
	let ltr = 0;
	let inCode = false;
	for (const ch of text) {
		if (ch === "`") {
			inCode = !inCode;
			continue;
		}
		if (inCode || !LETTER_RE.test(ch)) continue;
		if (RTL_RE.test(ch)) rtl++;
		else ltr++;
	}
	if (rtl === 0) return "ltr";
	return rtl / (rtl + ltr) >= 0.3 ? "rtl" : "ltr";
}
