# Plan: Timeways online

Status: draft 1, 2026-10-03. The user decided the scope and the three features. Draft 2 takes in the account decision of the user: Battle.net login from the first version (section 5). It also adds voice examples from real NPC lines (section 22). Nothing is built. When a part is built, its rules move into `GAMEPLAY.md` (section 21 of this plan), and this plan marks the part as done.

This plan builds on these plans and does not repeat them:

- `docs/plans/quest-variety.md` owns the step kinds of a side quest and the check of an offer.
- `docs/plans/npc-knowledge.md` owns the kind of an NPC and `zones.toml`.
- `docs/plans/talk-window.md` owns the talk window and the two pools of model calls.
- `docs/plans/links.md` owns rows, calls, reads, and proof (5.14).
- Gnomish Relay SPEC 9.7 and 9.8 own the bridge, the app protocol, and the sandbox of the story program. This plan asks for changes there (section 6.7), and the relay session decides them.

## 1. Goal

Timeways online is a website, an API, and a database that we host. Players share three things, each one only when they turn it on:

1. **A quest library.** A player publishes a quest that they wrote. Other players get it on the website or in the game. In the game, it comes as an offer from a fitting NPC, like a side quest (3.4). Quests get likes, play counts, and reports. A guild or a roleplay group can keep quests for its members only.
2. **A game text library.** The addon already keeps the quest text, gossip, and books that the player read (5.10). Players share them, with `$N` for the name. A text joins the library only after several independent players sent the same text. Every player downloads the library. Each player's spoiler limit still holds on their own computer. Section 8 also gives a middle path: share only which texts exist and where, and never the words.
3. **Votes on AI text.** A thumbs up or down on a narrator line, a quest offer, or a talk answer. The vote carries the line, never a player name and never typed words. The best lines become candidates for the golden samples. The worst show weak prompts. A player can also report a lore problem in a `/lore` answer.

The pooled text also gives **voice examples**: real lines in the voice of an NPC for the talk prompt (section 22). Most of them come from your own reading, with no pool.

The same "several independent reports" rule also pools **world facts**: the title and the faction of an NPC, where it stands, and the continent of a zone. `npc-knowledge.md` needs these in a hand-written `zones.toml` today.

No hosted AI comes in this plan. A hosted model is a later business (relay SPEC 11.6). The API leaves room for it, and builds none of it.

The rules of `GAMEPLAY.md` 2 still hold:

- **No advantage.** A library quest gives story rewards only. No pooled data helps in combat, trade, or a race.
- **Private by default.** Every upload is off until the player turns it on. Downloads are off too. Section 15 lists what leaves the computer.
- **Only players with the addon take part.** A player with no Timeways sees nothing.
- **Only WoW Forever lore.** Every shared text passes the cutoff check (5.9) on the server and on the computer of each player.

## 2. What changes, in short

| Part | Change | Owner |
|---|---|---|
| `crates/online-rules` (new) | The checks that need no world: the shape and limits of a quest template, the banned words, the later names, the text hash, the independence rule. One crate for the server and the story program, so both give the same verdict. | Timeways |
| `crates/online-server` (new) | The API and the website, in one program. | Timeways |
| Accounts | "Log in with Battle.net" on the website (OAuth code flow). The desktop app links with a short code (RFC 8628 device flow). | Timeways |
| Database | PostgreSQL, 22 tables (section 13). | Timeways |
| Storage | Download files in object storage behind a CDN. | Timeways |
| Story program | `library.rs`, `sharing.rs`, `pooled.rs`, `votes.rs`, `world_reports.rs`, `review.rs`, `online_call.rs`, `voice_lines.rs` (new). New lines for the addon. | Timeways |
| Addon | A Sharing section in the game options, Publish on the quest form, thumbs, Report, `/quest add`. | Timeways |
| App protocol | New `online_call`, `online_answered`, `online_failed`, and `online_updated` lines. Protocol 3. | relay |
| Bridge | The HTTPS route to one host, the device code link, the token file with refresh, the `online` folder, a budget for each kind of call. | relay |
| Story sandbox | No change: no network. One more folder to read. | relay |

## 3. Who makes the calls

**The story program has no network** (relay SPEC 9.7, decision 9), and it keeps none. The bridge makes every outside call, as it does for a model call. The pattern is the same:

1. The addon sends a line, as for any feature.
2. The story program decides what to send, checks it, and takes out every name.
3. The story program asks the bridge with `online_call`, as it asks for a model with `model_call`.
4. The bridge checks the shape, adds the token, and calls the one API host with `curl`.
5. The bridge answers with `online_answered` or `online_failed`.

So no addon and no story program ever holds the token, and no call reaches any host but ours.

```
 WoW client            desktop                                               our servers
 ┌──────────┐ strip  ┌──────────────────┐ online_call ┌───────────────┐      ┌─────────────────┐
 │ Timeways │ ─────▶ │ Gnomish Relay    │ ◀────────── │ timeways-     │      │ online-server   │
 │  addon   │        │ bridge           │ ──────────▶ │ story         │      │  API + website  │
 │          │ ◀───── │  token file      │ online_     │  (no network) │      │                 │
 └──────────┘ slots  │  curl, https,    │ answered    │  reads        │      │ PostgreSQL      │
                     │  one host only   │ ──── HTTPS ─┼───────────────┼────▶ │                 │
                     │  writes online/  │             │  online/ (ro) │      │ object storage  │
                     └──────────────────┘             └───────────────┘      │  + CDN (files)  │
                                                                             └─────────────────┘
 browser ──────────────────────────────────── HTTPS ──────────────────────────▶ website
```

## 4. Architecture

### 4.1 The parts

| Part | Job |
|---|---|
| **API** | JSON over HTTPS, under `/v1/`. The bridge and the website use it. |
| **Website** | Server-rendered HTML pages from the same program: browse quests, a quest page, a group page, "Delete my data", the legal pages, and the pages of the maintainers. |
| **Database** | PostgreSQL. Every table of section 13. |
| **Files** | The download files (section 6.4): the quest feed, the text library, the world facts, and the samples. A job builds them each hour, and puts them in object storage. A CDN serves them. |
| **Jobs** | One loop in the same program: build the files, count independent reports, give out review jobs, end old reviews, delete old IP data. |

### 4.2 The recommended stack

The stack is small, and it uses the language of this repo, so the checks live in one crate.

| Layer | Pick | Why |
|---|---|---|
| Language | Rust | `online-rules` is shared with the story program. One verdict, one set of tests. |
| HTTP | `axum` on `tokio` | Common, and small. |
| Database access | `sqlx` with plain SQL | The SQL is readable, and `sqlx` checks it against the schema at build time. |
| Pages | `askama` templates, and a little `htmx` | Server pages work with no JavaScript build. |
| Database | PostgreSQL 17 | Plain tables, JSON columns for steps, good backups. |
| Files | An S3-compatible store (Cloudflare R2 or Backblaze B2) | No fee for downloads on R2. The text library is the biggest download. |
| CDN | Cloudflare, free plan | Caches the files and the website. It also stops simple floods. |
| Deploy | One container image, built in CI | The same image runs on any host. |
| Errors | Logs to stdout, and an uptime check | Nothing to run at the start. |

**Not picked:** a separate front-end app (more code, no gain for a few pages), a queue server (one jobs loop is enough under 100k players), and Redis (the rate limits live in memory with one server, and in PostgreSQL with two).

### 4.3 Hosting and cost

Rough monthly costs, from public price lists of 2026. Check them again before the launch.

| Players | Setup | Rough cost each month |
|---|---|---|
| 100 | One small VPS (2 vCPU, 4 GB) with PostgreSQL on it. A nightly backup to object storage. Files on R2 and the free CDN. A domain. | 10 to 15 USD |
| 10,000 | One VPS (4 vCPU, 8 GB) for the program. A small managed PostgreSQL with daily backups. Files on R2: about 50 GB of downloads each month, with no download fee. | 40 to 80 USD |
| 100,000 | Two program servers behind a load balancer. Managed PostgreSQL with a standby. Files on R2: 0.5 to 1 TB of downloads each month after a big patch. Paid logs and monitoring. | 300 to 500 USD |

Why the numbers stay small:

- A player syncs once a day, and sends a few votes and texts. 100,000 players make about 1 million calls a day: about 12 calls each second on average.
- The text library is about 10 MB of text, and about 3 MB compressed. Each player downloads it once, and then only the changes.

**The real cost is people.** At 100,000 players, about 1,000 new quests can come each month. Community review (11.4) takes most of them, but the review queue still needs a person for some minutes each day. Reports need the same.

### 4.4 The repo

The server lives in this repo, so the shared checks need no release step.

```
crates/online-rules/     the checks with no world, used by the server and the story program
crates/online-server/    the API, the website, the jobs
  migrations/            SQL, one file for each change
  templates/             the pages
  tests/                 API tests against a real PostgreSQL in CI
```

`Cargo.toml` adds both crates to the workspace. `cargo deny check` covers the new dependencies.

## 5. Accounts

The user decided on 2026-10-03: **Battle.net login is the account system from the first version.** The website logs in with Battle.net. The desktop app links to the account with a short code (5.2). No password ever goes into the game or the terminal.

### 5.1 Log in with Battle.net

The website is a client of the Blizzard OAuth service. A maintainer registers it on develop.battle.net, and gets a client id and a client secret. The secret lives only in the config of the server.

**The flow:** the OAuth 2.0 authorization code flow, with `state` and PKCE (S256).

1. The player clicks "Log in with Battle.net". The server makes a random `state` and a PKCE verifier, keeps them in a short cookie, and sends the browser to `https://oauth.battle.net/authorize` with the scopes `openid wow.profile`.
2. The player logs in on the Blizzard page. We never see the password.
3. Blizzard sends the browser back to `/login/callback` with a code. The server checks `state`, and trades the code and the verifier for an access token at `https://oauth.battle.net/token`.
4. With that token, the server reads `https://oauth.battle.net/userinfo`: the account id (`sub`) and the BattleTag. It reads the WoW characters from the Profile API (5.3).
5. The server makes or finds the contributor of that `sub`, opens a website session, and **drops the Blizzard token**. It keeps no Blizzard token on disk or in the database.

- **Scopes.** `openid` gives the account id and the BattleTag. `wow.profile` gives the list of WoW characters. We ask for no other scope.
- **Regions.** `oauth.battle.net` serves the Americas, Europe, Korea, and Taiwan. China has its own service, and the first version leaves it out (open question 7).
- **The session** of the website lasts 30 days, in a cookie with `HttpOnly`, `Secure`, and `SameSite=Lax`. Log out ends it.
- **A new character list** needs a new Blizzard token, so "Refresh my characters" on My page runs the login again. Blizzard usually skips its page when the player is still logged in there.
- **One account is one contributor.** Each computer of one account is the same contributor. So the independence rule (12.2) counts one Battle.net account once, also with five computers.

### 5.2 Link the desktop app: the device code flow

The desktop app has no browser and no password box. So it links with the device authorization grant of RFC 8628. Our server is the authorization server of this flow. Blizzard is not part of it: the player proves the account through the website login of 5.1.

**The steps.**

1. The player clicks "Link this computer" in the Sharing section in the game, or runs `gnomish-relay online link`.
2. The bridge asks `POST /v1/device/code`. The answer:

   ```json
   {"device_code": "<43 characters>", "user_code": "KXM-492", "verification_uri": "https://<site>/link", "verification_uri_complete": "https://<site>/link?code=KXM-492", "expires_in": 600, "interval": 5}
   ```

3. The bridge opens `verification_uri_complete` in the default browser of the computer, and shows the code in the game and in the terminal: "Your browser opened. Log in with Battle.net, then enter KXM-492."
4. The website asks for the Battle.net login when there is no session. Then it shows the code box, filled from the link, and the kind of computer that asked ("A Windows computer, 1 minute ago"). The player checks the code and clicks "Link this computer".
5. Meanwhile, the bridge asks `POST /v1/device/token` with the device code, each `interval` seconds. When the player has clicked, the answer holds the tokens. The bridge writes them into `online.token` in its config folder, with mode 0600, and the game says "This computer is linked."

**The user code.**

- 3 letters, a dash, and 3 digits: `KXM-492`. The letters come from 20 letters with no look-alikes (no `I`, `L`, `O`, `S`, `Z`, or `U`), and the digits from `2` to `9`. So about 4 million codes.
- The website takes it in any case, with or without the dash.
- It lives 10 minutes, and works once. A code that is still open is never given twice.
- The code page takes at most 5 wrong codes from one session in 10 minutes, and 60 from one network. With 10 minutes of life, a guess almost never finds an open code.
- **The trap of a guessed code.** A guesser who enters an open code links the computer of the player to the account of the guesser. The uploads of that computer then go to the wrong account. So the game always shows the account of the link: "Linked to Ada#1234". The BattleTag shows only on the player's own screen. A player who sees a wrong BattleTag clicks "Unlink this computer".

