// i18n must be imported first so registrations and `init()` happen before any
// component subscribes to `$_`. The module has top-level side effects.
import { i18nReady } from "./lib/i18n";
import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

// Wait for svelte-i18n to finish loading the active locale's dictionary
// before mounting. If we mount eagerly, the first render hits `$_(...)`
// while no messages are loaded, svelte-i18n throws, and Svelte unwinds —
// leaving the user with a blank, body-coloured window. (See v1.2.1 bug.)
async function bootstrap() {
  await i18nReady;
  return mount(App, {
    target: document.getElementById("app")!,
  });
}

const app = bootstrap();

export default app;
