---
title: "Sincronizar saves entre Windows e Linux num PC com dual boot"
description: "Um PC, dois sistemas, duas pastas de saves. Sincroniza os saves entre Windows e Linux automaticamente, e porque é que partilhar uma pasta NTFS falha."
order: 18
updated: 2026-10-09
---

Num PC com dual boot, o mesmo jogo tem dois saves separados: um na tua pasta de utilizador do Windows e outro dentro de um prefixo do Proton no Linux. O Steam Cloud liga os dois nos jogos que o suportam; tudo o resto separa-se da primeira vez que mudas de sistema. O Hoard mantém-nos sincronizados automaticamente. Instala-o nos dois sistemas com a mesma conta, e o save de cada jogo segue-te seja qual for o sistema que arrancas.

## Porque é que um jogo tem dois saves

O disco é o mesmo, mas as pastas de saves não:

- **No Windows**, um jogo escreve em `Documents`, `Saved Games` ou numa das pastas `AppData` em `C:\Users\<tu>`.
- **No Linux**, o mesmo jogo de Windows corre através do Proton e escreve dentro do seu próprio prefixo: `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, seguido do mesmo caminho do Windows.

Duas cópias de um save, em dois sistemas que nunca estão ligados ao mesmo tempo. Joga onde jogares por último, o outro sistema não sabe. O Hoard procura nos dois sítios e associa-os por jogo, de modo que o save do Windows e o do Linux passam a ser duas versões de um mesmo histórico.

## O que o Steam Cloud já cobre

Nos jogos do Steam com Steam Cloud, o Steam sincroniza sozinho o save entre a tua instalação do Windows e a do Proton. Aí, o que o Hoard acrescenta é o histórico: o Steam guarda só o save atual, por isso um estragado substitui o bom nos dois sistemas. Nos jogos sem Steam Cloud, e em tudo o que está fora do Steam, o Hoard também trata da sincronização.

## Porque não partilhar simplesmente uma pasta no disco do Windows?

É a primeira ideia de quase toda a gente: apontar o Linux para os saves da partição do Windows e pronto. Costuma falhar de três maneiras:

- **Arranque rápido e hibernação.** Quando o Windows se desliga com o arranque rápido ativo, deixa a partição meio hibernada, e o Linux monta-a só de leitura ou recusa-se. O teu jogo não consegue escrever o save.
- **NTFS com o Proton.** Usar prefixos do Proton ou bibliotecas do Steam a partir de um disco NTFS é uma fonte conhecida de problemas de permissões e de nomes de ficheiros. Os jogos no Linux dão-se melhor num sistema de ficheiros do Linux.
- **As ligações são substituídas.** Ligar a pasta de saves de um sistema dentro do outro funciona até um jogo, uma atualização ou uma reinstalação trocar a ligação por uma pasta a sério, sem avisar.

Deixar cada sistema guardar os saves onde o jogo os espera, e sincronizá-los entre eles, evita os três.

## Configurar

1. **No Windows**, instala o Hoard e inicia sessão.
2. **No Linux**, instala o Hoard a partir da [página de transferência](/download) e inicia sessão com a mesma conta.
3. **Abre uma vez cada jogo do Proton no Linux**, para que o prefixo exista. Antes disso não há pasta onde pôr o save.
4. Vê a **Biblioteca** nos dois sistemas: devem aparecer os mesmos jogos em cada um, e o Hoard associa-os por jogo.

## A armadilha que só o dual boot tem

Com dois PCs separados, o save espera no servidor até a outra máquina o pedir. Num PC com dual boot, a "outra máquina" é o mesmo computador depois de reiniciar, e isso muda um hábito.

O Hoard envia um save quando o jogo fecha e a pasta fica parada. **Se sais do jogo e reinicias logo a seguir, o envio pode ainda não ter acontecido**, e o outro sistema arranca sem o teu último progresso. Vai pôr-se em dia da próxima vez que voltares a arrancar o primeiro, mas até lá podes ter jogado sobre o save antigo.

Portanto: sai do jogo, dá um momento ao Hoard, confirma na app que o save está atualizado, e só depois reinicia.

## Versões nativas de Linux

Alguns jogos têm versão nativa de Linux além da de Windows. As duas nem sempre usam o mesmo formato de save, e algumas guardam-nos em sítios completamente diferentes. Se queres o mesmo save nos dois sistemas, o mais seguro é correr também no Linux a versão de Windows com o Proton: no Steam, **Propriedades → Compatibilidade** e força uma versão do Proton. Assim os dois sistemas correm o mesmo jogo e escrevem os mesmos ficheiros.

## Definições e dispositivos

As definições gráficas podem ser diferentes entre os dois sistemas, por isso o Hoard faz backup dos ficheiros de definições como `graphics.ini` mas não os escreve por cima dos do outro sistema. Se mesmo assim os quiseres copiar, há uma opção para isso ao restaurar.

Cada sistema operativo conta como um dispositivo próprio, por isso um PC com dual boot usa dois dos três do plano gratuito. O Pro e os servidores alojados por ti não têm limite de dispositivos.

Preferes que os saves fiquem em casa? Corre o `hoard-server` num NAS ou noutra máquina e aponta os dois sistemas para ele. Sem conta connosco, sem telemetria para nós, nada a passar pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

<!-- faq -->

## Perguntas frequentes

### O Steam Cloud sincroniza entre Windows e Linux?

Sim, nos jogos que o suportam: o Steam guarda uma cópia na nuvem por conta, seja qual for o sistema em que jogas. Não guarda histórico, e não cobre jogos sem Steam Cloud nem nada fora do Steam.

### Posso deixar os meus saves na partição NTFS partilhada?

Não é recomendável. O arranque rápido pode deixar a partição só de leitura no Linux, e o Proton tem problemas conhecidos com NTFS. É mais fiável que cada sistema guarde os seus saves no seu sítio e sincronizá-los.

### Porque é que o meu save não apareceu depois de reiniciar?

O mais provável é que o envio não tivesse terminado quando reiniciaste. Volta a arrancar o primeiro sistema, deixa o Hoard enviar e confirma na app antes de mudares outra vez.

### Um PC com dual boot conta como um só dispositivo?

Não, como dois: cada sistema operativo regista-se como um dispositivo próprio. No plano gratuito são dois de três; o Pro e os servidores alojados por ti não têm limite.

### E se um jogo tiver versão nativa de Linux?

Os saves dela podem não coincidir com os da versão de Windows. Para partilhar um save, corre também no Linux a versão de Windows com o Proton.