**The device code** is 32 random bytes in base64url. The server keeps only its SHA-256.

**Polling** (RFC 8628 section 3.5):

| Answer | The bridge |
|---|---|
| `authorization_pending` | asks again after `interval` seconds |
| `slow_down` | adds 5 seconds to `interval`, and asks again |
| `access_denied` | stops. The player clicked "Cancel" on the website. |
| `expired_token` | stops. The game says "That code expired. Try again." |
| `200` with tokens | stops, and writes the tokens |

The bridge stops at `expires_in` in any case, and polls off its main loop. One link runs at a time.

**The tokens.**

| Token | Life | Form | Where |
|---|---|---|---|
| Access token | 1 hour | 32 random bytes, opaque | `online.token`; the server keeps a SHA-256 |
| Refresh token | 90 days from its last use | 32 random bytes, opaque | `online.token`; the server keeps a SHA-256 |

- **Refresh.** The bridge calls `POST /v1/device/refresh` when the access token has less than 5 minutes left, or after a `401`. The answer holds a new access token and a **new refresh token**. The old refresh token ends.
- **Reuse ends the link.** An old refresh token that comes again means a copy of the file. The server then revokes the whole link: both tokens and every later one of it. The player links again.
- **A refresh that fails** with `invalid_grant` means the link ended. The bridge deletes `online.token`. The next online call gets `online_failed` with `unlinked`, and the game says "This computer isn't linked anymore. Link it again in the Sharing options."
- Each call carries `Authorization: Bearer <access token>`. `curl` reads the header from a file (`-H @file`), so a token is never in the arguments of a process.
- The bridge is a public client: it has no secret. The device code, the PKCE of 5.1, and the short life of each code protect the flow.

**Revoke.**

- **On the website:** My page lists each linked computer: its kind, when it was linked, and when it was last used. "Unlink" ends its tokens at once.
- **In the game or the terminal:** "Unlink this computer" in the Sharing section, or `gnomish-relay online unlink`. The bridge calls `POST /v1/device/revoke` with its refresh token (as RFC 7009 does), and deletes `online.token` after the answer, or after 30 seconds with no answer.
- **Delete my data** (15.4) revokes every link of the account.
- A ban of a maintainer revokes every link too.

### 5.3 Characters

**First source: the Blizzard Profile API.** At each login, the server reads the WoW characters of the account with the scope `wow.profile` (`GET /profile/user/wow` of the region, with the namespace of the game). This is real proof: Blizzard says that this account owns this character.

- The server keeps the name, the realm, the region, and the faction of each character. It keeps nothing else of the answer: no level, class, race, gender, guild, or character id.
- These characters are **verified**.
- A character that the next list no longer holds loses its proof (a transfer or a deletion). A verified character that a later list of another account holds moves to that account.
- **The check before we rely on it.** The Profile API has namespaces for retail and for Classic. Whether one of them lists WoW: Forever characters is unknown (open question 3). A test with a real account decides it before step 3 of the build order. With no Forever namespace, no character is verified, and the fallback below is the only source.

**Fallback: characters that the desktop app reports.** With "Add my characters from the game" on, the story program sends the realm and the name of each character that enters the game (`report_character`). These characters are **not verified**: a hacked addon can send any name.

- A player sees their characters on My page, with "Not verified" next to the ones that only the game sent.
- A reported character that the Profile API later lists becomes verified.
- A character is "not verified" for good on the account that reported it, also when another account reported it too.

**The rule of public names.** A character name shows in public only when it is verified: for example "A quest by Ada of Stormrage" on a quest page and in an offer line. A not-verified character never shows in public. So nobody can publish under the name of another player's character.

**What a byline can be.** At each publish, the author picks how the quest names them: "Anonymous", their display name (a name that they type, checked by the words check), or a verified character. The BattleTag is never a byline.

### 5.4 Rejected for now: an install key with no login

The first draft offered an anonymous install key: the desktop app made a random key, with no login. It is rejected for now, for these reasons:

- **No proof of a real player.** A new install is a new key, and costs nothing. The rule of 3 independent reports (12.2) and community review (11.4) rest on accounts that are hard to fake. With keys alone, one person with a script is many "players".
- **No public names.** Only the Profile API proves that a player owns a character. With no login, no quest can say "by Ada of Stormrage".
- **No recovery and no proof for deletion.** A lost key is a lost account. A request to delete its data has no proof of who asks.
- **One way is simpler.** Two account systems mean two login paths, two sets of tests, and a merge of accounts.

Look again if Blizzard ends our API access, or if many players cannot use Battle.net (for example China). The API keeps one "contributor" behind every call, so a second way can come later with no change of the other endpoints.

### 5.5 Maintainers

The pages of the maintainers (the review queue, reports, golden candidates, prompt stats, bans) need a login of their own: GitHub OAuth, with a list of allowed GitHub users in the config of the server. A player account never reaches these pages.

## 6. The desktop app and the addon

### 6.1 Every call goes through the bridge

The addon has no network: a WoW addon can only talk in the game. The story program has no network: its sandbox has none. So the bridge is the only program that talks to our servers.

### 6.2 New lines between the story program and the bridge

| Line | From | Fields |
|---|---|---|
| `online_call` | story program | `call` (a number), `op` (a name of the list below), `body` (bounded JSON) |
| `online_answered` | bridge | `call`, `status` (the HTTP status), `body` (bounded JSON) |
| `online_failed` | bridge | `call`, `reason`: `off`, `unlinked`, `limit`, `busy`, `offline`, `too_big`, or `failed` |
| `online_link_ended` | bridge | `result`: `linked`, `expired`, `denied`, or `failed`, and `battletag` when `linked`. The end of a device code link (5.2). |
| `online_updated` | bridge | `files`: the names of the files that changed in the `online` folder |

The ops, with their HTTP call and their budget in the bridge:

| `op` | HTTP | Budget in the bridge | Feature |
|---|---|---|---|
| `link_start` | `POST /v1/device/code` | 10 a day | any upload |
| `unlink` | `POST /v1/device/revoke` | 10 a day | any upload |
| `sync` | `GET /v1/sync` with a link, `GET /v1/manifest` with none | 30 a day | any download |
| `report_character` | `POST /v1/characters/reported` | 20 a day | characters |
| `publish_quest` | `POST /v1/quests` | 10 a day | quests |
| `withdraw_quest` | `DELETE /v1/quests/{code}` | 20 a day | quests |
| `get_quest` | `GET /v1/quests/{code}` | 60 a day | quests |
| `rate_quest` | `PUT /v1/quests/{code}/rating` | 60 a day | quests |
| `played` | `POST /v1/quests/{code}/plays` | 60 a day | quests |
| `join_group` | `POST /v1/groups/join` | 10 a day | quests |
| `report` | `POST /v1/reports` | 20 a day | any |
| `share_texts` | `POST /v1/texts` | 24 a day, 50 texts each | texts |
| `share_world` | `POST /v1/world-facts` | 24 a day, 200 facts each | world facts |
| `vote` | `POST /v1/votes` | 24 a day, 20 votes each | votes |
| `review_answer` | `POST /v1/reviews/{id}` | 10 a day | review |
| `delete_me` | `DELETE /v1/me` | 3 a day | any |

- The bridge maps each op to its call. The story program never names a URL, a method, or a header.
- **Which calls need a link.** Every op that sends something needs a linked computer (5.2). The public reads need none: the manifest, the public files, and `get_quest` of a public quest. So "Get quests from other players" and "Use the shared game text" work with no Battle.net login. An op that needs a link, on a computer with none, gets `online_failed` with `unlinked` at once, and sends nothing.
- **Inside the bridge, not ops:** the polls of `POST /v1/device/token` (5.2), at most one each `interval`, and `POST /v1/device/refresh`, at most 48 a day. The story program never sees a token.
- The `body` is bounded JSON, as the journal is (relay SPEC 9.8): at most 32 KiB, depth at most 6, no control character, keys of `[a-z_]`.
- The bridge holds at most 2 open online calls. A third gets `online_failed` with `busy`.
- Each budget is a limiter of S14 with its own window. A hostile addon that drives Timeways sends at most these counts.
- `online_answered` carries at most 64 KiB. A longer answer fails with `too_big`. The story program reads the body as hostile text.
- A call with no answer in 30 seconds fails with `offline`.

### 6.3 The HTTPS route

The bridge calls `curl` as it does for a local model, with these differences:

- `--proto =https`, `--max-redirs 0`, `--max-time 30`, `--compressed`, `--fail-with-body`, and the header file of the token.
- The host is `[online] api` of the config. Setup writes our host. A host that is not HTTPS is refused when the config loads.
- The body goes in on stdin, never in the arguments.
- The bridge reads at most 64 KiB of an answer, and at most the size in the manifest for a file (6.4).
- No proxy setting of the environment reaches `curl`, as for a model call.

### 6.4 Downloads: the `online` folder

`sync` answers with a manifest: the name, size, SHA-256, and URL of each file. With a link, it also names the files of the player's groups, and up to 3 review jobs (11.4). With no link, the public manifest names only the public files.

| File | Holds | Size cap |
|---|---|---|
| `quests.jsonl` | The quest feed: the 500 most liked public quests for the locale, the quests that the player added, and the quests of their groups | 4 MiB |
| `texts.jsonl` | The text library of the locale (section 8) | 64 MiB |
| `world.jsonl` | The pooled world facts (section 9) | 4 MiB |
| `samples.jsonl` | The golden samples that the maintainers approved (10.2) | 256 KiB |

- The bridge downloads only a file whose SHA-256 changed. It writes the file into `<data>/timeways/online/` as `<name>.new`, checks the size and the SHA-256, and then renames it. A failed check keeps the old file.
- The story sandbox gets this folder to read, never to write. A missing folder is not bound, as for a missing lore pack (relay SPEC 11.4).
- After the rename, the bridge sends `online_updated` with the names. The story program reads those files again. With no `online_updated`, it reads them at its start.
- The bridge downloads only the files that the `sync` body asks for. So a player with the text library off never downloads it.
- The story program reads each file as hostile text: one JSON object on each line, each line checked, a bad line skipped.

### 6.5 When a sync runs

The story program has no clock of its own, so it asks for a sync on a line from the addon:

- at the first `character_entered` of a run, when the last sync is more than 1 hour old;
- on any line, when the last sync is more than 24 hours old;
- at once, after `/quest add` or a join of a group.

The time of the last sync lives in `timeways.sqlite`, next to the budget of the narrator (5.7). Only a sharing choice that downloads turns the sync on.

### 6.6 New addon lines

Each one is a game event with no reply of its own. The bridge passes a game event as free JSON (relay SPEC 9.8), so the addon lines need **no relay change**. The answer comes as a `notice` and in the journal, as for a quest offer.

| Line | Fields | When |
|---|---|---|
| `sharing_set` | `quests`, `texts`, `votes`, `world`, `review`, `characters` (booleans), `display_name` (optional) | The player changes the Sharing section |
| `quest_publish_asked` | `title`, `text`, `steps`, `giver`, `levels`, `byline`, `group` (optional) | Publish on the quest form |
| `quest_withdraw_asked` | `code` | Remove on a published quest |
| `quest_add_asked` | `code` | `/quest add <code>` |
| `quest_rated` | `code`, `liked` | Yes or No after a library quest |
| `group_join_asked` | `code` | `/quest group <code>` |
| `report_asked` | `what` (`quest`, `lore`), `ref`, `reason` | Report |
| `line_voted` | `ref`, `up` | A thumb |
| `link_asked` | none | Link this computer |
| `unlink_asked` | none | Unlink this computer |
| `online_delete_asked` | none | Delete my online data |

A text that the story program already has (a seen text, a model line, a quest) goes by its row number (`ref`), never as text again. So the addon never sends model text back to the desktop, and a hostile addon cannot vote for a line that the model never wrote.

### 6.7 The message for the relay session

Send this list to the Gnomish Relay session. Per the project rule, tell it again when each part lands.

