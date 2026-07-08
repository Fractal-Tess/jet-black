import "@workspace/ui/globals.css";
import { mount } from "svelte";
import App from "./App.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("Unable to find the desktop app mount element.");
}

mount(App, { target });
