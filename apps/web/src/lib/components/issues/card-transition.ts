import { cubicOut } from "svelte/easing";
import { crossfade } from "svelte/transition";

const MOVE_DURATION = 250;
const FADE_DURATION = 150;

export const [sendCard, receiveCard] = crossfade({
  duration: MOVE_DURATION,
  easing: cubicOut,
  fallback(_node, _params, intro) {
    return {
      css: (t) => `opacity: ${t}; transform: scale(${0.97 + 0.03 * t})`,
      duration: FADE_DURATION,
      easing: cubicOut,
      delay: intro ? FADE_DURATION : 0,
    };
  },
});
