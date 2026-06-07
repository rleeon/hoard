# Detección de saves

> El problema central: dado un juego, ¿dónde guarda la partida en disco? El
> nombre de la carpeta casi nunca es "Saves" — a veces es un GUID, a veces el
> título del juego en japonés, a veces `remote/` de Steam Cloud. Hoard ataca
> esto por capas.

La arquitectura es la del **ADR 0020**, en fases. Cada fase aporta una señal;
las señales se suman en un *score* `S ∈ [0,1]`.

```
Fase 0  roots.rs        ¿dónde mirar?           (raíces de búsqueda)
Fase 1  scoring.rs      ¿esto parece save?      (señales estáticas)
Fase 3  correlation.rs  ¿se escribió mientras   (la señal fuerte, +0.50)
                         el juego corría?
Fase 4  detection.rs    ¿de qué juego es?       (atribución + rescate)
```

---

## Las dos vías de detección

Hoard tiene **dos caminos** que conviven:

### 1. Catalog-first (ADR 0009 — el camino principal)

Para un juego conocido, Hoard mira el **manifest de Ludusavi** embebido
(`hoard-manifest`). El manifest dice, por título, plantillas de ruta tipo
`<winAppData>/Larian Studios/*/Savegames`. Hoard:

1. Expande los placeholders (`<winAppData>`, `<home>`, prefijos Proton/Wine…)
   con `pathexpand`.
2. Refina root→subcarpeta-save (si la plantilla apunta a una raíz, baja hasta
   la carpeta que realmente tiene saves usando `SAVE_PATTERNS`).
3. Si falla por Steam AppID, cae a buscar por nombre en el catálogo.

Esto resuelve la mayoría de juegos populares sin esfuerzo. Es preciso porque
parte de datos curados.

### 2. Catalog-free (ADR 0020 — lo nuevo, para lo que el catálogo no cubre)

Cuando el catálogo no sabe del juego (indie, japonés, mod, lanzador raro),
entra el **descubrimiento agresivo**: Hoard camina por las raíces conocidas y
**puntúa** cada carpeta candidata. Aquí viven las fases 1, 3 y 4.

---

## Fase 0 — ¿Dónde mirar? (`roots.rs`)

No se escanea todo el disco. Se camina sólo en raíces donde los juegos suelen
guardar:

- `user_save_roots(os)` — `Documents/My Games`, `AppData`, `~/.local/share`,
  `~/.config`, Saved Games, etc. según SO.
- `prefix_user_roots(prefix)` — dentro de cada **prefijo Proton/Wine** el
  `drive_c/users/<user>/...` equivalente.

El walk está acotado: profundidad máxima, timeout por raíz, y un tope de
candidatos (`AGGRESSIVE_WALK_MAX_CANDIDATES`) para no quemar I/O.

---

## Fase 1 — ¿Esto parece un save? (`scoring.rs`)

`score_dir(path, name)` combina señales **estáticas** (sin saber nada del juego
ejecutándose). Devuelve un `ScoreBreakdown { score, reasons }` — el `reasons`
es la lista de por-qués, que se reenvía al panel de diagnóstico.

**Señal de nombre** (`name_signal`):

| Coincidencia | Aporte |
|---|---|
| Nombre exacto en vocabulario (`save`, `partida`, `セーブ`, `存档`…) | +0.35 |
| Contiene un token save (substring) | +0.20 |
| Patrón slot/profile/user | +0.15 |

El vocabulario `SAVE_NAME_VOCAB` es multilingüe (en/es/de/fr/it/ru/ja/zh) para
no perder saves con nombre no-inglés.

**Señal de contenido** (`scan_content`, no recursivo):

| Contenido | Aporte |
|---|---|
| Extensión fuerte (`.sav .save .sl2 .ess .dsav`) | +0.30 |
| Extensión débil (`.dat .bin .profile`) + otra señal | +0.08 |
| Extensión ruidosa (`.json .xml .ini .cfg`) y nada más | +0.02 |

**Señal de recencia**: si hay un fichero save-like modificado en los últimos
180 días → +0.10.

