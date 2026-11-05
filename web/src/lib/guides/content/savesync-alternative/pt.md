---
title: "Alternativa ao SaveSync: sincronização de saves gratuita e open source"
description: "Procuras uma alternativa gratuita ao SaveSync? O Hoard faz backup e sincroniza saves de todos os jogos, não só cooperativos, em PC, Linux, macOS e Deck."
order: 29
updated: 2026-11-05
---

O SaveSync é uma aplicação paga no Steam, em acesso antecipado, pensada para o cooperativo: mantém um mundo partilhado disponível para o grupo através de entradas privadas do Steam Workshop, mesmo quando o anfitrião está offline. Se o que queres é sincronizar de graça os teus próprios saves, de todos os jogos que jogas e em todas as máquinas que tens, o [Hoard](/download) faz isso, e é open source.

## O que o SaveSync faz bem

- **Mundos cooperativos.** Passar um mundo de Valheim ou Factorio a quem vai alojar esta noite é precisamente o trabalho para que foi feito.
- **O Steam como transporte.** Os saves viajam como entradas privadas, não listadas e cifradas do Workshop, por isso não há outra conta para criar.
- **Dá-se bem com o Steam Cloud.** Funciona ao lado da sincronização do Steam em vez de lutar com ela.

## Onde o Hoard é diferente

| | SaveSync | Hoard |
|---|---|---|
| Preço | Pago, no Steam | Gratuito e open source (AGPL-3.0) |
| Jogos | Um conjunto de jogos cooperativos suportados | Todos os jogos da base de dados comunitária, 19 emuladores e qualquer pasta que adiciones |
| Armazenamento | 500 MB incluídos, mais à venda dentro da aplicação | Hoard One gratuito com 2 GB e três dispositivos, ou o teu próprio servidor sem quota nossa |
| Histórico | Conforme a aplicação | Todas as versões, desduplicadas |
| Plataformas | Windows | Windows, Linux, macOS e SteamOS |
| Partilhar um mundo com amigos | Sim | Não |

A última linha é a honesta. Se o objetivo é passar um mundo entre amigos, o SaveSync foi feito para isso e o Hoard não. Para tudo o resto, os teus próprios saves no fixo, no portátil e na Steam Deck, o Hoard cobre mais jogos e não custa nada.

## Como começar

1. Instala o Hoard e inicia sessão no Hoard One, recomendado para todos, ou aponta-o para o [teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca**. Os teus jogos são detetados a partir dos teus launchers e da base de dados comunitária de saves.
3. Joga. Cada gravação junta uma versão, e com o Hoard nas tuas outras máquinas com a mesma conta, a mais recente espera-te lá. Vê [como sincronizar saves entre a Steam Deck e o PC](/guides/sync-saves-steam-deck-pc).

## Hoard One, ou o teu próprio servidor

O Hoard é gratuito das duas maneiras, e a aplicação é a mesma em ambas:

- **Hoard One**, recomendado para todos. Nada para configurar nem para manter a correr: um servidor gerido na UE, gratuito com 2 GB e três dispositivos, sem cartão. O [Pro](/pricing) só acrescenta espaço, 100 GB e dispositivos ilimitados, para quando os teus saves já não couberem.
- **O teu próprio servidor.** Corres o `hoard-server` num PC, num NAS ou num Raspberry Pi e todas as tuas máquinas sincronizam através dele. Sem conta connosco, sem telemetria para nós e sem quota nossa: nada passa pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo programa, a mesma deteção, o mesmo histórico. O Hoard One guarda todas as versões para sempre; no teu próprio servidor decides tu quanto histórico guardar.

<!-- faq -->

## Perguntas frequentes

### Há uma alternativa gratuita ao SaveSync?

O Hoard. A aplicação, a linha de comandos e o servidor são gratuitos e open source, e o Hoard One, o servidor gerido, é gratuito com 2 GB e três dispositivos. Não partilha mundos entre amigos, mas sincroniza os teus próprios saves de todos os jogos.

### Posso mandar um save a um amigo com o Hoard?

Não como funcionalidade: o Hoard mantém os teus saves sincronizados entre as tuas próprias máquinas. Podes exportar os teus saves tal como o jogo os escreveu e mandar tu os ficheiros.

### O Hoard é gratuito?

Sim. A aplicação, a linha de comandos e o servidor são gratuitos e de código aberto (AGPL-3.0), e correr o teu próprio servidor não custa mais do que a máquina. O Hoard One, o servidor gerido que recomendamos a todos, também é gratuito com 2 GB e três dispositivos; o Pro acrescenta espaço por 1,99 € por mês.

### O self-hosting envia alguma coisa ao Hoard?

Não. No teu próprio servidor não há conta connosco nem telemetria para nós: os teus saves, os teus utilizadores e os teus logs ficam na tua máquina e nunca tocam nos nossos.