1. App protocol 3: `online_call`, `online_answered`, `online_failed`, `online_updated` (6.2), with `deny_unknown_fields` and the bounded JSON of the journal for `body`. The bridge accepts story protocol 2 and 3.
2. The op table of 6.2: the map from op to method and path, and one S14 limiter for each op with the counts of the table. No new theorem: each limiter holds alone.
3. At most 2 open online calls, apart from the model calls.
4. The HTTPS route of 6.3, in its own module (`online_http.rs`), with the fake server tests of the model route: a redirect, a 500, a huge answer, a slow answer, a proxy in the environment, and an HTTP host in the config.
5. The device code link of 5.2: `link_start` asks for the code, opens `verification_uri_complete` in the default browser (`xdg-open`, `open`, or `start`, with no shell), shows the code in the terminal of `gnomish-relay online link`, and answers the story program with the user code. Then the bridge polls `POST /v1/device/token` off its main loop, with `slow_down` and the end of life of RFC 8628, and sends `online_link_ended` at the end. One link at a time.
6. `online.token` in the config folder, mode 0600, in `deny_folders` with a named test. It holds the access token, its end time, and the refresh token. The bridge refreshes it before the end and after a `401`, and writes the new pair with a rename, so a crash never leaves half a file. An `invalid_grant` deletes it. `unlink` and `delete_me` revoke the tokens, then delete it.
7. `gnomish-relay online link` and `gnomish-relay online unlink` run the same steps from the terminal.
8. The `online` folder (6.4): the download, the checks, the rename, the read-only bind in the story sandbox, and `online_updated`.
9. Config: `[online] api` (HTTPS only) and `[online] enabled` (default `true`). With `enabled = false`, every `online_call` fails with `off` at once, and the bridge sends nothing. This is the switch of the desktop for a player who never wants any call.
10. `status` gets one line: "Timeways online: off", "Timeways online: on, not linked", or "Timeways online: linked to Ada#1234 (last sync 2 hours ago)".
11. A local log: each call that left the computer, with its op, its time, and its size, in `<data>/timeways/online.log`, the newest 1,000 lines. `gnomish-relay online log` prints it. So a player sees what left the computer.
12. A fuzz target `online_http` for the parse of the answer, the manifest, and each answer of the device flow.

### 6.8 Sandbox impact

- **The story program:** no change of its walls. It still has no network. It gets one more folder to read.
- **The bridge:** a new outbound route to one host. The bridge already downloads the lore dump and the releases with `curl`, so this is a new use of a known tool, not a new tool.
- **The coding agents:** none. The token is in the config folder, which the classifier already denies to every agent.
- **Windows:** the story program has no sandbox there (5.12). The online calls still go only through the bridge, so a hacked story program on Windows has the network of any program, as today.

### 6.9 What a hostile addon gains

A hostile addon can already send any text out of the game: a whisper to a friend of the attacker. So the online lines add no new way out. A hostile addon that drives Timeways can publish junk under the player's name, vote, and report, inside the budgets of 6.2. The review and the reputation of section 12 bound the harm, and the player sees each publish in the journal.

## 7. The quest library

### 7.1 The template

A published quest is a **template**: a quest with no world. Each player's story program checks it against their own world before it offers it.

```json
{
  "format": 1,
  "title": "Rats in the Cellar",
  "text": "The cellar is full of rats again, $N. Clear them out, and I'll tell everyone who did it.",
  "genre": "errand",
  "giver": {"kind": "host", "zone": "Elwynn Forest"},
  "faction": "alliance",
  "levels": [5, 12],
  "locale": "enUS",
  "steps": [
    {"goal": "kill", "creature": "Cellar Rat", "count": 6},
    {"goal": "meet", "npc": "Innkeeper Farley"}
  ]
}
```

| Field | Rule |
|---|---|
| `title` | 1 to 60 bytes. No `\|`, no control character, no emoji, no banned word, no name after the cutoff. |
| `text` | 0 to 400 bytes, with the same rules. `$N` is the only `$` code. |
| `genre` | A genre of quest-variety.md 3.1. |
| `giver` | `{"npc": "<exact name>"}`, or `{"kind": "<kind>", "zone": "<zone>"}` with a kind of npc-knowledge.md 5.1. |
| `faction` | `alliance`, `horde`, or `null` for both. |
| `levels` | The lowest and the highest level, 1 to 60, lowest first. |
| `locale` | The client locale of the author, such as `enUS`. Names match byte for byte, so a quest goes only to players of its locale. |
| `steps` | 1 to 4 steps of the kinds below, with the limits of quest-variety.md 4.11. |

**The step kinds of a template:**

| Kind | From a player quest (4.7) | In a template |
|---|---|---|
| `visit` | Go to a place | yes |
| `meet` | Talk to an NPC | yes |
| `kill` | Defeat a creature | yes, 1 to 10, as a side quest |
| `carry` | Bring an item to the giver | yes: you keep the items (quest-variety.md 4.9) |
| `talk`, `emote`, `visit_at`, `enter`, `defeat`, `wait`, any order | none | yes, with the rules of quest-variety.md |
| Defeat a player, find a player | yes | **no**: a template never names a player |
| `other` (as written) | yes | **no**: no NPC can judge it |
| `slap` | none | only in a comic quest, as quest-variety.md 4.10 |
| `game_quest`, `level` | none | not in format 1 |

**The reward** of a template is story only: trust from the giver, a line in the journal, and a deed. The money and the items of a player quest never go out. A promise from a stranger is no reward.

### 7.2 Publish, in the game

1. The quest form of 4.7 gets a **Publish** button next to Save, on a saved quest.
2. Publish opens a small panel: "Who gives it?" (an NPC that you met, or "Any innkeeper in Goldshire"), the levels (your level, plus and minus 3, to change), "Who can get it?" (Everyone, or one of your groups), and "Show my name as" (Anonymous, your display name, or a verified character, 5.3). A computer that is not linked shows "Link this computer" in place of Publish.
3. The addon takes out the names of known players (`TaskNames.lua`), as for "Help me write". It refuses a quest with a step that has no template kind, and names the step: "Remove the step "Find Ada" first. Shared quests can't name players."
4. The addon sends `quest_publish_asked`. The panel says "Checking...".
5. The story program checks the template (7.3). A fail comes back as a notice with the reason, and the panel keeps the quest.
6. The story program sends `publish_quest`. The answer holds a short code, such as `7KQ4MX`, and the state `in_review`.
7. The Quests page shows the quest under "Shared Quests" with its state: "In review", "Published", or "Not published", with the reason.

With no model, step 5 runs only the code checks, and the server review takes longer.

### 7.3 The checks of a template

In order. The first fail stops the publish.

1. **The shape and the limits** (`online-rules`): the table of 7.1. The same function runs on the server.
2. **The words** (`online-rules`): banned words, later names (5.9), a URL, a web address, an e-mail address, a chat channel name, and a run of more than 6 digits.
3. **The world of the author** (story program): each place, NPC, and creature passes the side quest check of 3.4 for the author. So an author can only name what they saw in the game. This also stops typos.
4. **No overlap with game quests** (story program): the rule of 3.4 for the quests that the author read.
5. **The local AI** (story program, 11.1): one model call of the story pool.

### 7.4 Get a quest

Three ways:

- **A code.** The website shows "In WoW, type /quest add 7KQ4MX". The story program asks `get_quest`, keeps the template, and the Quests page lists it under "Added Quests" until a fitting NPC offers it.
- **The feed.** With "Get quests from other players" on, the sync downloads `quests.jsonl` (6.4). The story program picks from it (7.5).
- **A group.** `/quest group <code>` joins a guild or a roleplay group. Its quests come in the feed of each member.

The story program keeps at most 50 added quests, and the newest 500 of the feed. It never shares which quests a player added: the feed is the same file for every player of a locale.

### 7.5 The offer, from a fitting NPC

`/quest` at an NPC works as today (3.4), with one new first step:

1. **Find a template that fits.** A template fits the NPC when all of these hold:
   - its giver is this NPC by its exact name, or its kind is the kind of this NPC (npc-knowledge.md 5) and its zone is the zone of the NPC;
   - your level is inside its levels, and its faction is yours or `null`;
   - it passes the side quest check of 3.4 against **your** world: you know every place, NPC, and creature. So a library quest never spoils a place that you have not seen;
   - it passes the no-repeat rules of quest-variety.md 3: its shape and its title words are new;
   - you never accepted it before.
2. **Pick.** An added quest that fits comes first. Else a fitting quest of the feed comes in at most one offer of three, from a count, as the hero hook of npc-memory.md 10.2 does. The most liked one wins, then the newest.
3. **Offer.** The offer is a side quest, with the template's words. `$N` becomes the name of the character in the addon. The offer line adds the byline: "Written by Ada of Stormrage." for a verified character, the display name, or nothing for Anonymous. No model call runs, so a library quest also works with no model.
4. **Else** the model writes a quest, as today.

A library quest is a side quest from there on: the same accept, progress, kills, journal, and limits (3 open quests). Its row keeps the code of the template, and its proof root is Shared (5.14): another player wrote it.

### 7.6 Likes, plays, and reports

- **Plays.** At accept and at the end of a library quest, the story program sends `played` with `accepted` or `done`. The server counts each contributor once for each quest. A play count is the word of each player's computer, so the website calls it "plays", never "verified".
- **Likes.** After the end, the Quests page asks "Did you enjoy this quest?" with Yes and No. Only a contributor who sent `done` can rate. A rating can change, and it counts once.
- **Reports.** Report on a library quest, in the game or on the website, with a reason: Offensive, Spam, Names a real person, Breaks the lore, Doesn't work, Other. Section 11.3 says what happens next.
- **Withdraw.** The author removes a quest with Remove on the Quests page or the website. It leaves the feed at the next file build. A player who holds it keeps it, because their copy is a side quest of their world.

### 7.7 Guilds and roleplay groups

- A group has a name, a kind (`guild` or `rp`), an owner, and an invite code. The server does not check that a guild exists in the game: a group is only a list of contributors.
- The owner makes the group on the website, and shares the code in the game. `/quest group 4XPL9R` joins.
- A quest with "Who can get it?" set to a group goes only to its members, and skips community review: the owner reviews it on the group page. Reports still reach the maintainers.
- The owner can remove a quest from the group, remove a member, and make a new code.

### 7.8 The website

| Page | Holds |
|---|---|
| Quests | A list with filters (zone, level, faction, genre) and sorts (Most liked, Most played, Newest) |
| A quest | The title, the text with "you" in place of `$N`, the steps in the words of the game (quest-variety.md 11), the giver, the levels, likes, plays, Report, and the line "In WoW, type /quest add 7KQ4MX" with Copy |
| An author | The byline (a display name or a verified character) and the public quests under it. A quest with the byline Anonymous never shows on an author page. |
| A group | Its quests and its members (members only), invite code (owner only) |
| Log in | "Log in with Battle.net" |
| Link | The code box of the device flow (5.2) |
| My page | My quests with their state, my groups, my characters (verified or not), my linked computers with Unlink, my sharing counts, Refresh my characters, Delete my data |
| Help | How to install Timeways, how to turn on sharing, the rules for quests |
| Terms, Privacy, Takedown | Section 16 |

The website never shows game text, a vote, or a world fact. It shows only quests that players wrote.

## 8. The game text library

### 8.1 What a player shares

With "Share game text" on, the story program sends each new seen text (5.10) of each character, in batches of at most 50, at most once each hour:

```json
{"kind": "quest", "title": "The Fargodeep Mine", "npc": "Marshal Dughan", "zone": "Elwynn Forest", "locale": "enUS", "build": "1.60.1.70009", "text": "...", "hash": "<64 hex>"}
```

- The text is the one in the `learned` table: one line, `$N` in place of the name of the character, at most 2000 bytes.
- **Never sent:** a rumor (a `/talk` answer, 3.1.1), a letter that a player wrote (5.10), the time when the player read it, the character, and the realm.
- **One more name check.** The story program drops a text that holds the name of a known player (the alias table, 5.11) or the realm name. Game text from the server holds no such name, so a hit means a strange text, and it stays home.
- A text goes once for each contributor. The story program marks it as sent.

### 8.2 The same text

The hash is SHA-256 of `kind`, `locale`, `title`, `npc`, and `text`, joined with a zero byte. The zone is not in the hash: the same book in two places is the same text. The server keeps the zone that most reports give.

Some texts change with the reader: race, class, or gender ("Greetings, night elf"). Each form is its own text, and each one needs its own reports. Most texts have one form.

### 8.3 When a text joins the library

A text joins the library when **3 independent contributors** sent the same hash (12.2). Until then, it waits on the server, and nobody downloads it.

When two texts claim the same `kind`, `title`, and `npc`, with different words, both can join. A text of the same slot with 3 more independent reports than another wins, and the other goes to the review queue. A Blizzard hotfix that changes a text gives two forms for a while. Both are canon of their time.

### 8.4 Use on the computer of each player

With "Use the shared game text" on, the sync downloads `texts.jsonl`. The story program indexes it in memory at its start, as it does for seen text.