**Señales negativas** (restan):

- Nombre delator (`config`, `cache`, `logs`, `shadercache`, `temp`,
  `screenshots`…) → −0.45.
- Carpeta **sólo imágenes** → −0.40 (son screenshots).
- Carpeta **sólo ruido** (config/log, sin nada save-like) → −0.35.

**Regla dura**: una carpeta sólo-imágenes o sólo-ruido **nunca** se
auto-confirma aunque el nombre matchee — se capa por debajo de `SCORE_POSSIBLE`.

### Cortes

```
S ≥ 0.60   → save confirmado            (SCORE_CONFIRMED)
0.35–0.60  → "posible": corroborar      (SCORE_POSSIBLE)
S < 0.35   → descartado
```

---

## Fase 3 — La joya: correlación proceso↔escritura (`correlation.rs`)

Esta es la señal que rompe el techo del nombre. Idea: **si una carpeta se
reescribe justo mientras un proceso-juego está vivo, es un save** — da igual
cómo se llame.

- `sample_game_processes()` muestrea procesos que parecen juego (`is_game_like`
  filtra navegadores, el propio Hoard, etc.).
- Cuando se observa escritura en una carpeta con un juego corriendo, se
  registra en el `CorrelationStore` (persistente).
- `signal_for(path)` camina ancestros: si esta carpeta o un padre quedó
  correlacionada, devuelve la señal (incluye el nombre del proceso atribuido,
  `.exe` recortado).
- `score_with_correlation` **suma `CORRELATION_BONUS = 0.50`** sobre el score
  estático.

Ese +0.50 es enorme: una carpeta GUID con score estático ~0.15 (nombre nulo,
algún `.dat`) cruza el 0.60 y se confirma.

---

## Fase 4 — ¿De qué juego es? Atribución y rescate (`detection.rs`)

`discover_unattributed(os, store, known_paths)` corre **una sola vez** (no por
juego) al final de `detect_all`. Camina `user_save_roots` + prefijos Wine
buscando saves que el catálogo **no** atribuyó a nadie.

**Puerta de precisión**: en raíces tan amplias, un acierto sólo-por-nombre (Low)
inventaría juegos fantasma. Por eso sólo emergen candidatos que estén
**corroborados por correlación** o con confianza ≥ Medium. Lo Low se descarta.

**Atribución del nombre** (`attribute_game_name`):

1. Si hay correlación → usa el **nombre del proceso** que escribió (lo más
   fiable: el binario que tocó el save).
2. Si no → el **ancestro no-genérico más cercano**. Salta segmentos genéricos
   (`Saves`, `My Games`, `AppData`…) hasta dar con algo que parezca título:
   `…/My Games/Skyrim/Saves` → **"Skyrim"**.

Devuelve `AttributedSave { slug, display_name, path, confidence, reason }`. El
`reason` explica al usuario por qué apareció.

`path_already_known` evita duplicar lo que el catálogo ya encontró (compara
prefijos en ambos sentidos: ni subcarpetas ni padres de rutas conocidas).

---

## Niveles de confianza (lo que ve el usuario)

`classify_dir_as_save_like` traduce score + correlación a un nivel:

```
score ≥ 0.60  y corroborado por correlación   → High
score ≥ 0.60  sin correlación                 → Medium
score < 0.60                                  → Low
```

High sólo se desbloquea cuando coinciden **score fuerte Y correlación** — es la
combinación que casi nunca da falso positivo.

---

## Overrides manuales

Al final de `detect_all`, `apply_manual_overrides` aplica los `manual_paths`
que el usuario fijó a mano. Tienen prioridad absoluta: la detección automática
nunca pisa una ruta puesta por el usuario.

---

## Resumen mental

> Catálogo para lo conocido. Para lo desconocido: puntúa por nombre + contenido
> + recencia, y si la carpeta se movió mientras el juego corría, eso pesa más
> que todo lo demás. Atribuye por el proceso que escribió, o por la carpeta
> padre con nombre de juego.
