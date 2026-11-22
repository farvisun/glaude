import { describe, expect, it } from "vitest";
import { detectDir } from "./bidi";

describe("detectDir", () => {
	it("english is ltr", () => expect(detectDir("Hello world")).toBe("ltr"));
	it("persian is rtl", () => expect(detectDir("سلام دنیا")).toBe("rtl"));
	it("persian starting with a latin word is rtl", () =>
		expect(detectDir("Claude یک دستیار هوش مصنوعی است که کد می‌نویسد")).toBe("rtl"));
	it("english with one persian word stays ltr", () =>
		expect(detectDir("The word سلام means hello in Persian and is used everywhere")).toBe("ltr"));
	it("ignores code spans", () => expect(detectDir("فایل `src/main.rs` و `Cargo.toml` را ببین")).toBe("rtl"));
	it("empty is ltr", () => expect(detectDir("")).toBe("ltr"));
});