- **A pooled text is a third source**, after your own text and before the lore pack: your text first, then the pool, then the pack (3.1.1).
- **The spoiler limit holds.** A pooled quest text passes only when you hold that quest or turned it in. A pooled gossip text passes only when you met its NPC. A pooled book passes only when you visited its zone.
- **It is not your knowledge.** A pooled text never joins your `learned` table, never shows on the Knowledge page, and the answer never says "You read...". The source reads "players report that Marshal Dughan says". So two players still know different things (3.1.1).
- When you read the text yourself, your own copy wins.
- A pooled text has the proof root Shared (5.14).

### 8.5 The middle path: where texts are, not their words

Blizzard owns the game text. The middle path shares no words:

- A player sends only the hash, the kind, the title, the NPC, the zone, the locale, the build, and the length.
- The library holds the same fields. A text joins it with the same rule of 3 independent reports.
- The download holds no text. The words come only from the player's own reading.

What the middle path still gives:

- **The Knowledge page** shows what is left: "Elwynn Forest: 3 books you haven't read".
- **The narrator** can notice a first book of a place with the number of books there.
- **Quest overlap** (3.4) covers the titles, the NPCs, and the zones of every pooled quest, not only the quests that you read. So a side quest never steps on a game quest that you have not reached.
- **npc-knowledge** gets the NPCs of each zone from the pool.
- **A check of seen text.** A seen text whose hash is in the library is confirmed by other players.

What it loses: `/lore` cannot answer from a text that you did not read.

### 8.6 Which path to build

| | Full text | Middle path |
|---|---|---|
| Lore answers | Better: the words of the game for every place that you reached | As today: only your own reading |
| Blizzard text on our servers | Yes: we copy and send it to every player | No words. Titles and NPC names only. |
| Risk | A takedown, and a strain with Blizzard (section 16) | Low. Titles are short names. |
| Server load | About 10 MB per locale | About 1 MB |

The user accepts the risk of the full text. The build order (section 18) still builds the middle path first, because the full text uses everything that it needs, and the middle path keeps working if a takedown ends the full text. **The full text is one switch on the server**: a file build with or without the words. The story program reads both forms.

## 9. World facts

With "Share world facts" on, the story program sends facts that the addon already reads (npc-knowledge.md 3):

| Fact | From | Example |
|---|---|---|
| `npc_title` | `npc_seen` `title` | "Innkeeper Farley" is "Innkeeper" |
| `npc_faction` | `npc_seen` `faction` | "Innkeeper Farley" is alliance |
| `npc_zone` | the zone and the subzone of the sighting | "Innkeeper Farley" stands in "Goldshire", "Elwynn Forest" |
| `zone_continent` | `C_Map` of the zone (check with the API gate) | "Elwynn Forest" is in "Eastern Kingdoms" |
| `zone_parent` | the zone of a subzone | "Goldshire" is in "Elwynn Forest" |

- A fact joins `world.jsonl` after 3 independent reports of the same value.
- **The hand-written file wins.** `zones.toml` of npc-knowledge.md 6.1 stays the first source. The pool fills only what it does not hold. A conflict goes to the maintainers, who fix one of the two.
- A fact names only NPCs, zones, and titles, never a player. The addon never sends a player or a pet as `npc_seen` (3.4, 5.11).
- The story program sends each fact once, and again only when its value changes.
- A maintainer can turn a pooled fact into a line of `zones.toml`. The pool then needs nothing more for it.

## 10. Votes on AI text, and lore reports

### 10.1 What a vote carries

With "Rate AI lines" on, a small thumb up and thumb down shows next to:

- a narrator line on the Chronicle page,
- a quest offer on the Quests page,
- each NPC line in the talk window.

A click sends `line_voted` with the row of the line. The story program builds the vote:

```json
{"kind": "talk", "text": "Work? The mill has rats, and nobody wants them.", "up": true, "model": "claude-haiku", "prompt": "talk-3f9a", "samples": "npc_replies-71c2", "locale": "enUS"}
```

- `text` is the text as the model wrote it, before the swap of names. Each `{P7}` becomes `{P}`.
- `model` is the kind of the model: `claude-<model>`, or `local-<model>`. No path, no address.
- `prompt` is an id of the template of the prompt: the name of the call and the first 4 hex digits of the SHA-256 of its fixed text. The `samples` id works the same. So the maintainers see which prompt a line came from, and never the prompt itself. A prompt holds the world of the player.
- **Never sent:** the prompt, the player's typed words, the question of `/lore`, the name of the character, any name of the alias table.
- **The typed-words guard.** The story program drops a vote on a talk answer that repeats 3 or more words in a row of the player's words in that conversation, or that holds the name of the character. So a name that the player typed never leaves through a quote.
- A player votes once on a line. A second click changes the vote.

### 10.2 Golden samples

A line is a **candidate** when it has at least 10 up votes from independent contributors (12.2), and at least 80 % of its votes are up.

- A maintainer reads each candidate on the candidates page. The samples rule (3.2.1) says a sample names no real place or person, so the maintainer edits each name out, or drops the line.
- An approved sample goes into `samples.jsonl` of the next file build. The story program adds it to the samples of its voice, after the samples of the repo. A sample from the file passes the same checks as a line of the repo: banned words, later names, the size.
- A sample from the file also goes into the repo at the next release, so a player with sharing off gets it too.

### 10.3 Weak prompts

The prompt stats page shows, for each `prompt` id, `samples` id, and `model`: the votes, the share of down votes, and the newest down-voted lines. A prompt with a high share of down votes is the first to rewrite. The page shows nothing about a player.

### 10.4 Lore reports

A `/lore` answer in the lore book gets a Report link. The reasons: Wrong, Spoils something, From a later expansion, Made up. The report carries:

- the answer text of the model,
- the sources of its passages (names of pages and books, never their words),
- the reason.

It never carries the question: the question is the player's typed words. A maintainer reads lore reports on the reports page. A report of a later name becomes a new line of `later_names.txt`, with its test.

## 11. Moderation

### 11.1 The first filter: the player's own AI

Before a quest leaves the computer, the story program asks the player's model one question. It is one call of the story pool.

```
You check a quest that a player wrote for other players of World of Warcraft. Answer in JSON:
{"ok": true} or {"ok": false, "reason": "<one of: hateful, sexual, harassment, real_person, spam, off_topic, lore>"}.
Refuse a quest that attacks a group of people, holds sexual content, targets a real person or player,
advertises anything, has nothing to do with the game, or tells of events after the first year of WoW.
The quest:
<<<
...
>>>
```

- The quest is fenced data (`house::fenced`).
- A refusal shows at once, with the words of 17: the player fixes it and tries again. An honest player gets the answer in seconds.
- A failed call, or no model, passes the quest to the server with a mark `no_local_check`. The server then sends it to human review.
- The client check helps honest players. A cheater can skip it, so the server never trusts it.

### 11.2 The server checks

Cheap and in order:

1. The token, the budget of the contributor (12.1), and the state of the contributor (not limited, not banned).
2. The shape, the limits, and the words of `online-rules` (7.3, steps 1 and 2).
3. **Duplicates.** The same normalized title and text as a public quest is refused. A text that is 90 % the same as 3 recent quests of other contributors goes to the queue.
4. **Holdback.** The first 3 quests of a contributor go to human review, always.

A quest that passes goes to community review (11.4). A group quest goes to its owner (7.7).

### 11.3 Reports and the queue

- A public quest with reports from 3 independent contributors leaves the feed at once, and goes to the queue. A report from a Trusted contributor (12.3) counts as 2.
- The queue page shows each item with its reports, its review votes, and its author's history. A maintainer picks Keep, Remove, or Remove and limit the author.
- Every action of a maintainer is a row of `moderation_actions`, with who and why.
- An author whose quest was removed sees "Not published" with the reason, on the Quests page and on My page.

### 11.4 Community review

Before a public quest goes into the feed, a few other players' AIs read it, in the background.

**Who reviews.** A contributor with "Help review quests" on, with a model, a linked computer, and at the level Trusted (12.3). Not the author. Not a member of a group of the author. Not from the network of the author (12.2). The server picks 5 at random from the reviewers who synced in the last 24 hours.

**The job.** A review job comes in the `sync` answer: the title, the text, and the steps. Nothing about the author. The story program runs the prompt of 11.1 with the player's model, and sends `review_answer` with `ok` and a reason. The player sees nothing: it is a background call.

**The verdict.**

| Votes | Result |
|---|---|
| 3 or more `ok`, and at most 1 not `ok` | Public. One in 10 still goes to a maintainer as a sample. |
| 2 or more not `ok` | The queue |
| Fewer than 3 answers after 72 hours | The queue |

**Why one cheater cannot fake it alone.**

- The server picks the reviewers at random from many. The author cannot pick.
- A reviewer must be Trusted: 30 days and 20 confirmed contributions. Many fake reviewers take months.
- A reviewer from the network or the group of the author is never picked.
- A reviewer whose vote goes against the final result 3 times in 30 days drops to Known, and reviews no more for 30 days.
- The 1-in-10 sample checks the reviewers too.

**The cost to the reviewer.** A review is one call of the story pool. A reviewer gets at most 3 jobs a day. A player with Claude through their login sees this in the setting: "Uses your AI a few times a day." So the setting is off by default for everyone.

**With few players.** Under 20 Trusted reviewers, every quest goes to the queue. Community review starts when the pool is large enough to pick 5 at random.

## 12. Abuse and poisoning

### 12.1 Rate limits

Two layers: the bridge budgets of 6.2 stop a hostile addon on one computer. The server limits stop a script that calls the API with no bridge.

| What | Each contributor | Each network (12.2) |
|---|---|---|
| Device codes asked | 10 a day | 30 an hour |
| Wrong codes on the link page | 5 in 10 minutes | 60 in 10 minutes |
| Linked computers | 5 at once | n/a |
| Quests published | 3 a day, 20 in review | 10 a day |
| Texts | 2,000 a day | 10,000 a day |
| World facts | 5,000 a day | 20,000 a day |
| Votes | 200 a day | 1,000 a day |
| Reports | 20 a day | 60 a day |
| Reviews | 3 a day (given by the server) | n/a |
| All calls | 600 an hour | 3,000 an hour |

A call over a limit gets `429` with the seconds to wait. The bridge turns that into `online_failed` with `limit`.

### 12.2 Independent reports

A text, a world fact, a vote for a candidate, and a report on a quest count only from **independent contributors**. Two reports are independent when all of these hold:

- They come from two contributors.
- They come from two Battle.net accounts. Each account is one contributor (5.1), so this is the same as two contributors. A verified character belongs to one account, so two accounts never share one.
- They come from two networks: the IPv4 /24 or the IPv6 /48 of each call differs.
- Each contributor is at the level Known or higher (12.3) when the count runs.

The network of a call is kept as a keyed hash of the prefix, with a key that changes each month, for 30 days. So the server can compare networks, and cannot name an address.

A report of a New contributor is kept. It starts to count when the contributor becomes Known. So a new player loses nothing, and a fake account waits a week to matter.

`online-rules` holds this rule as one pure function, `independent_count(reports) -> usize`, with property tests (19.3).

### 12.3 Reputation

Four levels, from rows that the server already has. No hidden score.

| Level | Rule | Effect |
|---|---|---|
| New | Less than 7 days since the first call, or no confirmed contribution | Reports wait. Quests go to human review. |
| Known | 7 days, at least 1 contribution that others confirmed, nothing removed in 30 days | Reports count. |
| Trusted | 30 days, at least 20 confirmed contributions, nothing removed in 90 days, and a verified character (5.3) | Can review. A report on a quest counts as 2. |
| Limited | A removed quest, a ban of a maintainer, or reviews against the result | Reports and votes do not count. Quests go to human review. Ends after 30 days with nothing removed. |

A confirmed contribution is a text or a fact that joined the library with the reports of others, or a public quest with more likes than reports.

### 12.4 Attacks and answers

