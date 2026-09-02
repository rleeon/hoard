<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import Seo from '$lib/components/Seo.svelte';
  import { localeHref } from '$lib/i18n/href';
  import { ArrowLeft } from 'lucide-svelte';

  const isEs = $derived(String($locale ?? 'en').toLowerCase().startsWith('es'));
  const courtesy = $derived(!['en', 'es'].includes(String($locale ?? 'en').slice(0, 2)));

  /**
   * The live sub-processor list. Kept as data rather than markup so both
   * languages render the same rows and nobody has to remember to edit two
   * tables when a provider changes.
   *
   * `since` is the date the provider started processing data for the Service,
   * that is what a customer needs in order to check whether they were given
   * the notice period before a new one came in.
   */
  const ROWS = [
    {
      name: 'Supabase Pte Ltd',
      purpose: { es: 'Autenticación y base de datos de metadatos', en: 'Authentication and metadata database' },
      data: { es: 'Correo, identificadores, metadatos de partidas', en: 'Email, identifiers, save metadata' },
      location: { es: 'Fráncfort, Alemania (AWS)', en: 'Frankfurt, Germany (AWS)' },
      since: '2026-01'
    },
    {
      name: 'Cloudflare, Inc. — R2',
      purpose: { es: 'Almacenamiento de instantáneas', en: 'Snapshot storage' },
      data: { es: 'Contenido de las partidas, cifrado en reposo', en: 'Save contents, encrypted at rest' },
      location: { es: 'Unión Europea', en: 'European Union' },
      since: '2026-01'
    },
    {
      name: 'Cloudflare, Inc. — Turnstile',
      purpose: { es: 'Captcha del inicio de sesión web', en: 'Web sign-in captcha' },
      data: { es: 'Dirección IP y señales del navegador', en: 'IP address and browser signals' },
      location: { es: 'Red global de Cloudflare', en: 'Cloudflare global network' },
      since: '2026-03'
    },
    {
      name: 'Fly.io, Inc.',
      purpose: { es: 'Alojamiento de la API', en: 'API hosting' },
      data: { es: 'Todo el tráfico de la API en tránsito', en: 'All API traffic in transit' },
      location: { es: 'París, Francia', en: 'Paris, France' },
      since: '2026-01'
    },
    {
      name: 'Microsoft Corp. — GitHub Pages',
      purpose: { es: 'Alojamiento del sitio público', en: 'Public site hosting' },
      data: { es: 'Registros de acceso al sitio', en: 'Site access logs' },
      location: { es: 'Red CDN', en: 'CDN network' },
      since: '2026-01'
    },
    {
      name: 'Polar Software Inc.',
      purpose: { es: 'Comerciante Registrado: cobro, IVA, facturación', en: 'Merchant of Record: charging, VAT, invoicing' },
      data: { es: 'Datos de pago y facturación', en: 'Payment and billing data' },
      location: { es: 'Estados Unidos', en: 'United States' },
      since: '2026-04'
    },
    {
      name: 'Resend, Inc.',
      purpose: { es: 'Correo transaccional', en: 'Transactional email' },
      data: { es: 'Dirección de correo y contenido del mensaje', en: 'Email address and message content' },
      location: { es: 'Unión Europea', en: 'European Union' },
      since: '2026-06'
    }
  ];
</script>

<Seo path="/legal/subprocessors" key="subprocessors" />

