<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import Seo from '$lib/components/Seo.svelte';
  import { localeHref } from '$lib/i18n/href';
  import { OPERATOR, operatorComplete } from '$lib/legal';
  import { ArrowLeft } from 'lucide-svelte';

  const isEs = $derived(String($locale ?? 'en').toLowerCase().startsWith('es'));
  const courtesy = $derived(!['en', 'es'].includes(String($locale ?? 'en').slice(0, 2)));
  const complete = operatorComplete();
</script>

<Seo path="/legal/notice" key="notice" />

<section class="mx-auto max-w-3xl px-4 pb-16 pt-16 sm:px-6 sm:pb-20 sm:pt-20 2xl:pt-12">
  <a
    href={$localeHref('/')}
    class="ring-focus inline-flex items-center gap-1.5 text-sm text-ink-soft transition-colors hover:text-ink"
  >
    <ArrowLeft class="h-4 w-4" />
    {$_('legal.back_home')}
  </a>

  <h1 class="mt-6 text-balance text-4xl font-semibold tracking-tight text-ink sm:text-5xl">
    {$_('legal.notice_title')}
  </h1>
  <p class="mt-3 text-sm text-ink-faint">{$_('legal.last_updated')}</p>

  {#if courtesy}
    <p class="mt-5 rounded-lg border border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm text-amber-300">
      {$_('legal.courtesy_notice')}
    </p>
  {/if}

  <article
    class="prose-legal mt-10 space-y-6 text-[15px] leading-relaxed text-ink-soft [&_h2]:mt-10 [&_h2]:text-xl [&_h2]:font-semibold [&_h2]:text-ink [&_a]:text-accent [&_a]:underline [&_a:hover]:text-emerald-300 [&_strong]:text-ink [&_ul]:list-disc [&_ul]:pl-6 [&_ul]:space-y-1.5 [&_li]:marker:text-ink-faint [&_table]:w-full [&_table]:text-sm [&_th]:text-left [&_th]:font-semibold [&_th]:text-ink [&_th]:py-2 [&_th]:pr-4 [&_td]:py-2 [&_td]:pr-4 [&_td]:text-ink-soft [&_tr]:border-t [&_tr]:border-line"
  >
    {#if isEs}
      <p>
        En cumplimiento del artículo 10 de la Ley 34/2002, de 11 de julio, de Servicios de la
        Sociedad de la Información y de Comercio Electrónico (LSSI-CE), se hacen constar los
        siguientes datos identificativos del prestador del servicio.
      </p>

      <h2>1. Titular</h2>
      <table>
        <tbody>
          <tr><th>Titular</th><td>{OPERATOR.name}</td></tr>
          <tr>
            <th>NIF</th>
            <td>{complete ? OPERATOR.taxId : 'Pendiente de publicación'}</td>
          </tr>
          <tr>
            <th>Domicilio</th>
            <td>{complete ? OPERATOR.address : 'Pendiente de publicación'}</td>
          </tr>
          <tr>
            <th>Correo electrónico</th>
            <td><a href="mailto:{OPERATOR.email}">{OPERATOR.email}</a></td>
          </tr>
          <tr><th>Sitio web</th><td><a href={OPERATOR.site}>hoard.services</a></td></tr>
        </tbody>
      </table>
      <p>
        El titular es una persona física, por lo que no consta inscripción en Registro
        Mercantil. La actividad no está sujeta a régimen de autorización previa ni a colegiación
        profesional.
      </p>

      <h2>2. Objeto</h2>
      <p>
        Este sitio web da a conocer y permite contratar Hoard, un servicio de sincronización y
        copia versionada de partidas guardadas de videojuegos. Las condiciones de contratación y
        uso figuran en los <a href={$localeHref('/legal/terms')}>Términos del Servicio</a>, y el
        tratamiento de datos personales en la
        <a href={$localeHref('/legal/privacy')}>Política de Privacidad</a>.
      </p>

      <h2>3. Propiedad intelectual e industrial</h2>
      <p>
        La marca Hoard, el logotipo, el diseño y los textos de este sitio pertenecen a su
        titular. El código fuente del software se distribuye bajo licencia AGPL-3.0 y está
        disponible en
        <a href="https://github.com/rleeon/hoard" target="_blank" rel="noopener">GitHub</a>. El
        catálogo de rutas de guardado incluido en la aplicación procede de PCGamingWiki a través
        del manifiesto de Ludusavi y se distribuye bajo licencia CC BY-NC-SA 3.0.
      </p>

      <h2>4. Responsabilidad</h2>
      <p>
        El titular no se hace responsable del contenido de los sitios de terceros enlazados
        desde este sitio. El uso del servicio se rige por sus Términos, que incluyen las
        exclusiones y limitaciones de responsabilidad aplicables.
      </p>

      <h2>5. Legislación aplicable</h2>
      <p>
        Este aviso legal se rige por la legislación española. Para reclamaciones de consumo,
        la Comisión Europea ofrece una plataforma de resolución de litigios en línea en
        <a href="https://ec.europa.eu/consumers/odr" target="_blank" rel="noopener"
          >ec.europa.eu/consumers/odr</a
        >.
      </p>
    {:else}
      <p>
        In compliance with article 10 of Spanish Law 34/2002 on Information Society Services and
        Electronic Commerce (LSSI-CE), the following identification details of the service
        provider are published.
      </p>

      <h2>1. Provider</h2>
      <table>
        <tbody>
          <tr><th>Provider</th><td>{OPERATOR.name}</td></tr>
          <tr>
            <th>Tax ID (NIF)</th>
            <td>{complete ? OPERATOR.taxId : 'Pending publication'}</td>
          </tr>
          <tr>
            <th>Registered address</th>
            <td>{complete ? OPERATOR.address : 'Pending publication'}</td>
          </tr>
          <tr>
            <th>Email</th>
            <td><a href="mailto:{OPERATOR.email}">{OPERATOR.email}</a></td>
          </tr>
          <tr><th>Website</th><td><a href={OPERATOR.site}>hoard.services</a></td></tr>
        </tbody>
      </table>
      <p>
        The provider is a natural person and is therefore not entered in the Commercial
        Registry. The activity is not subject to prior authorisation or professional
        association membership.
      </p>

      <h2>2. Purpose</h2>
      <p>
        This website presents and allows you to subscribe to Hoard, a service for syncing and
        versioning video game save files. Contracting and usage conditions are set out in the
        <a href={$localeHref('/legal/terms')}>Terms of Service</a>, and the processing of
        personal data in the <a href={$localeHref('/legal/privacy')}>Privacy Policy</a>.
      </p>

      <h2>3. Intellectual and industrial property</h2>
      <p>
        The Hoard mark, logo, design and copy of this site belong to its owner. The software's
        source code is distributed under the AGPL-3.0 licence and is available on
        <a href="https://github.com/rleeon/hoard" target="_blank" rel="noopener">GitHub</a>. The
        save-path catalogue bundled with the application comes from PCGamingWiki via the
        Ludusavi manifest and is distributed under the CC BY-NC-SA 3.0 licence.
      </p>

      <h2>4. Liability</h2>
      <p>
        The provider is not responsible for the content of third-party sites linked from this
        site. Use of the service is governed by its Terms, which include the applicable
        exclusions and limitations of liability.
      </p>

      <h2>5. Applicable law</h2>
      <p>
        This legal notice is governed by Spanish law. For consumer complaints, the European
        Commission provides an online dispute resolution platform at
        <a href="https://ec.europa.eu/consumers/odr" target="_blank" rel="noopener"
          >ec.europa.eu/consumers/odr</a
        >.
      </p>
    {/if}
  </article>
</section>