| Attack | Answer |
|---|---|
| A fake game text, sent from many fake Battle.net accounts, to put false lore in `/lore` | 3 independent Known contributors from 3 networks. A pooled text is "players report", never your own reading. The cutoff check of 5.9 runs on every answer. Your own reading wins. |
| A flood of junk quests | The budgets, the holdback, the duplicate check, and community review |
| A hateful quest | The local AI, the words check, community review, reports, the queue |
| A quest that names a real player to harass them | The name strip of the addon, the local AI, the `real_person` reason, reports. A template never has a player step. |
| A ring of fake reviewers | Trusted needs 30 days and 20 confirmed contributions. Random picks. Network and group exclusions. The 1-in-10 sample. |
| A brigade of down votes on a quest | Only players who finished it rate. Each contributor counts once. |
| Report spam to remove a good quest | 3 independent reports only hide it. A maintainer decides. A report against the final result counts against the reporter. |
| Poisoned votes to make a bad golden sample | 10 independent up votes, and a maintainer reads every candidate |
| A poisoned download file | The story program reads every file as hostile text, with the checks of a quest, a sample, and a seen text. Later: the files carry a signature of an offline key (open question 13). |
| A stolen `online.token` | It reaches only the calls of one account. Each access token lives 1 hour. The first refresh of the thief or the player ends the whole link (5.2). "Unlink" on the website ends it at once. |
| Many free Battle.net accounts | A week and a confirmed contribution before reports count. Trusted needs a verified character, so a reviewer needs a WoW license, once the Profile API covers Forever (5.3). |
| A guessed device code | 5.2: about 4 million codes, 10 minutes of life, 5 tries in 10 minutes, and the game shows the BattleTag of the link. |
| A not-verified character used as a byline to pose as another player | Never shown in public (5.3). |
| A hostile addon that drives the uploads | 6.9 |

## 13. The data model

PostgreSQL. Every id is a `bigint` from a sequence, except the hash ids. Each table has `created_at`. A newtype in Rust wraps each id: `ContributorId`, `LinkId`, `CharacterId`, `QuestId`, and the rest.

**People and access**

| Table | Columns | Note |
|---|---|---|
| `contributors` | `id`, `bnet_sub` (unique), `battletag`, `display_name`, `level` (`new`, `known`, `trusted`, `limited`), `limited_until`, `first_call_at`, `banned_at` | One Battle.net account. The BattleTag shows only to its owner and to the maintainers. |
| `characters` | `id`, `contributor_id`, `region`, `realm`, `name`, `faction`, `proof` (`verified`, `reported`), `verified_at`, `reported_at` | 5.3. A verified `region`, `realm`, and `name` is unique. |
| `device_codes` | `device_code_hash`, `user_code` (unique while open), `client_os`, `expires_at`, `interval`, `last_poll_at`, `approved_by`, `denied_at` | 5.2. Deleted 1 day after the end. |
| `links` | `id`, `contributor_id`, `client_os`, `access_hash`, `access_expires_at`, `refresh_hash`, `refresh_expires_at`, `old_refresh_hashes`, `last_used_at`, `revoked_at` | One linked computer. An old refresh hash that comes again revokes the row. |
| `web_sessions` | `token_hash`, `contributor_id`, `expires_at` | A website session |
| `maintainers` | `github_id`, `name`, `added_at` | The pages of section 5.5 |
| `call_networks` | `contributor_id`, `network_hash`, `day` | 12.2. Deleted after 30 days. |

**Quests**

| Table | Columns | Note |
|---|---|---|
| `quests` | `id`, `code`, `author_id`, `byline` (`anonymous`, `display_name`, or a character id), `group_id` (null), `state` (`in_review`, `public`, `hidden`, `removed`, `withdrawn`), `format`, `locale`, `title`, `text`, `genre`, `giver` (JSON), `faction`, `level_min`, `level_max`, `steps` (JSON), `local_check` (`passed`, `none`), `published_at` | One template |
| `quest_reviews` | `quest_id`, `reviewer_id`, `assigned_at`, `answered_at`, `ok`, `reason`, `model` | 11.4 |
| `quest_plays` | `quest_id`, `contributor_id`, `accepted_at`, `done_at` | One row for each pair |
| `quest_ratings` | `quest_id`, `contributor_id`, `liked`, `updated_at` | One row for each pair |
| `groups` | `id`, `name`, `kind` (`guild`, `rp`), `owner_id`, `invite_hash` | 7.7 |
| `group_members` | `group_id`, `contributor_id`, `joined_at` | |

**Texts and facts**

| Table | Columns | Note |
|---|---|---|
| `texts` | `hash`, `kind`, `locale`, `title`, `npc`, `zone`, `length`, `body` (null on the middle path), `first_build`, `state` (`waiting`, `pooled`, `removed`) | 8.3 |
| `text_reports` | `hash`, `contributor_id`, `network_hash`, `zone`, `build`, `reported_at` | One row for each pair of text and contributor |
| `world_facts` | `id`, `kind`, `subject`, `value`, `locale`, `state` | 9 |
| `world_fact_reports` | `fact_id`, `contributor_id`, `network_hash`, `reported_at` | |

**Votes and reports**

| Table | Columns | Note |
|---|---|---|
| `lines` | `hash`, `kind`, `text`, `prompt_id`, `samples_id`, `model`, `locale`, `state` (`open`, `candidate`, `approved`, `dropped`), `approved_text` | One model line, by the hash of its text |
| `votes` | `line_hash`, `contributor_id`, `up`, `updated_at` | One row for each pair |
| `reports` | `id`, `reporter_id`, `target_kind` (`quest`, `lore`, `text`), `target_ref`, `reason`, `payload` (JSON: the answer and its sources for `lore`), `resolved_at`, `resolution` | |
| `moderation_actions` | `id`, `maintainer_id`, `target_kind`, `target_ref`, `action`, `reason` | Never deleted, except the rows of a deleted contributor become anonymous |

**Files**

| Table | Columns | Note |
|---|---|---|
| `file_builds` | `name`, `locale`, `sha256`, `size`, `url`, `built_at` | The manifest of `sync` |

**Rules of the data:**

- Only `characters` holds a character name and a realm. No table holds an IP address, a Blizzard token, or a typed word.
- `text_reports`, `world_fact_reports`, and `votes` keep the contributor so that a deletion can remove them, and so that each contributor counts once.
- A count is a query, never a column that a call adds to. So a deletion fixes every count at once.

## 14. The API

JSON, under `/v1/`. "Token" is the access token of a linked computer (5.2). "Session" is the cookie of the website (5.1). "None" is a public call. Errors are `{"error": "<code>", "message": "<a line for a person>"}`.

| Method and path | Auth | Does |
|---|---|---|
| `GET /login` | none | Sends the browser to Battle.net, with `state` and PKCE (5.1) |
| `GET /login/callback` | none | Checks `state`, trades the code, reads the account and the characters, drops the Blizzard token, opens a session |
| `POST /logout` | session | Ends the session |
| `POST /v1/device/code` | none | 5.2. Answers the device code, the user code, the links, `expires_in` (600), and `interval` (5). |
| `GET /link`, `POST /link` | session | The code box. `POST` approves or denies one user code for the account of the session. |
| `POST /v1/device/token` | none | The poll, with the device code. Answers `authorization_pending`, `slow_down`, `access_denied`, `expired_token`, or the tokens and the BattleTag. |
| `POST /v1/device/refresh` | none | With a refresh token. Answers a new pair, and ends the old refresh token. |
| `POST /v1/device/revoke` | none | With a refresh token. Ends the link. Always answers 200, as RFC 7009 does. |
| `GET /v1/me/links` | session | My linked computers |
| `DELETE /v1/me/links/{id}` | session | Unlink this computer |
| `POST /v1/characters/reported` | token | The fallback of 5.3: the realm and the name of a character from the game |
| `GET /v1/manifest?want=quests,texts,world,samples` | none | The manifest of the public files |
| `GET /v1/sync?want=quests,texts,world,samples` | token | The same, with the files of my groups and up to 3 review jobs |
| `POST /v1/quests` | token | Publish a template. Answers the code and the state. |
| `GET /v1/quests` | none | The public list, with filters and sorts. The website uses it. |
| `GET /v1/quests/{code}` | token or none | One template: public, or of a group of the caller |
| `DELETE /v1/quests/{code}` | token or session | The author withdraws it |
| `PUT /v1/quests/{code}/rating` | token or session | `{"liked": true}`. Only after a play with `done`. |
| `POST /v1/quests/{code}/plays` | token | `{"step": "accepted"}` or `{"step": "done"}` |
| `POST /v1/groups` | session | Make a group |
| `POST /v1/groups/join` | token or session | Join by code |
| `DELETE /v1/groups/{id}/members/{contributor}` | session | The owner removes a member, or a member leaves |
| `POST /v1/texts` | token | Up to 50 texts, or 50 hashes on the middle path |
| `POST /v1/world-facts` | token | Up to 200 facts |
| `POST /v1/votes` | token | Up to 20 votes |
| `POST /v1/reports` | token or session | One report |
| `POST /v1/reviews/{id}` | token | The answer of a review job |
| `GET /v1/me` | token or session | My data: BattleTag, characters, quests, groups, counts, level |
| `GET /v1/me/export` | session | Everything we hold about me, as one JSON file |
| `DELETE /v1/me` | token or session | Delete my data (15.4) |
| `GET /files/{name}` | none | A download file, through the CDN |

**Versions.** The path holds the version. A breaking change is `/v2/`, and `/v1/` keeps working for 6 months. Each answer of `sync` carries `min_story_version`. A story program below it gets the notice "Update Timeways to keep sharing." and sends no more online calls.

**The website** uses the same handlers through the session cookie, and a CSRF token on each form.

**The Blizzard calls** of the server are only two, both at a login: `userinfo` and the Profile API. Each one has a timeout of 10 seconds. A failed Profile API call keeps the old character list and still logs the player in.

## 15. Privacy

### 15.1 Opt-in

Every choice is off until the player turns it on. Each one is its own switch in the Sharing section. A switch that sends needs a linked computer (5.2). A switch that only downloads needs none:

| Switch | Sends | Downloads |
|---|---|---|
| Share my quests | The quests that you publish, plays, and likes | Your own quests' state |
| Get quests from other players | Plays and likes of library quests | `quests.jsonl` |
| Share game text | Seen texts (or their hashes on the middle path) | none |
| Use the shared game text | none | `texts.jsonl` |
| Share world facts | NPC titles, factions, zones | `world.jsonl` |
| Rate AI lines | Votes, lore reports | `samples.jsonl` |
| Help review quests | Review answers | Review jobs |
| Add my characters from the game | The realm and the name of each character that enters the game (5.3) | none |

The first switch that the player turns on shows one popup that says what leaves the computer, with a link to the privacy page. Turning a switch off stops its calls at once. `[online] enabled = false` on the desktop stops every call, whatever the game says (6.7).

### 15.2 What leaves the computer

- The access token of the link, and the IP address of each call (the server keeps only a keyed hash of the network, 30 days).
- The locale and the build of the client.
- For each switch, only the fields of its section: 7.1, 8.1, 9, 10.1, 10.4, 11.4.
- The byline that the player picks for each quest. The default is "Anonymous".
- With "Add my characters from the game" on, the realm and the name of your own characters.

### 15.3 What never leaves

- The name of any other player, and the guild name.
- The name and the realm of your own characters, unless "Add my characters from the game" is on.
- Typed words: `/lore` questions, `/talk` words, and the ideas of "Help me write".
- The world file, the chronicle, the hero sheet, player stories, and the prompts.
- The rewards and the players of a player quest.
- The level, the class, and the race of the character, except the level band that the author picks for a quest.

### 15.4 Delete my data

- **In the game:** "Delete my online data" in the Sharing section. It asks first: "Delete everything you shared? Your published quests come down too." The bridge sends `delete_me`, and deletes `online.token` after the answer.
- **On the website:** the same button on My page.
- **What goes:** the contributor, its characters, its links (each one revoked), its quests (they leave the feed at the next build), its plays, likes, votes, reports, text reports, fact reports, reviews, groups that it owns, and network rows. A text or a fact that falls below 3 independent reports leaves the next build. A moderation row keeps its action, with no contributor.
- **When:** at once in the database. Backups age out in 30 days.
- **Copies on other computers:** a player who accepted a deleted quest keeps it in their world, as a side quest. The privacy page says so.
- **A lost computer:** log in with Battle.net on the website, then Unlink it, or Delete my data. The Battle.net login is the proof of who asks.

### 15.5 What the website keeps from Battle.net

| Kept | Why |
|---|---|
| The account id (`sub`) | To know the same player at the next login |
| The BattleTag | To show the player which account a computer is linked to (5.2). Only the player and the maintainers see it. Never a byline. |
| The name, realm, region, and faction of each WoW character | Verified bylines (5.3). The faction keeps a quest of one faction from a byline of the other. |

| Never kept | |
|---|---|
| The Blizzard access token | It is used in the login request, then dropped. |
| The password, the e-mail address, the real name, the phone number, the payment data | Blizzard never sends them for our scopes. We never ask. |
| The level, class, race, gender, guild, gear, and play time of a character | We do not need them. The Profile API sends some of them, and the server drops them. |
| The Battle.net friends list | We never ask for it. |

## 16. Legal basics

This section lists what to prepare. It is not legal advice. A lawyer reads the terms and the privacy policy before the launch.

**Terms of use.** In plain words:

