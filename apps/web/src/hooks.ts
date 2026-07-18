import {
  decodeConvexLoad,
  encodeConvexLoad,
  initConvex,
} from "convex-svelte/sveltekit";
import { PUBLIC_CONVEX_URL } from "$env/static/public";

initConvex(PUBLIC_CONVEX_URL);

export const transport = {
  ConvexLoadResult: {
    encode: encodeConvexLoad,
    decode: decodeConvexLoad,
  },
};