<section class="mx-auto max-w-4xl px-4 py-16 sm:px-6 sm:py-20">
  <a
    href={$localeHref('/')}
    class="ring-focus inline-flex items-center gap-1.5 text-sm text-ink-soft transition-colors hover:text-ink"
  >
    <ArrowLeft class="h-4 w-4" />
    {$_('legal.back_home')}
  </a>

  <h1 class="mt-6 text-balance text-4xl font-semibold tracking-tight text-ink sm:text-5xl">
    {$_('legal.subprocessors_title')}
  </h1>
  <p class="mt-3 text-sm text-ink-faint">{$_('legal.last_updated')}</p>

  {#if courtesy}
    <p class="mt-5 rounded-lg border border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm text-amber-300">
      {$_('legal.courtesy_notice')}
    </p>
  {/if}

  <article
    class="prose-legal mt-10 space-y-6 text-[15px] leading-relaxed text-ink-soft [&_h2]:mt-10 [&_h2]:text-xl [&_h2]:font-semibold [&_h2]:text-ink [&_a]:text-accent [&_a]:underline [&_a:hover]:text-emerald-300 [&_strong]:text-ink"
  >
    {#if isEs}
      <p>
        Esta es la lista completa de proveedores que tratan datos personales por cuenta nuestra
        para operar Hoard. Forma parte de la
        <a href={$localeHref('/legal/privacy')}>Política de Privacidad</a> y se mantiene
        actualizada aquí para que puedas consultarla en cualquier momento.
      </p>
      <p>
        <strong>Aviso de cambios:</strong> antes de incorporar un nuevo sub-encargado que trate
        contenido de usuarios, lo anunciaremos en esta página y por correo electrónico con al
        menos <strong>30 días</strong> de antelación. Si no estás conforme, podrás cancelar y
        exportar tus datos antes de que el cambio entre en vigor, con reembolso proporcional si
        tienes suscripción de pago.
      </p>
    {:else}
      <p>
        This is the complete list of providers that process personal data on our behalf to
        operate Hoard. It forms part of the
        <a href={$localeHref('/legal/privacy')}>Privacy Policy</a> and is kept current here so
        you can check it at any time.
      </p>
      <p>
        <strong>Change notice:</strong> before adding a new sub-processor that handles user
        content, we will announce it on this page and by email at least
        <strong>30 days</strong> in advance. If you are not comfortable with it, you may cancel
        and export your data before the change takes effect, with a pro-rata refund if you are
        a paying subscriber.
      </p>
    {/if}

    <div class="overflow-x-auto">
      <table class="w-full min-w-[42rem] text-sm">
        <thead>
          <tr class="border-b border-line">
            <th class="py-2 pr-4 text-left font-semibold text-ink">
              {isEs ? 'Proveedor' : 'Provider'}
            </th>
            <th class="py-2 pr-4 text-left font-semibold text-ink">
              {isEs ? 'Función' : 'Purpose'}
            </th>
            <th class="py-2 pr-4 text-left font-semibold text-ink">
              {isEs ? 'Datos tratados' : 'Data processed'}
            </th>
            <th class="py-2 pr-4 text-left font-semibold text-ink">
              {isEs ? 'Ubicación' : 'Location'}
            </th>
            <th class="py-2 text-left font-semibold text-ink">{isEs ? 'Desde' : 'Since'}</th>
          </tr>
        </thead>
        <tbody>
          {#each ROWS as row (row.name)}
            <tr class="border-t border-line align-top">
              <td class="py-2.5 pr-4 text-ink">{row.name}</td>
              <td class="py-2.5 pr-4">{isEs ? row.purpose.es : row.purpose.en}</td>
              <td class="py-2.5 pr-4">{isEs ? row.data.es : row.data.en}</td>
              <td class="py-2.5 pr-4">{isEs ? row.location.es : row.location.en}</td>
              <td class="py-2.5 text-ink-faint">{row.since}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    {#if isEs}
      <h2>Auto-alojamiento</h2>
      <p>
        Si ejecutas Hoard contra tu propio servidor, ninguno de estos proveedores interviene:
        el responsable del tratamiento eres tú y los sub-encargados son los que tú elijas.
      </p>
      <h2>Contacto</h2>
      <p>
        Para cualquier duda sobre esta lista o para solicitar copia de los acuerdos de encargo:
        <a href="mailto:support@hoard.services">support@hoard.services</a>.
      </p>
    {:else}
      <h2>Self-hosting</h2>
      <p>
        If you run Hoard against your own server, none of these providers is involved: you are
        the controller and the sub-processors are whichever ones you choose.
      </p>
      <h2>Contact</h2>
      <p>
        For any question about this list or to request a copy of the processing agreements:
        <a href="mailto:support@hoard.services">support@hoard.services</a>.
      </p>
    {/if}
  </article>
</section>