- You must be 13 or older (16 in some EU countries, for consent to data use), and have a Battle.net account in good standing.
- You wrote what you publish, or you have the right to share it.
- You give us a free, worldwide license to host, show, and send your quests to other players, and to change them to fit the game (for example, `$N`).
- No hate, no harassment, no sexual content, no real people, no ads, no cheats.
- We can remove anything, and end any account.
- Timeways is a fan project. It is not made by, endorsed by, or connected to Blizzard Entertainment.
- No warranty. The service can stop at any time.

**Privacy policy.** Who runs the service and how to reach them. What we collect (section 15), why (to run the features the player turned on), the legal basis (consent, for each switch), how long we keep each thing, who processes it (the hosting, the database, the CDN), and the rights: see, export, delete, and complain to a data authority. It names the data of 15.5, and Blizzard as the source of the login and the characters.

**Takedown.** A page and a mail address for copyright notices.

- In the US, a DMCA agent registered with the Copyright Office protects a host of user content. The fee is small. Register before the launch.
- A valid notice hides the item at once, and the author gets the reason and a way to answer.
- A contributor with repeated valid notices is banned.

**The Blizzard text risk.** Plainly:

- Blizzard owns the quest text, the gossip, and the books. The full text library of section 8 copies that text and sends it to every player. We have no license for that.
- Fan sites show quest text, and Blizzard often lets them. That is not a license, and it can end at any time.
- The Blizzard EULA forbids data mining. The addon reads only the text that the game shows to the player, but the pool collects it from many players.
- The likely result of a problem is a takedown notice. The worse results are a cease-and-desist letter, or action against the Battle.net accounts of the project.
- So the full text is one switch on the server (8.6). A takedown turns it off, and the middle path keeps working. The project never ships Blizzard text in the repo or in a release (5.10).
- The Battle.net login binds us to the Blizzard Developer API terms. They limit what we keep, how long, and any commercial use (open question 4). Blizzard can end our client id, and then nobody can log in. Section 5.4 says what comes then.
- The Blizzard add-on policy asks for free add-ons, with no ads in the game. The addon never asks for money, and the website shows no ads.

**Before the launch:** the terms, the privacy policy, the takedown page, the DMCA agent, a contact address, and a decision on the full text.

## 17. UI copy

The copy follows the UI copy rules of `CLAUDE.md`. Players never see "online call", "sync", "pool", "hash", "template", "contributor", "token", "device code", "OAuth", "refresh", "independent report", "reputation", "bridge", or "model".

**In the game.** The Sharing section is a part of the Timeways options in the game (the AddOns tab of the game options).

| Where | Copy |
|---|---|
| Section title | Sharing |
| Section line | Share with other Timeways players. Everything is off until you turn it on. |
| Switch | Share my quests |
| Switch | Get quests from other players |
| Switch | Share game text |
| Switch hint | Quest text, gossip, and books you read. Your character's name is removed. |
| Switch | Use the shared game text |
| Switch hint | Lore answers can use text other players found, for places you've reached. |
| Switch | Share world facts |
| Switch hint | Which NPCs stand where, and their titles. |
| Switch | Rate AI lines |
| Switch | Help review quests |
| Switch hint | Your AI checks a few quests a day for other players. |
| Name box | Your name on shared quests |
| Name box hint | Anonymous |
| Button, not linked | Link this computer |
| While linking | Your browser opened. Log in with Battle.net, then enter KXM-492. |
| Linked | Linked to Ada#1234 |
| Button, linked | Unlink this computer |
| Link done | This computer is linked. |
| Code expired | That code expired. Try again. |
| Link canceled on the website | Linking canceled. |
| Link ended | This computer isn't linked anymore. Link it again in the Sharing options. |
| A sending switch, not linked | Link this computer to share. |
| Switch | Add my characters from the game |
| Switch hint | Shows them on your page. Only characters Battle.net confirms show on your quests. |
| Publish question | Show my name as |
| Publish choices | Anonymous, Ada's Tales, Ada of Stormrage |
| Button | Delete my online data |
| First switch popup | This sends what you share to the Timeways website. It never sends your character's name or anything you type. |
| Popup buttons | Okay, Cancel |
| Delete popup | Delete everything you shared? Your published quests come down too. |
| Delete popup buttons | Delete, Cancel |
| After delete | Your online data is deleted. |
| Quest form button | Publish |
| Publish panel title | Publish Quest |
| Publish question | Who gives this quest? |
| Publish choice | Any innkeeper in Goldshire |
| Publish question | Who can get it? |
| Publish choices | Everyone, Ada's Guild |
| Publish while checking | Checking... |
| A step that can't go | Remove "Find Ada" first. Shared quests can't name players. |
| Local check fail | Can't publish this. It looks like it names a real person. Edit it and try again. |
| Quests page heading | Shared Quests |
| Quest states | In review, Published, Not published |
| Quests page heading | Added Quests |
| Added, waiting | Waiting for an innkeeper in Goldshire. |
| `/quest add` done | Quest added. Look for it from an innkeeper in Goldshire. |
| `/quest add` bad code | No quest with that code. Check the code and try again. |
| Offer line | Innkeeper Farley has a quest for you. Written by Ada. |
| After a library quest | Did you enjoy this quest? |
| Rating buttons | Yes, No |
| Thumbs tooltips | Good line, Bad line |
| Report button | Report |
| Report reasons | Offensive, Spam, Names a real person, Breaks the lore, Doesn't work, Other |
| After a report | Thanks. We'll take a look. |
| No connection | Can't reach Timeways online. Try again later. |
| Over a limit | You've shared a lot today. Try again tomorrow. |
| Desktop switch off | Sharing is turned off on your computer. |
| Old version | Update Timeways to keep sharing. |

**On the website:**

| Where | Copy |
|---|---|
| Home heading | Quests by players |
| Home line | Quests written by Timeways players. Find one you like, and play it in WoW. |
| Get button | Play in WoW |
| Get panel | In WoW, type /quest add 7KQ4MX |
| Copy button | Copy |
| Stats | 412 plays · 87% liked it |
| Empty list | No quests here yet. |
| Sorts | Most liked, Most played, Newest |
| My page heading | My Quests |
| Delete button | Delete my data |
| Login button | Log in with Battle.net |
| Link page heading | Link Your Computer |
| Link page line | Enter the code from the game. |
| Link page asker | A Windows computer asked 1 minute ago. |
| Link buttons | Link this computer, Cancel |
| Link done | This computer is linked. You can close this page. |
| Wrong code | That code didn't work. Check it in the game and try again. |
| Computers heading | Your Computers |
| Computer row | Windows · linked Oct 3 · last used today |
| Unlink button | Unlink |
| Unlink popup | Unlink this computer? It stops sharing until you link it again. |
| Characters heading | Your Characters |
| Not verified tag | Not verified |
| Refresh button | Refresh my characters |
| Byline on a quest | A quest by Ada of Stormrage |
| Group heading | Ada's Guild |
| Invite line | Share this code in your guild: /quest group 4XPL9R |

**Bad and good:**

| Bad | Good | Why |
|---|---|---|
| Upload seen-text batch | Share game text | Name the goal, not the mechanism. |
| Your contribution reached quorum (3/3). | (nothing) | Internal words. The player needs no news here. |
| Community review pending (2/5 votes) | In review | A status is a few words. |
| Template failed validation: step 3 kind `find_player` not allowed | Remove "Find Ada" first. Shared quests can't name players. | Say what is wrong and what to do. |
| Rate this model output | Good line | "Model" is an internal word. |
| Moderation failed: `banned_word` | Can't publish this. It uses a word we don't allow. | No codes. Keep the player's text. |
| The bridge could not reach api.timeways | Can't reach Timeways online. Try again later. | "Bridge" is internal. |
| HTTP 429 Too Many Requests | You've shared a lot today. Try again tomorrow. | No numbers, no blame. |
| This device is not linked | (only the Link this computer button) | The button already says it. |
| Enter your RFC 8628 user code to authorize the device | Enter the code from the game. | No internal words. |
| OAuth refresh failed: invalid_grant | This computer isn't linked anymore. Link it again in the Sharing options. | Say what happened and what to do. |
| Character unverified (Profile API) | Not verified | A short tag. "Profile API" is internal. |
| Thy name shall be inscribed in the annals | Show my name as | Fake flourish. |
| Send your tome to the Great Library of the Bronze Flight! | Share my quests | Fake flourish. The UI never performs a voice. |
| Reputation: Trusted | (nothing) | The level is a mechanism. |

The Report reasons, the switch names, and the website lines go through the CLAUDE.md rules again when a person writes the final pages.

## 18. Build order

Each step is one commit with its tests, and each one ships. The first version is steps 1 to 6.

**First version: the quest library, by code, with a person who reviews.**

1. **Decide and check.** The domain and the host. Register the client on develop.battle.net. Check with a real account whether the Profile API lists WoW: Forever characters, and read the API terms and rate limits (open questions 3 to 5). Write the terms, the privacy policy, and the takedown page.
2. **`online-rules`.** The template parse and check, the words check, the hash, and `independent_count`, with all their tests. Nothing calls them yet.
3. **The server skeleton.** Log in with Battle.net, the characters of the Profile API, the device flow, links, Unlink, `GET /v1/me`, `DELETE /v1/me`, the rate limits, the migrations, the API tests against PostgreSQL and a fake Blizzard OAuth server in CI. One host, no files yet.
4. **The relay.** Protocol 3, the HTTPS route, the device code link, the token file with refresh, the op table, the local log (6.7). Wait for the relay session.
5. **Publish and get by code.** `quest_publish_asked`, the checks of 7.3, `publish_quest`, the queue page for a maintainer, `get_quest`, `/quest add`, the fit and the offer of 7.5. A quest goes public only after a maintainer reads it.
6. **The website.** The quest list, the quest page, My page, Delete my data, the legal pages. Likes, plays, and reports.

**Later, each one on its own:**

7. **Votes and lore reports**, and the candidates and prompt stats pages. Low risk, and it helps the prompts at once.
8. **World facts.** It helps npc-knowledge.md. The `world.jsonl` file and the first `sync` manifest.
9. **The feed.** `quests.jsonl`, and offers from the feed (one in three).
10. **Groups.** Guilds and roleplay groups.
11. **The text library, middle path.** Hashes and where texts are. The Knowledge page count, the overlap check, the narrator moment.
12. **Community review**, when 20 Trusted reviewers exist.
13. **Samples from players.** `samples.jsonl`.
14. **The full text library**, after the legal decision of section 16.
15. **Characters from the game**, the fallback of 5.3, when the Profile API does not cover Forever.
16. **Voice lines from your own reading** (section 22, sources 1, 2, and 4). It needs nothing online, so it can come before step 1, after the kinds of npc-knowledge.md and the conversation of talk-window.md. `voice_lines.rs`, the block, the copy check, the reads, and the budget tests.
17. **Pooled voice lines** (section 22, source 3), after step 14. The `pooled:<hash>` address of the reads.
18. **`GAMEPLAY.md`.** The text of section 21. This plan marks the steps as done.

## 19. Tests

Each test is a sentence, and reads arrange, act, assert. No test needs the game, a model, or the real API. The server tests need a PostgreSQL in CI, as a service of the job.

### 19.1 `crates/online-rules/tests/`

`template.rs`:

- `a_template_with_a_find_player_step_is_refused`
- `a_template_with_an_other_step_is_refused`
- `a_title_over_sixty_bytes_is_refused`
- `a_text_with_a_dollar_code_other_than_n_is_refused`
- `a_text_with_a_web_address_is_refused`
- `a_text_with_a_name_after_the_cutoff_is_refused`
- `a_kill_count_over_ten_is_refused`
- `levels_out_of_order_are_refused`
- `a_slap_step_needs_a_comic_quest`
- `a_template_of_an_unknown_format_is_refused`

`hash.rs`:

- `the_zone_is_not_part_of_the_text_hash`
- `two_texts_that_differ_by_one_byte_have_two_hashes`
- `the_hash_of_a_known_text_never_changes` (a fixed vector)

`independence.rs`:

- `two_reports_from_one_contributor_count_once`
- `reports_from_one_network_count_once`
- `a_new_contributor_counts_only_once_known`
- `a_limited_contributor_never_counts`
- `three_independent_reports_pool_a_text`

### 19.2 `crates/story/tests/`

`library.rs`:

- `a_library_quest_passes_the_side_quest_check`
- `a_library_quest_with_a_place_you_never_visited_waits`
- `a_library_quest_for_another_faction_never_comes`
- `a_library_quest_outside_your_levels_never_comes`
- `an_added_quest_comes_before_a_feed_quest`
- `a_feed_quest_comes_at_most_once_in_three_offers`
- `a_library_quest_you_accepted_never_comes_again`
- `a_library_quest_needs_no_model`
- `a_library_quest_has_shared_proof`
- `the_offer_names_the_author`

