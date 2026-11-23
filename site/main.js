// Download buttons: the right file for this computer, from the latest release.
(() => {
	const REPO = "farvisun/glaude";
	const ua = navigator.userAgent;
	const os = /Mac/i.test(ua) ? "mac" : /Win/i.test(ua) ? "win" : /Linux|X11/i.test(ua) && !/Android/i.test(ua) ? "linux" : null;
	const names = { mac: "Download for macOS", win: "Download for Windows", linux: "Download for Linux" };

	const primary = document.querySelector("[data-primary-download]");
	const primaryLabel = document.querySelector("[data-primary-label]");
	if (os && primaryLabel) primaryLabel.textContent = names[os];
	if (os) document.querySelector(`.dl[data-os="${os}"]`)?.classList.add("current");

	const patterns = {
		dmg: /\.dmg$/i,
		exe: /-setup\.exe$/i,
		msi: /\.msi$/i,
		appimage: /\.AppImage$/i,
		deb: /\.deb$/i,
		rpm: /\.rpm$/i,
	};
	const primaryAsset = { mac: "dmg", win: "exe", linux: "appimage" };

	fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { headers: { Accept: "application/vnd.github+json" } })
		.then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
		.then((release) => {
			const assets = release.assets || [];
			const find = (kind) => assets.find((a) => patterns[kind].test(a.name));
			for (const el of document.querySelectorAll("[data-asset]")) {
				const a = find(el.dataset.asset);
				if (a) {
					el.href = a.browser_download_url;
					el.title = `${a.name} · ${(a.size / 1048576).toFixed(1)} MB`;
				} else if (!el.classList.contains("btn")) {
					// No such package in this release: hide the small link
					// (and the whole "Also as" line when nothing is left).
					el.hidden = true;
					const sep = el.previousElementSibling?.classList.contains("sep") ? el.previousElementSibling : el.nextElementSibling;
					if (sep?.classList.contains("sep")) sep.hidden = true;
					const line = el.closest(".alts");
					if (line && !line.querySelector("a:not([hidden])")) line.hidden = true;
				}
			}
			const version = (release.tag_name || "").replace(/^v/, "");
			if (version) {
				for (const el of document.querySelectorAll("[data-version]")) el.textContent = version;
				const label = document.querySelector("[data-release-label]");
				if (label) label.textContent = `Version ${version} is out`;
				const pill = document.querySelector("[data-release-pill]");
				if (pill) pill.href = release.html_url;
			}
			if (os && primary) {
				const a = find(primaryAsset[os]);
				if (a) primary.href = a.browser_download_url;
			}
		})
		.catch(() => {
			/* Offline or rate-limited: the links keep pointing at the releases page. */
		});
})();
