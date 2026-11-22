import { describe, expect, it } from "vitest";
import { maskUrl, isSocks, proxies } from "./proxies.svelte";

const settings = {
	proxies: [
		{ id: "a", name: "Office", url: "http://u:secret@office:8080", noProxy: "" },
		{ id: "b", name: "Home", url: "socks5://home:1080", noProxy: "" },
	],
	appDefault: "a",
	byAccount: { "codex:work": "b", "claude:solo": "none" },
	byProject: { "/p/app": "none", "/p/app/deep": "b" },
	terminals: true,
};

describe("proxies", () => {
	it("hides the password", () => {
		expect(maskUrl("http://u:secret@office:8080")).toBe("http://u:•••@office:8080");
		expect(maskUrl("http://office:8080")).toBe("http://office:8080");
		expect(isSocks("SOCKS5://h:1")).toBe(true);
	});

	it("resolves conversation, project, account, then app default", () => {
		proxies.s = structuredClone(settings);
		// Its own choice wins.
		expect(proxies.resolve({ provider: "codex", account: "work", cwd: "/p/app" }, "a")).toMatchObject({ id: "a", source: "conversation" });
		expect(proxies.resolve({ provider: "codex", account: "work", cwd: "/x" }, "none")).toMatchObject({ id: null, source: "conversation" });
		// The closest project folder.
		expect(proxies.resolve({ provider: "claude", account: "default", cwd: "/p/app/deep/src" })).toMatchObject({ id: "b", source: "project" });
		expect(proxies.resolve({ provider: "claude", account: "default", cwd: "/p/app/x" })).toMatchObject({ id: null, source: "project" });
		// Not a prefix match on a sibling folder.
		expect(proxies.projectRule("/p/application")).toBeUndefined();
		// The account, then the app default.
		expect(proxies.resolve({ provider: "codex", account: "work", cwd: "/x" })).toMatchObject({ id: "b", source: "account" });
		expect(proxies.resolve({ provider: "claude", account: "solo", cwd: "/x" })).toMatchObject({ id: null, source: "account" });
		expect(proxies.resolve({ provider: "claude", account: "other", cwd: "/x" })).toMatchObject({ id: "a", source: "app" });
	});

	it("gives shells the default only when terminals use the proxy", () => {
		proxies.s = { ...structuredClone(settings), terminals: false };
		expect(proxies.forShell("/x")).toBeNull();
		proxies.s = structuredClone(settings);
		expect(proxies.forShell("/x")).toBe("a");
		expect(proxies.forShell("/p/app")).toBeNull();
	});
});
