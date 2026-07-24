import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";

const target = document.querySelector("#app");
if (!(target instanceof HTMLElement)) {
  throw new Error("Jet Black application mount point is missing.");
}

mount(App, { target });
