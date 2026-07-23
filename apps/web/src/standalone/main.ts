import { mount } from "svelte";
import StandaloneApp from "./StandaloneApp.svelte";
import "./app.css";

const target = document.querySelector("#app");
if (!(target instanceof HTMLElement)) {
  throw new Error("Standalone application mount point is missing.");
}

mount(StandaloneApp, { target });
