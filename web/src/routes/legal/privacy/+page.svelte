<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import { ArrowLeft } from 'lucide-svelte';

  const isEs = $derived(String($locale ?? 'en').toLowerCase().startsWith('es'));
</script>

<svelte:head>
  <title>{$_('legal.privacy_title')} — Hoard</title>
  <meta name="robots" content="index,follow" />
</svelte:head>

<section class="mx-auto max-w-3xl px-4 py-16 sm:px-6 sm:py-20">
  <a
    href="/"
    class="inline-flex items-center gap-1.5 text-sm text-zinc-400 transition-colors hover:text-white"
  >
    <ArrowLeft class="h-4 w-4" />
    {$_('legal.back_home')}
  </a>

  <h1 class="mt-6 text-balance text-4xl font-bold tracking-tight text-white sm:text-5xl">
    {$_('legal.privacy_title')}
  </h1>
  <p class="mt-3 text-sm text-zinc-500">{$_('legal.last_updated')}</p>

  <article
    class="prose-legal mt-10 space-y-6 text-[15px] leading-relaxed text-zinc-300 [&_h2]:mt-10 [&_h2]:text-xl [&_h2]:font-semibold [&_h2]:text-white [&_h3]:mt-6 [&_h3]:text-base [&_h3]:font-semibold [&_h3]:text-zinc-100 [&_a]:text-emerald-400 [&_a]:underline [&_a:hover]:text-emerald-300 [&_strong]:text-white [&_ul]:list-disc [&_ul]:pl-6 [&_ul]:space-y-1.5 [&_li]:marker:text-zinc-600 [&_table]:w-full [&_table]:text-sm [&_th]:text-left [&_th]:font-semibold [&_th]:text-white [&_th]:py-2 [&_th]:pr-4 [&_td]:py-2 [&_td]:pr-4 [&_td]:text-zinc-400 [&_tr]:border-t [&_tr]:border-zinc-800"
  >
    {#if isEs}
      <p>
        Esta Política explica qué datos personales recogemos sobre ti cuando usas Hoard,
        para qué los usamos, con quién los compartimos y qué derechos tienes. El
        responsable del tratamiento es <strong>Raimundo León Oliva</strong>, persona física
        residente en España, fundador y operador de la plataforma Hoard, contactable en
        <a href="mailto:support@hoard.services">support@hoard.services</a>.
      </p>

      <h2>1. Qué datos recogemos</h2>
      <p>
        <strong>Datos de cuenta.</strong> Tu dirección de correo electrónico, identificador
        único de cuenta, fecha de registro y, si inicias sesión con Google, el identificador
        de Google asociado. No recogemos tu nombre real salvo que tú lo introduzcas
        voluntariamente.
      </p>
      <p>
        <strong>Datos de uso del servicio.</strong> Snapshots de saves que subes (cifrados
        en reposo en el almacenamiento), nombres de juegos, identificador de dispositivo
        generado por la app, marcas de tiempo de operaciones, tamaño y hash SHA-256 de cada
        snapshot.
      </p>
      <p>
        <strong>Datos técnicos.</strong> Dirección IP, agente de usuario y versión de la
        aplicación cuando contactas con la API, retenidos hasta 30 días en logs operativos
        para diagnóstico y prevención de abuso.
      </p>
      <p>
        <strong>Logs de diagnóstico de la app.</strong> La aplicación de escritorio envía a
        nuestro servidor eventos de diagnóstico de nivel informativo o superior (no
        depuración), identificados por nombre de dispositivo y un identificador estable
        generado por la app, para diagnosticar fallos de sincronización. Se conservan 14
        días y luego se borran automáticamente. En instalaciones autoalojadas estos logs
        residen únicamente en tu propio servidor, sin terceros.
      </p>
      <p>
        <strong>Datos de facturación.</strong> Los procesa íntegramente nuestro Merchant
        of Record. Sólo recibimos confirmación de estado de suscripción (plan, periodo,
        próxima renovación, cancelación). No vemos ni almacenamos tu tarjeta ni tu
        dirección postal.
      </p>

      <h2>2. Para qué los usamos y base legal</h2>
      <table>
        <thead>
          <tr><th>Finalidad</th><th>Base legal</th></tr>
        </thead>
        <tbody>
          <tr><td>Crear y gestionar tu cuenta, autenticarte</td><td>Ejecución de contrato (art. 6.1.b RGPD)</td></tr>
          <tr><td>Almacenar y sincronizar tus saves</td><td>Ejecución de contrato</td></tr>
          <tr><td>Procesar tu suscripción Pro</td><td>Ejecución de contrato</td></tr>
          <tr><td>Diagnóstico, prevención de abuso, seguridad</td><td>Interés legítimo (art. 6.1.f)</td></tr>
          <tr><td>Cumplir obligaciones legales (fiscales, requerimientos)</td><td>Obligación legal (art. 6.1.c)</td></tr>
          <tr><td>Comunicaciones de servicio (avisos críticos)</td><td>Ejecución de contrato</td></tr>
        </tbody>
      </table>

      <h2>3. Sub-encargados del tratamiento</h2>
      <p>
        Para operar el servicio recurrimos a los siguientes proveedores. Todos están
        sujetos a un contrato de encargo del tratamiento conforme al art. 28 RGPD y
        procesan datos exclusivamente en la Unión Europea.
      </p>
      <table>
        <thead>
          <tr><th>Proveedor</th><th>Función</th><th>Ubicación</th></tr>
        </thead>
        <tbody>
          <tr><td>Supabase (Singapore Pte Ltd, infraestructura en AWS Frankfurt)</td><td>Autenticación y base de datos de metadatos</td><td>Fráncfort, Alemania</td></tr>
          <tr><td>Cloudflare R2 (Cloudflare Inc., jurisdicción EU)</td><td>Almacenamiento de snapshots cifrado en reposo</td><td>Unión Europea</td></tr>
          <tr><td>Fly.io (Fly.io Inc.)</td><td>Hosting de la API</td><td>París, Francia (CDG)</td></tr>
          <tr><td>GitHub Pages (Microsoft Corp.)</td><td>Hosting del sitio público</td><td>Red CDN, principalmente EU para tráfico EU</td></tr>
          <tr><td>Merchant of Record (proveedor de pagos externo)</td><td>Cobro, IVA, facturación</td><td>EU / EEA</td></tr>
          <tr><td>Resend (cuando se active)</td><td>Envío de correos transaccionales</td><td>Unión Europea</td></tr>
        </tbody>
      </table>
      <p>
        Algunos proveedores (Cloudflare, GitHub, Fly.io) tienen casa matriz en Estados
        Unidos. Su operativa para clientes europeos se realiza bajo el marco
        EU-U.S. Data Privacy Framework y cláusulas contractuales tipo aprobadas por la
        Comisión Europea. Los datos personales relevantes no se replican fuera de la UE.
      </p>

      <h2>4. Ubicación y transferencias</h2>
      <p>
        Tu cuenta, metadatos y snapshots se almacenan en la Unión Europea (Fráncfort para
        la base de datos, infraestructura de Cloudflare R2 con jurisdicción EU para los
        archivos). No replicamos datos fuera de la UE.
      </p>

      <h2>5. Retención</h2>
      <p>
        Mantenemos tus datos mientras tu cuenta esté activa. Si eliminas tu cuenta, todos
        los snapshots y metadatos asociados se borran de forma irreversible en un plazo
        máximo de 30 días. Las copias de seguridad operativas se sobrescriben en ciclos de
        30 días. Los logs técnicos se conservan hasta 30 días y los logs de diagnóstico de la
        app durante 14 días. Los registros de
        facturación se conservan durante los plazos exigidos por la legislación fiscal
        española (en general, 6 años).
      </p>

      <h2>6. Tus derechos</h2>
      <p>
        Como interesado tienes derecho a: acceder a tus datos; rectificarlos; suprimirlos
        ("derecho al olvido"); limitar el tratamiento; oponerte al tratamiento basado en
        interés legítimo; recibir tus datos en formato portable; retirar el consentimiento
        cuando éste sea la base legal. Puedes ejercer cualquiera de estos derechos
        escribiéndonos a
        <a href="mailto:support@hoard.services">support@hoard.services</a>.
      </p>
      <p>
        Muchos de estos derechos están automatizados en la app: puedes exportar todos tus
        datos en formato original desde tu cuenta, y eliminar la cuenta entera con un clic.
      </p>
      <p>
        Si consideras que hemos tratado tus datos de forma indebida, puedes presentar una
        reclamación ante la Agencia Española de Protección de Datos (AEPD) en
        <a href="https://www.aepd.es" target="_blank" rel="noopener">aepd.es</a> o ante la
        autoridad de control de tu país de residencia.
      </p>

      <h2>7. Cookies y similares</h2>
      <p>
        El sitio público (hoard.services) no usa cookies de seguimiento ni analítica de
        terceros. Sólo se usa almacenamiento local del navegador (localStorage) para
        recordar tu sesión de Supabase tras iniciar sesión. No usamos cookies publicitarias
        ni compartimos datos con redes sociales.
      </p>

      <h2>8. Menores</h2>
      <p>
        El Servicio no está dirigido a menores de 16 años (o la edad mínima de
        consentimiento digital aplicable). Si descubres que un menor de esa edad tiene una
        cuenta, contáctanos y la eliminaremos.
      </p>

      <h2>9. Seguridad</h2>
      <p>
        Los snapshots se cifran en reposo en el almacenamiento. La comunicación entre la
        app y el servidor usa TLS. Las contraseñas no se almacenan en texto plano
        (delegamos autenticación en Supabase, que aplica hashing fuerte). Auditamos
        accesos a la API mediante logs estructurados. Aun así, ninguna medida de
        seguridad es perfecta; te recomendamos no usar el servicio para datos
        legalmente sensibles.
      </p>

      <h2>10. Cambios en esta Política</h2>
      <p>
        Podemos actualizar esta Política. Cambios sustanciales se notificarán por correo
        electrónico con al menos 30 días de antelación. La fecha de "última actualización"
        al inicio refleja la versión vigente.
      </p>

      <h2>11. Contacto</h2>
      <p>
        Para cualquier cuestión sobre privacidad o ejercicio de derechos:
        <a href="mailto:support@hoard.services">support@hoard.services</a>.
      </p>
    {:else}
      <p>
        This Policy explains what personal data we collect about you when you use Hoard,
        why we use it, who we share it with and what rights you have. The data controller
        is <strong>Raimundo León Oliva</strong>, a natural person resident in Spain, founder
        and operator of the Hoard platform, reachable at
        <a href="mailto:support@hoard.services">support@hoard.services</a>.
      </p>

      <h2>1. What data we collect</h2>
      <p>
        <strong>Account data.</strong> Your email address, a unique account ID, the date
        of signup and, if you sign in with Google, the associated Google identifier. We do
        not collect your real name unless you choose to enter it.
      </p>
      <p>
        <strong>Service usage data.</strong> The save snapshots you upload (encrypted at
        rest in storage), game names, a device identifier generated by the app, operation
        timestamps, and the size and SHA-256 hash of each snapshot.
      </p>
      <p>
        <strong>Technical data.</strong> IP address, user-agent and app version when
        contacting the API, retained for up to 30 days in operational logs for diagnostics
        and abuse prevention.
      </p>
      <p>
        <strong>App diagnostic logs.</strong> The desktop app sends our server
        informational-or-higher diagnostic events (not debug), identified by device name
        and a stable app-generated identifier, so we can diagnose sync failures. They are
        kept for 14 days and then deleted automatically. On self-hosted installations these
        logs live solely on your own server, with no third parties.
      </p>
      <p>
        <strong>Billing data.</strong> Processed entirely by our Merchant of Record. We
        only receive subscription status updates (plan, period, next renewal,
        cancellation). We do not see or store your card or postal address.
      </p>

      <h2>2. Purposes and legal bases</h2>
      <table>
        <thead>
          <tr><th>Purpose</th><th>Legal basis</th></tr>
        </thead>
        <tbody>
          <tr><td>Create and manage your account, authenticate you</td><td>Performance of contract (Art. 6(1)(b) GDPR)</td></tr>
          <tr><td>Store and sync your saves</td><td>Performance of contract</td></tr>
          <tr><td>Process your Pro subscription</td><td>Performance of contract</td></tr>
          <tr><td>Diagnostics, abuse prevention, security</td><td>Legitimate interest (Art. 6(1)(f))</td></tr>
          <tr><td>Comply with legal obligations (tax, court orders)</td><td>Legal obligation (Art. 6(1)(c))</td></tr>
          <tr><td>Service communications (critical notices)</td><td>Performance of contract</td></tr>
        </tbody>
      </table>

      <h2>3. Sub-processors</h2>
      <p>
        To operate the service we rely on the following providers. All are bound by a
        data processing agreement under Art. 28 GDPR and process data exclusively within
        the European Union.
      </p>
      <table>
        <thead>
          <tr><th>Provider</th><th>Function</th><th>Location</th></tr>
        </thead>
        <tbody>
          <tr><td>Supabase (Singapore Pte Ltd, infrastructure on AWS Frankfurt)</td><td>Authentication and metadata database</td><td>Frankfurt, Germany</td></tr>
          <tr><td>Cloudflare R2 (Cloudflare Inc., EU jurisdiction)</td><td>Snapshot storage, encrypted at rest</td><td>European Union</td></tr>
          <tr><td>Fly.io (Fly.io Inc.)</td><td>API hosting</td><td>Paris, France (CDG)</td></tr>
          <tr><td>GitHub Pages (Microsoft Corp.)</td><td>Public site hosting</td><td>CDN network, EU-first for EU traffic</td></tr>
          <tr><td>Merchant of Record (third-party payment provider)</td><td>Billing, VAT, invoicing</td><td>EU / EEA</td></tr>
          <tr><td>Resend (once enabled)</td><td>Transactional email delivery</td><td>European Union</td></tr>
        </tbody>
      </table>
      <p>
        Some providers (Cloudflare, GitHub, Fly.io) have parent companies in the United
        States. Their operations for European customers are run under the EU-U.S. Data
        Privacy Framework and Standard Contractual Clauses approved by the European
        Commission. Relevant personal data is not replicated outside the EU.
      </p>

      <h2>4. Location and transfers</h2>
      <p>
        Your account, metadata and snapshots are stored in the European Union (Frankfurt
        for the database, Cloudflare R2 EU-jurisdiction infrastructure for files). We do
        not replicate data outside the EU.
      </p>

      <h2>5. Retention</h2>
      <p>
        We retain your data for as long as your account is active. If you delete your
        account, all snapshots and associated metadata are irreversibly removed within 30
        days. Operational backups are overwritten on a 30-day cycle. Technical logs are
        retained for up to 30 days and app diagnostic logs for 14 days. Billing records are
        kept for the periods required by
        Spanish tax law (generally 6 years).
      </p>

      <h2>6. Your rights</h2>
      <p>
        As a data subject you have the right to: access your data; rectify it; erase it
        ("right to be forgotten"); restrict processing; object to processing based on
        legitimate interest; receive your data in a portable format; withdraw consent
        where that is the legal basis. You can exercise any of these rights by writing to
        <a href="mailto:support@hoard.services">support@hoard.services</a>.
      </p>
      <p>
        Many of these rights are automated in the app: you can export all your data in
        original format from your account, and delete the entire account with one click.
      </p>
      <p>
        If you believe we have mishandled your data, you can lodge a complaint with the
        Spanish Data Protection Agency (AEPD) at
        <a href="https://www.aepd.es" target="_blank" rel="noopener">aepd.es</a> or with
        the supervisory authority in your country of residence.
      </p>

      <h2>7. Cookies and similar</h2>
      <p>
        The public site (hoard.services) uses no tracking cookies and no third-party
        analytics. We use only browser localStorage to remember your Supabase session
        after login. We do not use advertising cookies and we do not share data with
        social networks.
      </p>

      <h2>8. Children</h2>
      <p>
        The Service is not directed at children under 16 (or the applicable digital
        consent age). If you become aware that a child under that age holds an account,
        contact us and we will delete it.
      </p>

      <h2>9. Security</h2>
      <p>
        Snapshots are encrypted at rest in storage. Communication between the app and
        server uses TLS. Passwords are not stored in plain text (we delegate
        authentication to Supabase, which applies strong hashing). API access is audited
        via structured logs. No security measure is perfect; we recommend not using the
        Service for legally sensitive data.
      </p>

      <h2>10. Changes to this Policy</h2>
      <p>
        We may update this Policy. Material changes will be notified by email at least 30
        days in advance. The "last updated" date at the top reflects the version in
        force.
      </p>

      <h2>11. Contact</h2>
      <p>
        For any privacy questions or to exercise your rights:
        <a href="mailto:support@hoard.services">support@hoard.services</a>.
      </p>
    {/if}
  </article>
</section>
