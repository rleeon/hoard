<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { marquee } from '$lib/actions/marquee';

  let { class: className = '' }: { class?: string } = $props();

  // Hoard runs no model of its own; these are examples of the agents people
  // bring, not an allow-list. Anything that can run a command works. The marks
  // are single-colour copies in /static/agents (lobe-icons, MIT, and
  // simple-icons, CC0; each file says which). Ollama and LM Studio are not
  // agents but the local models one runs on, hence their tag.
  type Agent = { slug: string; name: string; tag?: 'open' | 'local' };
  const AGENTS: Agent[] = [
    { slug: 'opencode', name: 'opencode', tag: 'open' },
    { slug: 'claude-code', name: 'Claude Code' },
    { slug: 'codex', name: 'Codex', tag: 'open' },
    { slug: 'gemini-cli', name: 'Gemini CLI', tag: 'open' },
    { slug: 'cursor', name: 'Cursor' },
    { slug: 'goose', name: 'Goose', tag: 'open' },
    { slug: 'github-copilot', name: 'GitHub Copilot' },
    { slug: 'ollama', name: 'Ollama', tag: 'local' },
    { slug: 'cline', name: 'Cline', tag: 'open' },
    { slug: 'windsurf', name: 'Windsurf' },
    { slug: 'roo-code', name: 'Roo Code', tag: 'open' },
    { slug: 'zed', name: 'Zed', tag: 'open' },
    { slug: 'amp', name: 'Amp' },
    { slug: 'kilo-code', name: 'Kilo Code', tag: 'open' },
    { slug: 'warp', name: 'Warp' },
    { slug: 'openhands', name: 'OpenHands', tag: 'open' },
    { slug: 'kiro', name: 'Kiro' },
    { slug: 'lm-studio', name: 'LM Studio', tag: 'local' },
    { slug: 'qwen-code', name: 'Qwen Code', tag: 'open' },
    { slug: 'junie', name: 'Junie' },
    { slug: 'trae', name: 'Trae' },
    { slug: 'antigravity', name: 'Antigravity' },
    { slug: 'qoder', name: 'Qoder' }
  ];
  // One copy of the list is ~5k px, wider than a 4K panel, so two are enough
  // for the wrap (see the fact strip on the home).
  const COPIES = [0, 1];
</script>

<!-- The home's fact strip, carrying agents instead of facts. -->
<div class={className} role="region" aria-label={$_('ai.agents_aria')}>
  <div class="marquee">
    <div class="marquee-inner" use:marquee={{ speed: 40 }}>
      {#each COPIES as copy (copy)}
        <ul class="flex shrink-0 items-center" aria-hidden={copy > 0 || undefined}>
          {#each AGENTS as a (a.slug)}
            <li class="flex shrink-0 items-center gap-2.5 whitespace-nowrap px-7 py-4">
              <img
                src={`/agents/${a.slug}.svg`}
                alt=""
                width="28"
                height="28"
                draggable="false"
                class="h-6 w-6"
              />
              <span class="text-base font-medium text-ink">{a.name}</span>
              {#if a.tag}
                <span
                  class="rounded-full border border-white/15 px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider text-ink-faint"
                  >{$_(`ai.tag_${a.tag}`)}</span
                >
              {/if}
            </li>
          {/each}
        </ul>
      {/each}
    </div>
  </div>
</div>
