// Code blocks in replies get a bar with the language, a wrap toggle and a
// copy button; long ones are folded with "Show all N lines". The bar lives
// inside the <pre> so it goes away whenever Svelte replaces the block.

const FOLD_LINES = 24;

function enhance(root: HTMLElement) {
	for (const pre of root.querySelectorAll<HTMLPreElement>("pre:not([data-cb])")) {
		const code = pre.querySelector("code");
		if (!code) continue;
		pre.dataset.cb = "1";
		const lang = /language-([\w+#-]+)/.exec(code.className)?.[1] ?? "";
		const bar = document.createElement("div");
		bar.className = "cb-bar";
		bar.setAttribute("contenteditable", "false");
		const label = document.createElement("span");
		label.className = "cb-lang";
		label.textContent = lang || "text";
		const wrap = document.createElement("button");
		wrap.type = "button";
		wrap.className = "cb-btn";
		wrap.textContent = "Wrap";
		wrap.title = "Wrap long lines";
		wrap.onclick = () => {
			const on = pre.classList.toggle("cb-wrap");
			wrap.setAttribute("aria-pressed", String(on));
		};
		const copy = document.createElement("button");
		copy.type = "button";
		copy.className = "cb-btn";
		copy.textContent = "Copy";
		copy.onclick = () => {
			void navigator.clipboard?.writeText(code.textContent ?? "").then(() => {
				copy.textContent = "Copied";
				setTimeout(() => (copy.textContent = "Copy"), 1200);
			});
		};
		bar.append(label, wrap, copy);
		pre.prepend(bar);
		const lines = (code.textContent ?? "").split("\n").length;
		if (lines > FOLD_LINES + 4) {
			pre.classList.add("cb-long");
			const more = document.createElement("button");
			more.type = "button";
			more.className = "cb-more";
			more.textContent = `Show all ${lines} lines`;
			more.onclick = () => {
				const open = pre.classList.toggle("cb-open");
				more.textContent = open ? "Show less" : `Show all ${lines} lines`;
			};
			pre.append(more);
		}
	}
}

export function codeBlocks(node: HTMLElement, _html: string) {
	enhance(node);
	return {
		update() {
			enhance(node);
		},
	};
}
