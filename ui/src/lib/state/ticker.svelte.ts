// One shared clock for spinners, so every spinner on screen moves in step
// and only one timer runs however many are visible.

const FRAME_MS = 120;

class Ticker {
	frame = $state(0);
	private users = 0;
	private timer: ReturnType<typeof setInterval> | null = null;

	/** Start ticking while at least one subscriber is mounted. */
	subscribe(): () => void {
		this.users++;
		if (!this.timer) this.timer = setInterval(() => this.frame++, FRAME_MS);
		return () => {
			this.users--;
			if (this.users === 0 && this.timer) {
				clearInterval(this.timer);
				this.timer = null;
			}
		};
	}
}

export const ticker = new Ticker();

export const reducedMotion = () =>
	typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
