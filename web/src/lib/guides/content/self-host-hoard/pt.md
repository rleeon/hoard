---
title: "Como auto-hospedar o Hoard com Docker (self-hosted)"
description: "Corre o teu servidor Hoard com Docker Compose e sincroniza os saves entre todos os teus dispositivos: grátis, código aberto, sem conta connosco e sem quota."
order: 0
featured: true
updated: 2026-10-09
---

O Hoard é de código aberto e pode ser auto-hospedado. Em vez de usar o Hoard Cloud, você pode rodar o mesmo `hoard-server` na sua própria máquina e apontar todos os dispositivos para ele — sem conta e sem limite de espaço além do disco que você der a ele. Este guia coloca um servidor no ar com Docker em poucos minutos.

## Por que auto-hospedar o Hoard

- **Controle total.** Seus saves ficam em hardware que você controla, não na nuvem de outra pessoa.
- **Sem cota.** O espaço é limitado apenas pelo seu próprio disco.
- **Mesmo app, mesmos recursos.** Histórico versionado e sincronização em segundo plano funcionam igual ao Hoard Cloud — só muda o backend.
- **Código aberto.** Você pode ler, auditar e modificar o servidor.

Essa é a diferença principal em relação a ferramentas como o [Ludusavi](/guides/ludusavi-alternative): o Ludusavi é ótimo para backups locais e para usar sua própria nuvem via Rclone, mas a sincronização você mesmo monta. O Hoard oferece um servidor de sincronização gerenciado que você sobe uma vez e ao qual cada dispositivo se conecta.

## O que o self-hosting significa para os teus dados

Vale a pena dizê-lo sem rodeios, porque é o ponto em que quase todas as comparações se enganam sobre o Hoard.

**O Hoard Cloud** é a opção gerida: inicias sessão e os teus saves ficam nos nossos servidores, na UE.

**Um Hoard self-hosted é inteiramente teu.** Os teus dispositivos falam com o teu servidor e com mais nada. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Não conseguimos ver um save, o nome de um jogo ou um endereço de email, pela simples razão de que nada disso nos chega. Se o Hoard Cloud fechasse amanhã, a tua instalação continuaria igual.

E, para ser exato numa coisa: o teu servidor tem sim os seus próprios acessos — o utilizador que crias mais abaixo e um token por dispositivo. São teus, na tua máquina, na tua base de dados. O que não existe é uma conta connosco.

## O que você precisa

- Uma máquina que fique ligada (um servidor doméstico, um NAS que rode Docker ou um VPS pequeno).
- Docker e Docker Compose instalados (em um NAS Synology, o pacote Container Manager).
- Opcionalmente um domínio e um proxy reverso para HTTPS (recomendado para qualquer coisa fora da sua rede local).

## Instalação com Docker Compose

Não é preciso clonar o repositório. O servidor é uma imagem pronta (`ghcr.io/rleeon/hoard`, amd64 e arm64), e o único arquivo que você baixa é o `docker-compose.yml` dela:

```sh
mkdir hoard && cd hoard
curl -O https://raw.githubusercontent.com/rleeon/hoard/main/deploy/docker/docker-compose.yml

# Used only on the first start: they create that admin and print a device token in the log, once
HOARD_ADMIN_USERNAME=alice HOARD_ADMIN_PASSWORD='CHANGE_ME' docker compose up -d
docker compose logs -f server                  # wait for "listening", then copy the token
```

Na primeira inicialização, o contêiner grava um `config.toml` funcional em `./config/`, ao lado do arquivo compose; com o `docker compose` puro, não há nada a preparar. Os dados ficam em um volume nomeado do Docker (`hoard-data`) — faça backup como em qualquer outro volume. O contêiner escuta internamente na porta `12421`; use outra porta do host com `HOARD_PORT=9000 docker compose up -d`.