`publish.rs`:

- `a_published_quest_holds_no_known_player_name`
- `a_published_quest_never_carries_the_reward`
- `a_quest_with_a_target_the_author_never_saw_is_refused`
- `a_failed_local_check_sends_nothing`
- `with_no_model_the_quest_goes_with_no_local_check`

`sharing.rs`:

- `nothing_is_sent_while_every_switch_is_off`
- `a_rumor_is_never_shared`
- `a_text_with_a_known_player_name_stays_home`
- `a_text_goes_once_for_each_contributor`
- `the_middle_path_sends_no_words`
- `a_world_fact_goes_again_only_when_it_changes`

`pooled.rs`:

- `a_pooled_quest_text_needs_the_quest`
- `a_pooled_gossip_text_needs_the_npc`
- `a_pooled_book_needs_its_zone`
- `a_pooled_text_never_joins_the_knowledge_page`
- `your_own_text_comes_before_a_pooled_text`
- `a_bad_line_in_a_download_is_skipped`
- `zones_toml_wins_over_a_pooled_fact`

`votes.rs`:

- `a_vote_carries_the_text_before_the_name_swap`
- `a_vote_on_a_talk_answer_that_quotes_typed_words_is_dropped`
- `a_vote_never_holds_the_name_of_the_character`
- `a_vote_names_a_row_and_never_takes_text_from_the_addon`
- `a_lore_report_never_holds_the_question`

`review.rs`:

- `a_review_job_runs_one_call_of_the_story_pool`
- `a_review_job_knows_nothing_of_the_author`

`online_call.rs`:

- `an_op_that_sends_with_no_link_gets_unlinked_and_sends_nothing`
- `a_download_works_with_no_link`
- `a_link_that_ends_shows_one_notice`
- `a_character_is_reported_only_with_its_switch_on`

- `a_sync_runs_at_most_once_an_hour_at_entry`
- `an_online_failure_shows_one_notice`
- `an_old_story_program_stops_its_online_calls`

### 19.3 Property tests (`crates/story/tests/properties.rs` and `crates/online-rules/tests/properties.rs`)

- For any set of reports, `independent_count` never exceeds the number of contributors or of networks. Make one contributor and one network likely.
- For any user code that the server makes, the code has 3 letters of the alphabet of 5.2, a dash, and 3 digits from 2 to 9, and the code box reads it back in any case, with or without the dash.
- For any set of reports, removing every report of one contributor lowers the count by at most one.
- For any order of the same reports, the count is the same.
- For any play, no `online_call` body holds the name of the character, a realm, or a name of the alias table.
- For any play with every switch off, the story program sends no `online_call`.
- For any template that `online-rules` passes, the story program check gives the same verdict for the shape and the words.

### 19.4 Server tests (`crates/online-server/tests/`)

- `a_call_with_no_token_is_refused`
- `a_token_is_stored_only_as_a_hash`
- `a_login_with_a_wrong_state_is_refused`
- `a_login_keeps_no_blizzard_token`
- `a_login_keeps_only_name_realm_region_and_faction_of_a_character`
- `a_failed_profile_call_keeps_the_old_characters`
- `a_character_that_the_new_list_lacks_loses_its_proof`
- `a_verified_character_moves_to_the_account_that_lists_it_last`
- `a_reported_character_is_not_verified`
- `a_byline_with_a_not_verified_character_is_refused`
- `a_device_code_lives_ten_minutes`
- `a_poll_before_the_interval_gets_slow_down`
- `a_poll_before_approval_gets_authorization_pending`
- `a_denied_code_gets_access_denied`
- `an_approved_code_gives_tokens_once`
- `a_sixth_wrong_code_in_ten_minutes_is_refused`
- `a_refresh_gives_a_new_pair_and_ends_the_old_refresh_token`
- `an_old_refresh_token_used_again_revokes_the_link`
- `an_access_token_after_one_hour_gets_401`
- `unlink_on_the_website_ends_both_tokens_at_once`
- `revoke_with_an_unknown_token_answers_200`
- `deleting_my_data_revokes_every_link`
- `a_public_quest_needs_no_token`
- `the_public_manifest_names_no_group_file`
- `the_first_three_quests_of_a_contributor_go_to_review`
- `three_reports_from_one_network_hide_nothing`
- `three_independent_reports_hide_a_quest`
- `a_rating_needs_a_finished_play`
- `deleting_my_data_removes_my_reports_and_votes`
- `deleting_my_data_drops_a_text_below_three_reports_from_the_next_build`
- `a_group_quest_goes_only_to_members`
- `a_call_over_the_limit_gets_429_with_the_wait`
- `the_full_text_switch_off_builds_files_with_no_words`
- `five_reviewers_never_include_the_author_or_a_group_member`
- `two_failed_reviews_send_a_quest_to_the_queue`
- `the_export_holds_every_row_of_the_contributor`

### 19.5 Addon tests (`crates/addon-tests/tests/`)

- `sharing.rs`: every switch starts off; the first switch shows the popup; Delete asks first; a sending switch with no link shows "Link this computer to share."; a linked computer shows its BattleTag; Unlink asks first.
- `publish.rs`: Publish needs a saved quest; a step with a player shows the line of 17; the panel keeps the quest after a fail.
- `thumbs.rs`: a thumb sends the row, never the text.

### 19.6 Fuzz

- `fuzz/fuzz_targets/online_files.rs` (new): random lines of `quests.jsonl`, `texts.jsonl`, `world.jsonl`, and `samples.jsonl`. The read never fails and never panics. A line that passes passes the checks of its kind. Seeds in `fuzz/seeds/online_files/`.
- `fuzz/fuzz_targets/online_answer.rs` (new): random `online_answered` bodies for each op.
- `fuzz/fuzz_targets/template.rs` (new): random templates through `online-rules`. A template that passes holds no `|`, no control character, and no `$` code but `$N`.
- `fuzz/fuzz_targets/input.rs`: the new addon lines of 6.6.
- The server: a fuzz target for each request body, in `crates/online-server/fuzz/`.

## 20. Open questions

1. **Decided: Battle.net login** (the user, 2026-10-03). The install key with no login is rejected for now (5.4).
2. **The full text library.** Build it after the middle path, or stay on the middle path? The user accepts the risk. A lawyer's view first?
3. **The Profile API and WoW: Forever.** Does a namespace of the Profile API list Forever characters? A test with a real account decides it before build step 3. With none, no character is verified: bylines are display names only, and Trusted needs another rule.
4. **The Blizzard API terms.** What they allow for data use: how long we keep a character list, and whether we can show a character name in public. What they say about commercial use, for the later hosted model. A reading of the current terms decides, before the launch.
5. **The Blizzard API rate limits.** The published limit was about 36,000 calls an hour and 100 a second for each client. Our use is 2 calls for each login, far below it. Check the current numbers.
6. **The BattleTag in the game.** The game shows "Linked to Ada#1234" against a guessed code (5.2). Is that fine on a screen that a streamer shows? An option: show only the first letters.
7. **China.** Battle.net in China has its own OAuth service. Leave it out of the first version?
8. **The domain and the name** of the website.
9. **The kill count of a template.** A player quest allows 1 to 250 kills. A template follows the side quest limit of 10. Raise it for templates?
10. **Carry items.** A template `carry` step follows the fixed list of quest-variety.md 4.9. Allow any item of the game for a template?
11. **Who reviews in the first months.** Every quest goes to a maintainer until 20 Trusted reviewers exist. Is one maintainer enough, or does the user want a few trusted players as moderators?
12. **Other locales.** A quest goes only to its locale. Allow a translation by its author later?
13. **Signed files.** Sign the download files with an offline key, so a hacked server or CDN cannot push files? The story program already reads them as hostile.
14. **The real-player check of the local AI.** The model cannot know which names are players. Is the name strip of the addon plus reports enough?
15. **A website editor.** Write a quest on the website, with the names of the world facts? It needs the world facts first, and a check with no world of the author.

## 21. GAMEPLAY.md text

In 2, rule 5, add after the first sentence:

> Sharing with other players through Timeways online (4.9) is off until you turn it on, one choice at a time.

Add a new section after 4.8:

> ### 4.9 Timeways online
>
> Timeways online is a website and a database for players who want to share. Plan: `docs/plans/online.md`. Not built.
>
> - **Opt-in.** Each choice is off until you turn it on. Each one has its own switch in the Sharing section of the Timeways options. "Delete my online data" deletes everything you shared.
> - **What never leaves your computer:** the name of your character and your realm (unless you add your characters from the game), the name of another player, the words that you type, your world, your chronicle, and the prompts.
> - **Who makes the calls.** The desktop app makes every call to our servers, as it makes the model calls. The story program has no network. The addon has none either.
> - **Accounts.** The website logs in with Battle.net. The desktop app links to the account with a short code: it opens the website, you log in with Battle.net, and you enter the code. No password goes into the game or the terminal. Unlink ends the link at once. Downloads need no account. Every upload needs a linked computer.
> - **Characters.** The website reads your WoW characters from Battle.net. Only these verified characters can show by name in public: "A quest by Ada of Stormrage". The game can also add your characters, but they stay "Not verified" and never show in public.
>
> **The quest library.**
>
> - You publish a quest that you wrote (4.7). It loses the names of players, its rewards, and any step that names a player or that the game cannot see. You pick an NPC as its giver, or a kind of NPC in a zone, and a level band.
> - Your own AI checks it first. The server checks it again. A few other players' AIs read it before it goes public, and a person reads the rest.
> - Other players get it with a code, from the feed, or through a guild or a roleplay group.
> - A library quest comes as a side quest (3.4) from a fitting NPC. It passes the check of a side quest against your own world, so it never names a place that you have not seen. It needs no model.
> - Its rewards are story only. Its proof is Shared (5.14).
> - Players like it, play it, and report it.
>
> **The game text library.**
>
> - You share the quest text, gossip, and books that you read (5.10), with `$N` for your name. A rumor never goes.
> - A text joins the library only after 3 independent players sent the same text.
> - A text of the library passes your spoiler limit only for a quest that you hold or did, an NPC that you met, or a zone that you visited. It is never your own knowledge: the Knowledge page leaves it out, and an answer says "players report".
> - Blizzard owns this text. So the library can also hold only which texts exist and where, with no words.
>
> **World facts.** You share the title, the faction, and the place of the NPCs that you see, and the continent of each zone. A fact joins the library after 3 independent reports. The hand-written `zones.toml` wins.
>
> **Votes.** You rate a narrator line, a quest offer, or a talk answer with a thumb. The vote carries the line and the prompt it came from, never a name or your words. A person reads the best lines and makes golden samples of them. You can also report a problem in a `/lore` answer. The report never carries your question.
>
> **Independent reports.** Two reports count apart only from two Battle.net accounts on two networks, and only from accounts with a week of history. One account with many computers counts once.

In 3.4, after "**The offer**", add:

> - **A library quest** (4.9) comes first when one fits the NPC: an added quest always, a quest of the feed in at most one offer of three. It passes the same check against your world. With none, the model writes the quest.

In 3.5, add:

> - **Voice examples.** The talk prompt gets 2 or 3 real lines in the voice of the NPC, in place of the golden samples: first its own gossip and quest text that you read, then lines that you read of NPCs of the same kind, then lines of the shared game text (4.9) that pass your spoiler limit. A line is the start of a text of at most 1000 characters, cut at a sentence end within 200 characters, with "you" in place of `$N`. A book or a rumor is never a voice line. A line of another NPC never names a zone that this NPC does not know. With fewer than 2 lines, the golden samples stay. The NPC copies the manner, never the facts: an answer that repeats 8 words in a row of a voice line is refused.

In 5.10, after "The lore of each player", add:

> **The shared game text** (4.9), when you turn it on: a third source after your own text and before the pack. It passes the spoiler limit as 4.9 says, and its source reads "players report".

In 5.12, in "**The story program**", add:

> - Calls to Timeways online (4.9) go through the bridge with `online_call`, as model calls do. The bridge adds the token and calls one HTTPS host. The story program reads the downloaded files from a folder that it cannot write.

In 5.14, in the table of roots, change the row Shared:

> | Shared | another player said it: a player story, a library quest, or a shared game text, also as a voice line of a talk |

In 5.14, in the table of reads, add to the row Talk:

> and the `learned` rows of its voice lines, and the pooled texts of its voice lines (3.5)

In 9, add:

> 10. **Battle.net characters for WoW: Forever** (4.9). Does the Profile API list them? A test with a real account decides.
> 11. **The full text library** (4.9). Share the words of game text, or only where texts are?

