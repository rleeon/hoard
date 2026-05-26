<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import { ArrowLeft } from 'lucide-svelte';

  const isEs = $derived(String($locale ?? 'en').toLowerCase().startsWith('es'));
</script>

<svelte:head>
  <title>{$_('legal.terms_title')} — Hoard</title>
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
    {$_('legal.terms_title')}
  </h1>
  <p class="mt-3 text-sm text-zinc-500">{$_('legal.last_updated')}</p>

  <article
    class="prose-legal mt-10 space-y-6 text-[15px] leading-relaxed text-zinc-300 [&_h2]:mt-10 [&_h2]:text-xl [&_h2]:font-semibold [&_h2]:text-white [&_h3]:mt-6 [&_h3]:text-base [&_h3]:font-semibold [&_h3]:text-zinc-100 [&_a]:text-emerald-400 [&_a]:underline [&_a:hover]:text-emerald-300 [&_strong]:text-white [&_ul]:list-disc [&_ul]:pl-6 [&_ul]:space-y-1.5 [&_li]:marker:text-zinc-600"
  >
    {#if isEs}
      <p>
        Estos Términos rigen el uso del servicio Hoard ("Hoard", "el Servicio", "nosotros"),
        operado por Rai León Oliva, persona física residente en España, contactable en
        <a href="mailto:support@hoard.services">support@hoard.services</a>. Al crear una cuenta
        o usar el Servicio aceptas estos Términos. Si no estás de acuerdo, no uses el Servicio.
      </p>

      <h2>1. Qué es Hoard</h2>
      <p>
        Hoard es un servicio de sincronización en la nube de partidas guardadas de videojuegos.
        Una aplicación de escritorio detecta archivos de save en tu equipo, genera snapshots
        firmados por contenido (SHA-256), los sube a almacenamiento en la UE y te permite
        restaurar versiones anteriores. El código del cliente y del servidor es open source
        bajo licencia AGPL-3.0 y puede ser auto-alojado por cualquiera.
      </p>

      <h2>2. Cuenta y elegibilidad</h2>
      <p>
        Para usar el Servicio debes tener al menos 16 años (o la edad mínima de consentimiento
        digital en tu jurisdicción si es superior). Eres responsable de mantener seguras tus
        credenciales y de toda actividad bajo tu cuenta. Una sola persona puede tener una sola
        cuenta gratuita.
      </p>

      <h2>3. Planes y suscripciones</h2>
      <p>
        El plan Free es gratuito y se ofrece tal cual, con los límites publicados en
        <a href="/pricing">la página de precios</a>. El plan Pro es de pago, con facturación
        recurrente mensual o anual. Los precios mostrados incluyen los impuestos aplicables a
        consumidores en la Unión Europea.
      </p>
      <p>
        Los pagos los procesa un proveedor externo que actúa como Comerciante Registrado
        (Merchant of Record), responsable del cobro, IVA, facturación y reembolsos. No
        almacenamos datos de tu tarjeta. La política completa de cobro la aplica el
        Merchant of Record en el momento de la compra.
      </p>

      <h3>3.1. Cancelación y reembolsos</h3>
      <p>
        Puedes cancelar tu suscripción en cualquier momento desde la sección Cuenta. La
        cancelación es efectiva al final del periodo facturado en curso; mantienes acceso a
        Pro hasta esa fecha y luego pasas a Free automáticamente.
      </p>
      <p>
        Como consumidor en la UE tienes 14 días de derecho de desistimiento desde la
        contratación. Al iniciar el Servicio expresamente solicitas su ejecución inmediata,
        y reconoces que pierdes el derecho de desistimiento una vez el Servicio ha sido
        prestado en su totalidad durante el periodo. Para periodos parciales, el reembolso
        se valora caso a caso por el Merchant of Record.
      </p>

      <h2>4. Uso aceptable</h2>
      <p>
        Aceptas no usar el Servicio para: (a) subir contenido ilegal, infractor de derechos
        de terceros o que no sean datos de save de videojuegos legítimos; (b) intentar acceder
        a cuentas o datos que no son tuyos; (c) sobrecargar la infraestructura mediante
        scripts automatizados que excedan los límites de cuota publicados; (d) revender o
        rehospedar el Servicio sin autorización escrita. Nos reservamos el derecho de
        suspender cuentas que violen estas reglas, con notificación previa cuando sea
        razonable.
      </p>

      <h2>5. Tu contenido</h2>
      <p>
        Los datos que subes (snapshots de saves, nombres de juegos, metadatos) son y siguen
        siendo tuyos. Nos concedes una licencia limitada, no exclusiva y revocable para
        almacenarlos, transmitirlos y procesarlos exclusivamente con el fin de operar el
        Servicio. No los compartimos, no los analizamos para entrenar modelos, no los
        vendemos. Puedes exportarlos o eliminarlos en cualquier momento desde la cuenta.
      </p>

      <h2>6. Propiedad intelectual</h2>
      <p>
        La marca Hoard, el logotipo, el diseño del sitio y la documentación son propiedad
        del operador. El código fuente del cliente y del servidor está disponible bajo
        AGPL-3.0 en
        <a href="https://github.com/rleeon/hoard" target="_blank" rel="noopener">GitHub</a>.
      </p>

      <h2>7. Software open source y auto-alojado</h2>
      <p>
        Puedes auto-alojar el servidor de Hoard usando el mismo binario que opera nuestra
        nube. Estos Términos sólo aplican al servicio gestionado en api.hoard.services. El
        uso del software auto-alojado se rige exclusivamente por AGPL-3.0.
      </p>

      <h2>8. Disponibilidad del servicio</h2>
      <p>
        Hacemos esfuerzos razonables para mantener el Servicio disponible, pero no
        garantizamos disponibilidad ininterrumpida. Podemos realizar mantenimiento
        programado o no programado. El plan Free se ofrece sin ningún compromiso de
        nivel de servicio. El plan Pro tiene un objetivo de 99.5 % de disponibilidad
        mensual; cualquier crédito por incumplimiento se decide caso a caso.
      </p>

      <h2>9. Suspensión y terminación</h2>
      <p>
        Podemos suspender o terminar tu acceso al Servicio si: (a) violas estos Términos;
        (b) tu uso causa daño a la infraestructura o a otros usuarios; (c) por requerimiento
        legal. En caso de terminación por nuestra parte sin causa atribuible a ti, te
        notificaremos con antelación razonable y permitiremos exportar tus datos.
      </p>
      <p>
        Puedes terminar tu cuenta en cualquier momento desde la sección Cuenta. La
        eliminación es permanente e irreversible.
      </p>

      <h2>10. Exenciones y limitación de responsabilidad</h2>
      <p>
        El Servicio se ofrece "tal cual". No garantizamos que los datos restaurados sean
        idénticos en todos los aspectos a los originales, especialmente en juegos con saves
        cifrados por hardware o ligados a una cuenta de plataforma. Recomendamos verificar
        los archivos críticos antes de continuar jugando.
      </p>
      <p>
        Salvo en los casos previstos por la legislación de consumidores aplicable, nuestra
        responsabilidad total agregada por cualquier reclamación relacionada con el
        Servicio se limita al importe que hayas pagado a Hoard en los doce meses
        anteriores al hecho que dio origen a la reclamación. Esta limitación no se aplica
        a daños causados por dolo o negligencia grave, ni a derechos que la ley no permita
        limitar.
      </p>

      <h2>11. Cambios en estos Términos</h2>
      <p>
        Podemos actualizar estos Términos. Cambios sustanciales se notificarán por correo
        con al menos 30 días de antelación. El uso continuado del Servicio tras la fecha
        efectiva del cambio supone aceptación.
      </p>

      <h2>12. Ley aplicable y resolución de disputas</h2>
      <p>
        Estos Términos se rigen por la ley española, sin perjuicio de la protección
        obligatoria de consumidores en tu lugar de residencia habitual dentro de la UE.
        Para disputas online, la Comisión Europea facilita una plataforma de resolución
        en
        <a href="https://ec.europa.eu/consumers/odr" target="_blank" rel="noopener"
          >ec.europa.eu/consumers/odr</a
        >.
      </p>

      <h2>13. Contacto</h2>
      <p>
        Cualquier consulta legal o sobre estos Términos:
        <a href="mailto:support@hoard.services">support@hoard.services</a>.
      </p>
    {:else}
      <p>
        These Terms govern use of the Hoard service ("Hoard", "the Service", "we"), operated
        by Rai León Oliva, a natural person resident in Spain, reachable at
        <a href="mailto:support@hoard.services">support@hoard.services</a>. By creating an
        account or using the Service you accept these Terms. If you do not agree, do not use
        the Service.
      </p>

      <h2>1. What Hoard is</h2>
      <p>
        Hoard is a cloud sync service for video game save files. A desktop application
        detects save files on your machine, produces content-addressed snapshots (SHA-256),
        uploads them to EU-based storage, and lets you restore previous versions. Both the
        client and server are open source under AGPL-3.0 and can be self-hosted by anyone.
      </p>

      <h2>2. Account and eligibility</h2>
      <p>
        To use the Service you must be at least 16 years old (or the minimum digital-consent
        age in your jurisdiction if higher). You are responsible for keeping your credentials
        secure and for all activity under your account. A single person may hold only one
        free account.
      </p>

      <h2>3. Plans and subscriptions</h2>
      <p>
        The Free plan is offered at no cost with the limits published on the
        <a href="/pricing">pricing page</a>. The Pro plan is paid with recurring monthly or
        annual billing. Prices displayed include applicable taxes for consumers in the
        European Union.
      </p>
      <p>
        Payments are processed by a third party acting as Merchant of Record, who is
        responsible for collection, VAT, invoicing and refunds. We do not store card data.
        The full payment policy is presented by the Merchant of Record at checkout.
      </p>

      <h3>3.1. Cancellation and refunds</h3>
      <p>
        You can cancel your subscription at any time from the Account section. Cancellation
        is effective at the end of the current billing period; you keep Pro access until
        that date and then drop to Free automatically.
      </p>
      <p>
        As an EU consumer you have a 14-day right of withdrawal from purchase. By starting
        the Service you expressly request its immediate performance and acknowledge that you
        lose the right of withdrawal once the Service has been fully supplied for the
        period. Partial periods are handled case by case by the Merchant of Record.
      </p>

      <h2>4. Acceptable use</h2>
      <p>
        You agree not to use the Service to: (a) upload illegal content, content that
        infringes third-party rights, or anything other than legitimate video game save
        data; (b) attempt to access accounts or data that are not yours; (c) overload the
        infrastructure with automated scripts that exceed published quota limits; (d) resell
        or rehost the Service without prior written authorisation. We may suspend accounts
        that violate these rules, with prior notice when reasonable.
      </p>

      <h2>5. Your content</h2>
      <p>
        The data you upload (save snapshots, game names, metadata) is and remains yours.
        You grant us a limited, non-exclusive, revocable licence to store, transmit and
        process it solely for the purpose of operating the Service. We do not share it, we
        do not analyse it to train models, we do not sell it. You can export or delete it
        at any time from your account.
      </p>

      <h2>6. Intellectual property</h2>
      <p>
        The Hoard mark, logo, site design and documentation are property of the operator.
        Client and server source code is available under AGPL-3.0 on
        <a href="https://github.com/rleeon/hoard" target="_blank" rel="noopener">GitHub</a>.
      </p>

      <h2>7. Open source and self-hosted use</h2>
      <p>
        You may self-host the Hoard server using the same binary that powers our cloud.
        These Terms apply only to the managed service at api.hoard.services. Use of the
        self-hosted software is governed solely by AGPL-3.0.
      </p>

      <h2>8. Service availability</h2>
      <p>
        We make reasonable efforts to keep the Service available, but we do not guarantee
        uninterrupted availability. Scheduled or unscheduled maintenance may occur. The
        Free plan is offered with no service-level commitment. The Pro plan targets 99.5%
        monthly uptime; any credit for missed targets is decided case by case.
      </p>

      <h2>9. Suspension and termination</h2>
      <p>
        We may suspend or terminate your access if: (a) you breach these Terms; (b) your
        use damages the infrastructure or other users; (c) we are legally required to. For
        terminations by us without fault on your side, we will give reasonable advance
        notice and let you export your data.
      </p>
      <p>
        You can terminate your account at any time from the Account section. Deletion is
        permanent and irreversible.
      </p>

      <h2>10. Disclaimers and limitation of liability</h2>
      <p>
        The Service is provided "as is". We do not warrant that restored data will be
        identical in every respect to the original, especially for games with
        hardware-encrypted saves or saves tied to a platform account. We recommend
        verifying critical files before resuming play.
      </p>
      <p>
        Except as required by applicable consumer protection law, our aggregate liability
        for any claim related to the Service is limited to the amount you have paid Hoard
        in the twelve months preceding the event giving rise to the claim. This limit does
        not apply to damages caused by wilful misconduct or gross negligence, nor to
        rights the law does not permit to limit.
      </p>

      <h2>11. Changes to these Terms</h2>
      <p>
        We may update these Terms. Material changes will be notified by email at least 30
        days before the effective date. Continued use of the Service after the effective
        date constitutes acceptance.
      </p>

      <h2>12. Governing law and disputes</h2>
      <p>
        These Terms are governed by the laws of Spain, without prejudice to mandatory
        consumer protection in your habitual residence within the EU. For online disputes,
        the European Commission provides a resolution platform at
        <a href="https://ec.europa.eu/consumers/odr" target="_blank" rel="noopener"
          >ec.europa.eu/consumers/odr</a
        >.
      </p>

      <h2>13. Contact</h2>
      <p>
        For any legal queries or questions about these Terms:
        <a href="mailto:support@hoard.services">support@hoard.services</a>.
      </p>
    {/if}
  </article>
</section>