Prefere ler a configuração antes de iniciar qualquer coisa, ou compilar a imagem você mesmo? O [guia de self-hosting do repositório](https://github.com/rleeon/hoard/blob/main/SELF-HOST_GUIDE.md) explica como cloná-lo.

### Em um NAS Synology (Container Manager)

O Container Manager não tem linha de comando para passar essas duas variáveis, e se recusa a iniciar um projeto enquanto faltar uma pasta montada: é o erro `Bind mount failed: '…/config' does not exist`. Quatro passos resolvem as duas coisas:

1. No File Station, crie uma pasta para o Hoard (por exemplo `docker/hoard`) e, dentro dela, uma pasta `config` vazia.
2. No Container Manager, abra **Project** → **Create**, defina essa pasta como caminho, escolha criar um `docker-compose.yml` e cole o conteúdo do arquivo (para obtê-lo, abra no navegador a URL da linha `curl` acima).
3. No arquivo colado, substitua `${HOARD_ADMIN_USERNAME:-}` e `${HOARD_ADMIN_PASSWORD:-}` pelo usuário e pela senha do seu administrador, sem mexer no resto de cada linha, para que fiquem assim:

   ```yaml
   HOARD_ADMIN_USERNAME: alice
   HOARD_ADMIN_PASSWORD: 'CHANGE_ME'
   ```

4. Conclua o assistente para iniciá-lo e abra o log de `hoard-server` em **Container**: o token do dispositivo aparece ali, uma única vez.

Com o arquivo como está, os saves ficam no armazenamento do próprio Docker, fora das suas pastas compartilhadas. Para mantê-los em uma pasta compartilhada da qual você já faz backup, crie também uma pasta `data` ao lado de `config` e troque a linha `hoard-data:/var/lib/hoard` por `./data:/var/lib/hoard`.

## Crie seu usuário e um token de dispositivo

Se você o iniciou com `HOARD_ADMIN_USERNAME` e `HOARD_ADMIN_PASSWORD`, isso já está feito: o usuário existe e o token dele está no log. Caso contrário, crie-os pela linha de comando, já que o servidor não tem tela de cadastro:

```sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'desktop'
```

O token é exibido uma única vez e **não pode ser recuperado depois**, então copie-o agora. Para cada dispositivo que você adicionar depois, não é preciso terminal: abra o endereço do servidor no navegador, entre no painel web com esse usuário e essa senha e use **Utilizadores** → **Novo token**.

## Conecte o app de desktop

Instale o [app de desktop do Hoard](/download) em cada máquina. No fluxo inicial, escolha **Self-Host** e cole a URL do seu servidor e o token recém-criado. A partir daí ele se comporta exatamente como o Hoard Cloud: detecta seus jogos, faz backup dos saves automaticamente e mantém o histórico versionado. Veja [sincronizar saves entre vários PCs](/guides/sync-game-saves-across-pcs) para o uso no dia a dia.

## Mantenha seu servidor atualizado

Como atualizar depende de como você instalou, e errar o comando não dá erro: simplesmente não faz nada. Vale saber qual é o seu caso.

**Docker Compose.** Baixe a imagem nova e recrie o contêiner. As duas metades, nesta ordem:

```sh
docker compose pull
docker compose up -d
```

Se parar na primeira, o contêiner antigo continua rodando intacto: `/v1/health` segue informando a versão antiga e a atualização parece ter falhado em silêncio. `git pull` não atualiza nenhum dos dois — o que roda é a imagem publicada, não o seu clone do repositório. Fixe uma versão (`ghcr.io/rleeon/hoard:1.1`) no lugar de `:latest` se preferir escolher quando uma nova chega.

**Unraid.** Aba *Docker* → Hoard → *Apply update* quando aparecer. Nada para digitar.

**Bare metal (systemd).** `sudo hoard-server upgrade` e depois `sudo systemctl restart hoard-server`. Ele troca o binário de forma atômica e de propósito não reinicia o serviço sozinho, para não cortar uma sincronização em andamento.

`hoard-server upgrade` é só para a instalação bare metal. Dentro de um contêiner ele se recusa de propósito — a troca de binário não sobreviveria ao próximo `docker compose up -d` — e imprime os dois comandos acima; rode `docker compose exec server hoard-server upgrade` se quiser vê-lo dizer isso. As migrações do banco de dados são aplicadas pelo servidor ao iniciar, então nunca há um passo separado para elas.

## Em produção

Para qualquer coisa exposta além da rede local, termine o TLS em um proxy reverso (Caddy, nginx ou Traefik). Prefere bare metal? O repositório também traz um script de instalação `systemd` e um comando `hoard-server upgrade` que troca o binário de forma atômica sem matar uma sincronização em andamento.

## Self-hosted ou Hoard Cloud?

Auto-hospedar é ideal se você já tem um servidor e quer controle total sem cota. Se preferir não manter infraestrutura, o [Hoard Cloud](/pricing) oferece a mesma sincronização gerenciada por nós, com um plano gratuito para começar. De qualquer forma, o app e seus saves continuam portáteis — você pode trocar depois.

<!-- faq -->

## Perguntas frequentes

### Um Hoard self-hosted comunica convosco?

Não. A aplicação de ambiente de trabalho fala com o endereço de servidor que lhe deres. Os teus saves, os teus utilizadores e os teus registos ficam na tua máquina, e nada disso nos chega.

### O servidor self-hosted é o mesmo código do Hoard Cloud?

Sim, o mesmo binário `hoard-server`, sob AGPL-3.0. Não há uma edição comunitária reduzida nem funcionalidades guardadas para a versão alojada.

### Onde ficam realmente guardados os saves?

Por omissão, no volume Docker que deres ao contentor, no teu próprio disco. Se já tens armazenamento de objetos, o servidor também fala S3, por isso MinIO, Garage ou Backblaze B2 servem de repositório. Em qualquer dos casos, os teus dispositivos só falam com o teu servidor.

### Posso pô-lo a correr num NAS?

Sim, em qualquer NAS que corra Docker. No Synology, segue os passos do Container Manager acima; para o Unraid, o repositório inclui um template. Em ambos os casos a imagem desce para os `PUID`/`PGID` que indicares, para que as pastas montadas fiquem do utilizador certo em vez de root.

### Preciso de domínio e HTTPS?

Na tua própria rede local, não. A partir do momento em que o servidor é acessível de fora, põe um proxy inverso à frente e termina aí o TLS: Caddy, nginx ou Traefik servem.

### E se o meu servidor estiver em baixo quando acabo de jogar?

O snapshot é tirado localmente, por isso não se perde nada. Sobe sozinho assim que o servidor voltar a responder.

### Posso começar no Hoard Cloud e mudar mais tarde?

Sim, nos dois sentidos. Podes exportar tudo a partir da página da tua conta, e a aplicação pode ser apontada a outro servidor sem reinstalar nada.