## 22. Voice examples from real NPC lines

The user decided this on 2026-10-03. The idea comes from the Stardew Valley mod ValleyTalk: a small model copies a real voice better than it follows a description of one. So the talk prompt gets 2 or 3 real lines in the voice of the NPC, from game text. The golden samples (3.2.1) stay as the fallback.

The first two sources need nothing online. The pool only adds the third source. So this section ships in two parts (18).

### 22.1 Where the lines come from

The sources, in order. The pick stops at 3 lines.

| Order | Source | Needs |
|---|---|---|
| 1 | **This NPC's own lines that you read**: the seen texts (5.10) of kind `gossip` or `quest` whose `npc` is this NPC | nothing |
| 2 | **Your lines of the same kind of NPC**: your seen texts of other NPCs of the kind of this NPC (npc-knowledge.md 5), by their title | the kinds of npc-knowledge.md |
| 3 | **Pooled lines** (8.4) of this NPC, then of the same kind | "Use the shared game text", with the full text library |
| 4 | **The golden samples** of `npc_replies.txt`, as today | nothing |

- A book is never a voice line. A book is a writer, not the voice of the NPC.
- A rumor is never a voice line. A rumor is model text (3.1.1), and a model must never learn its voice from its own words. Only rows of `TextSeen` count.
- Within a source, gossip comes before quest text, because gossip is spoken. Then the same zone as the NPC first, then the newest read first. Pooled lines order by their number of independent reports, then by hash, so every player gets the same order.
- **A local is wide.** Every NPC with no kind word is a local (npc-knowledge.md 5.1). So for a local, source 2 and the kind part of source 3 take only NPCs of the same zone.
- **The kind of another NPC** comes from its title in your world (`title_of`). With no title there, the pooled world fact `npc_title` (section 9) gives it, when "Use the shared game text" or world facts are on. With neither, the NPC is a local.

### 22.2 What each mode of the pool gives

| Mode | Source 1 and 2 | Source 3 |
|---|---|---|
| Pool off | your own reading | none |
| Middle path (8.5): hashes and places, no words | your own reading | **none.** The pool has no words. It still gives the kind of more NPCs through pooled titles, so source 2 finds more of your own lines. |
| Full text (8.4) | your own reading | pooled lines that pass your spoiler limit |

### 22.3 The rules of a line

1. **Skip a long text.** A seen text over 1000 characters is skipped. A long text is exposition, not a voice.
2. **Cut to the limit.** A line is the text from its start up to the last sentence end (`.`, `!`, or `?`) within 200 characters. With no sentence end there, the cut goes at the last whole word, with "...".
3. **`$N` becomes "you".** The line speaks to the player. A line with any other `$` code is skipped: the game fills those before the addon reads them, so another code means a strange text.
4. **No spoiler.** A line of your own reading passes: you read it. A pooled line passes the spoiler limit of 8.4: a quest text only for a quest that you hold or turned in, a gossip text only for an NPC that you met.
5. **No knowledge leak.** A line of another NPC (sources 2 and 3) that names a zone of `zones.toml` that this NPC does not know (npc-knowledge.md 6.3) is skipped. So a voice line never teaches the NPC a place. The NPC's own lines need no such check.
6. **The cutoff.** A line with a name after the cutoff (5.9) is skipped. Game text holds none, but a pooled text came from other computers.
7. **Once.** The same text gives one line, also when it comes from two sources.
8. **2 or nothing.** With 2 or 3 lines, the voice block takes the place of the golden samples. With 0 or 1 line, the prompt keeps the golden samples as today, and the one line is dropped. One example is too few: the model copies it word for word.
9. **The same lines for a whole conversation.** The pick takes the number of the conversation (talk-window.md 4.1) as its rotation: the first line of each source rotates by it. So the lines stay the same over the turns of one talk, and change from talk to talk.

### 22.4 The function

A new module `crates/story/src/voice_lines.rs`, with pure functions only:

```rust
pub const MIN_VOICE_LINES: usize = 2;
pub const MAX_VOICE_LINES: usize = 3;
/// A text over this is exposition, not a voice.
pub const MAX_SOURCE_CHARS: usize = 1000;
pub const MAX_LINE_CHARS: usize = 200;

/// Where a voice line came from. Its order is the order of the pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VoiceSource { OwnNpc, OwnKind, PoolNpc, PoolKind }

/// The row that a line came from, for the reads (22.6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VoiceRow { Learned(RowId), Pooled(TextHash) }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceLine { pub text: String, pub source: VoiceSource, pub row: VoiceRow }

/// What the pick needs to know about the NPC of the talk.
pub struct Speaker<'a> { pub npc: &'a str, pub zone: Option<&'a str>, pub knower: &'a Knower }

/// 2 or 3 lines in the voice of the speaker, or none: then the prompt keeps the golden samples.
pub fn voice_lines(speaker: &Speaker<'_>, world: &Character, seen: &[SeenRow], pool: &Pooled, rotation: u32) -> Vec<VoiceLine>

/// The cut form of a text (22.3, rules 1 to 3), or none when the text gives no line.
pub fn voice_line_of(text: &str) -> Option<String>
```

- `voice_lines` reads the four sources in order, keeps each line that passes 22.3, and stops at `MAX_VOICE_LINES`. Under `MIN_VOICE_LINES` it gives none.
- `voice_line_of` is the one place that cuts a text. The property test (22.9) uses it too.
- `Pooled` is the text library of `pooled.rs` (8.4). With the pool off, or on the middle path, it holds no words, and sources 3 give nothing.
- `story.rs` calls `voice_lines` once at the first turn of a conversation, and keeps the result with the conversation in memory (talk-window.md 4.1). So each later turn uses the same lines and the same reads.

### 22.5 The prompt block

The block takes the place of the golden samples in `talk::prompt`, in one fence, because a pooled line is hostile text (`house::fenced`):

```
How you talk. These are words you said before, or words of people like you. Copy the manner, never the facts, and never a whole line:
<<<
- Welcome to the inn, you. Make yourself at home, and mind the stairs.
- Hot food, a warm bed, and no questions. That's the rule of my house.
>>>
```

The prompt words are internal, not UI copy. The player never sees the block.

**The copy check.** Today the check of a talk answer refuses a copy of a golden sample (`samples::every_sample`). It also takes the voice lines of the prompt: an answer that holds 8 or more words in a row of a voice line is refused, as a copy of a sample is. A talk has no retry (3.5), so the NPC "says nothing".

**The order of the talk prompt**, with every plan:

1. The persona, its title and kind sentence (npc-knowledge.md 7.1), and the house rules.
2. What the NPC knows: the lore that it knows, the rumor block (npc-knowledge.md 7.2), and on the first turn only the memory block (npc-memory.md 5) and the talk of the town (standing.md 9.3).
3. **The voice block** (this section), or the golden samples.
4. The talk so far (talk-window.md 4.3), from the second turn.
5. The words of this turn.
6. The author's note, the options rule, and the JSON shape.

The voice block goes right before the talk so far. So the last examples of manner that the model reads before the dialogue are real lines.

**The budget.** A prompt has at most 1773 tokens (`tokens::Call::Talk`, talk-window.md 4.3).

| Part | Tokens, at most |
|---|---|
| The voice block: 3 lines of 200 characters, the marks, and the heading of about 150 characters | 190 |
| The golden samples that it replaces | counted as 0, so the table is on the safe side |

| Turn | Before this plan | With the voice block | Room left |
|---|---|---|---|
| First turn: talk-window.md 882, the talk of the town 230 (standing.md 9.3), npc-knowledge.md 100 | 1212 | 1402 | 371 |
| A later turn: talk-window.md 1217, npc-knowledge.md 100 | 1317 | 1507 | 266 |

Both fit. `tests/voice.rs` gets the longest first turn and the longest later turn with 3 lines of 200 characters, and the budget test covers both.

### 22.6 Reads and proof (5.14)

- A talk call **reads** the `learned` row of each voice line of its own reading. These rows have Game proof, so the proof of a talk does not change.
- A pooled voice line has no row in the world. Its read is a new address of links.md 5: `pooled:<hash>`, with the root Shared. So a talk that used a pooled line shows Shared: the words of another player's game shaped it. That is true, and it is the safe side.
- The prompt of the call keeps the lines, as it keeps all of its text (5.14, "Prompts").
- `story/reads.rs`, row Talk: add "and the rows behind its voice lines".

### 22.7 Edge cases

| Case | Rule |
|---|---|
| An NPC that you never read, of a kind with no lines | The golden samples, as today |
| You read only one gossip of this NPC | One line from source 1, and the pick goes on to source 2 |
| A quest text that holds the objectives at its end | The cut at 200 characters keeps the first sentences, which are the giver's words |
| A text that is one long sentence | The cut at the last whole word, with "..." |
| A text of exactly 1000 characters | Kept. 1001 is skipped. |
| A line that names the player's class or race ("Greetings, night elf") | Kept. The game filled it for this player. |
| A pooled line of a quest that you never took | Skipped (22.3, rule 4) |
| The full text library ends after a takedown (8.6) | The next file has no words. Source 3 gives nothing, and the talk works as on the middle path. |
| Two NPCs with one name | Source 1 takes the seen texts of the name, as the seen text does today (5.10) |
| An NPC whose title changed (npc-knowledge.md 4.1) | The newest title sets the kind |
| A seen text of another locale in the pool | Never: the pool file holds only the locale of the player (8.1) |
| The model answers with a voice line word for word | The copy check refuses it (22.5) |

### 22.8 Tests

`crates/story/tests/voice_lines.rs` (new):

- `the_npcs_own_lines_come_first`
- `lines_of_the_same_kind_come_after_the_npcs_own`
- `a_local_takes_lines_of_the_same_kind_only_from_its_zone`
- `pooled_lines_come_after_your_own_reading`
- `pooled_lines_of_the_npc_come_before_pooled_lines_of_its_kind`
- `gossip_comes_before_quest_text`
- `a_book_is_never_a_voice_line`
- `a_rumor_is_never_a_voice_line`
- `a_text_over_a_thousand_characters_is_skipped`
- `a_text_of_a_thousand_characters_is_kept`
- `a_line_is_cut_at_the_last_sentence_end_within_two_hundred_characters`
- `a_line_with_no_sentence_end_is_cut_at_a_whole_word`
- `dollar_n_becomes_you`
- `a_line_with_another_dollar_code_is_skipped`
- `a_pooled_quest_line_needs_the_quest`
- `a_pooled_gossip_line_needs_the_npc`
- `a_line_of_another_npc_that_names_a_zone_this_npc_does_not_know_is_skipped`
- `a_line_with_a_name_after_the_cutoff_is_skipped`
- `the_same_text_gives_one_line`
- `one_line_gives_no_voice_block`
- `the_middle_path_gives_no_pooled_line`
- `a_pooled_title_sets_the_kind_of_an_npc_that_you_saw_with_no_title`
- `the_lines_stay_the_same_for_each_turn_of_a_conversation`
- `the_lines_change_with_the_conversation_number`

`crates/story/tests/talk.rs`:

- `the_voice_block_takes_the_place_of_the_golden_samples`
- `the_voice_block_comes_before_the_talk_so_far`
- `an_answer_that_copies_eight_words_of_a_voice_line_is_refused`
- `a_talk_reads_the_learned_rows_of_its_voice_lines`
- `a_talk_with_a_pooled_voice_line_has_shared_proof`

`crates/story/tests/voice.rs`:

- `the_longest_first_turn_with_voice_lines_fits_the_talk_budget`
- `the_longest_later_turn_with_voice_lines_fits_the_talk_budget`

### 22.9 Property test (`crates/story/tests/properties.rs`)

`a_voice_line_in_a_prompt_is_always_read_or_counted_text`: for any play and any pool file, each line of the voice block of each talk prompt is `voice_line_of` of one of two texts:

- the text of a `TextSeen` row of this character, of kind `gossip` or `quest`, or
- a text of the pool file that passes the spoiler limit of 8.4 for this character. The pool file holds only texts with 3 independent reports (8.3), so a pooled line is always a counted text.

The test also checks that no voice line comes from a rumor row or a book. It makes the edges likely: texts of 999, 1000, and 1001 characters, a sentence end at character 200 and at 201, a text with `$N` at its start and at its end, a pooled quest text of a quest held, done, and never taken, and a pick with exactly 1 and exactly 2 lines.

### 22.10 Fuzz

- `fuzz/fuzz_targets/seen.rs`: `voice_line_of` on every random text. A line it gives is at most 200 characters, holds no `$`, and has no control character.
- `fuzz/fuzz_targets/online_files.rs`: `voice_lines` with the random pool file and a small world. It never panics.
