// Small screens: modals and panels become bottom sheets.
import { MediaQuery } from "svelte/reactivity";

export const narrow = new MediaQuery("max-width: 640px");
