import { beforeAll, describe, expect, it } from "vitest";

// The store saves to localStorage; tests run without a browser.
const mem = new Map<string, string>();
beforeAll(() => {
	(globalThis as { localStorage?: unknown }).localStorage = {
		getItem: (k: string) => mem.get(k) ?? null,
		setItem: (k: string, v: string) => void mem.set(k, v),
		removeItem: (k: string) => void mem.delete(k),
	};
});

describe("projects", () => {
	it("finds the project with the closest folder", async () => {
		const { projects } = await import("./projects.svelte");
		const shop = projects.add({ name: "Shop", folders: ["/p/shop", "/p/shop-api"], defaults: {} });
		const deep = projects.add({ name: "Docs", folders: ["/p/shop/docs"], defaults: {} });
		expect(projects.of("/p/shop/src")?.id).toBe(shop.id);
		expect(projects.of("/p/shop-api")?.id).toBe(shop.id);
		expect(projects.of("/p/shop/docs/a")?.id).toBe(deep.id);
		// A sibling folder with the same prefix is not inside.
		expect(projects.of("/p/shopping")).toBeNull();
	});

	it("moves projects and restores a removed one", async () => {
		const { projects } = await import("./projects.svelte");
		const names = () => projects.list.map((p) => p.name);
		const c = projects.add({ name: "Blog", folders: ["/p/blog"], defaults: {} });
		expect(names()).toEqual(["Shop", "Docs", "Blog"]);
		projects.move(c.id, 0);
		expect(names()).toEqual(["Blog", "Shop", "Docs"]);
		projects.move(c.id, 3);
		expect(names()).toEqual(["Shop", "Docs", "Blog"]);
		const gone = projects.remove(projects.list[1].id)!;
		expect(names()).toEqual(["Shop", "Blog"]);
		projects.restore(gone, 1);
		expect(names()).toEqual(["Shop", "Docs", "Blog"]);
		expect(JSON.parse(mem.get("glaude.projects.v1")!).list).toHaveLength(3);
	});

	it("sorts sessions", async () => {
		const { sortSessions } = await import("./projects.svelte");
		const list = [
			{ title: "b", updated: 3, created: 1 },
			{ title: "A", updated: 1, created: 3 },
			{ title: "c", updated: 2 },
		];
		expect(sortSessions(list, "recent").map((s) => s.title)).toEqual(["b", "c", "A"]);
		expect(sortSessions(list, "created").map((s) => s.title)).toEqual(["A", "c", "b"]);
		expect(sortSessions(list, "name").map((s) => s.title)).toEqual(["A", "b", "c"]);
	});
});
