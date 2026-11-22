// A one-second clock for elapsed times ("Running tests · 1m 20s"). Ticks
// only while something on screen shows a live time.

class Clock {
	now = $state(Date.now());
	private users = 0;
	private timer: ReturnType<typeof setInterval> | null = null;

	subscribe(): () => void {
		this.users++;
		this.now = Date.now();
		if (!this.timer) this.timer = setInterval(() => (this.now = Date.now()), 1000);
		return () => {
			this.users--;
			if (this.users === 0 && this.timer) {
				clearInterval(this.timer);
				this.timer = null;
			}
		};
	}
}

export const clock = new Clock();
