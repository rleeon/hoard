---
title: "Como auto-hospedar o Hoard com Docker (self-hosted)"
description: "Coloque seu próprio servidor Hoard no ar em minutos com o Docker Compose. Código aberto, gratuito e no seu hardware: uma nuvem totalmente self-hosted para seus saves de jogos, sem conta nem limite de espaço."
order: 0
featured: true
updated: 2026-09-01
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
- Docker e Docker Compose instalados.
- Opcionalmente um domínio e um proxy reverso para HTTPS (recomendado para qualquer coisa fora da sua rede local).

## Instalação com Docker Compose

Clone o repositório, crie uma configuração a partir do exemplo e suba o stack:

```sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
```

Aguarde até os logs mostrarem que o servidor está escutando. Os dados ficam em um volume nomeado do Docker (`hoard-data`) — faça backup como em qualquer outro volume. O contêiner escuta internamente na porta `12421`; use outra porta do host com `HOARD_PORT=9000 docker compose up -d`.

## Crie seu usuário e um token de dispositivo

O servidor não tem tela de cadastro — os usuários são criados pela linha de comando:

```sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'desktop'
```

O token é exibido uma única vez e **não pode ser recuperado depois**, então copie-o agora.

## Conecte o app de desktop

Instale o [app de desktop do Hoard](/download) em cada máquina. No fluxo inicial, escolha **Self-Host** e cole a URL do seu servidor e o token recém-criado. A partir daí ele se comporta exatamente como o Hoard Cloud: detecta seus jogos, faz backup dos saves automaticamente e mantém o histórico versionado. Veja [sincronizar saves entre vários PCs](/guides/sync-game-saves-across-pcs) para o uso no dia a dia.

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

Sim, em qualquer NAS que corra Docker. O repositório inclui um template de Unraid, e a imagem desce para os `PUID`/`PGID` que indicares, para que as pastas montadas fiquem do utilizador certo em vez de root.

### Preciso de domínio e HTTPS?

Na tua própria rede local, não. A partir do momento em que o servidor é acessível de fora, põe um proxy inverso à frente e termina aí o TLS: Caddy, nginx ou Traefik servem.

### E se o meu servidor estiver em baixo quando acabo de jogar?

O snapshot é tirado localmente, por isso não se perde nada. Sobe sozinho assim que o servidor voltar a responder.

### Posso começar no Hoard Cloud e mudar mais tarde?

Sim, nos dois sentidos. Podes exportar tudo a partir da página da tua conta, e a aplicação pode ser apontada a outro servidor sem reinstalar nada.
