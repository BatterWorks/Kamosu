# Sending a recipe into an app from the iPhone share sheet: prior art

Researched 2026-10-23 for Kamosu (self-hosted, PWA, no native iOS app, imports handed to an agent).
The question was how other recipe apps let someone on an iPhone send a recipe in with one tap from
Safari, Instagram, TikTok, Photos or copied text, and what does iOS allow a web app to do?

Claims are cited inline. "Decoded" means I downloaded the shared shortcut from Apple's iCloud record
API (`https://www.icloud.com/shortcuts/api/records/<id>`, the `fields.shortcut` asset, a binary
plist) and read its actions. What I could not verify is marked **unverified**.

What Kamosu decided from this is [ADR 0054](../adr/0054-a-phone-shares-into-kamosu-through-one-operation-and-an-add-only-key.md).
One finding below was corrected in the design session, and is marked where it stands: Instagram's
Share button reaches the system Share menu through "Share to…", so a Shortcut is listed there.

## The short answer

1. **A web app on iOS cannot be a share target.** Safari and Home Screen web apps do not support the
   Web Share Target API (`share_target` in the manifest), as of October 2026. Mealie, Tandoor and
   RecipeSage all declare `share_target`, and it does nothing on an iPhone.
2. **So the self-hosted apps without a native iOS app that offer an iPhone way in at all use an Apple
   Shortcut, and almost every one of those Shortcuts was written by a user rather than the project.** The projects document it in
   "community guide" pages, or not at all, and the Shortcuts break whenever an endpoint moves.
3. **Two Shortcut shapes exist**:
   - **Token shape.** The Shortcut holds an API key and calls the HTTP API itself with "Get Contents
     of URL". Used by Mealie, Norish, Tandoor, Nextcloud Cookbook, and Recipe Filer's "power-user"
     variant.
   - **Browser shape.** The Shortcut opens the app's own page in Safari with the shared link in the
     query string (`…/share-import?url=…`), and the page uses the browser's existing sign-in.
     There is no token and no per-user setup. Recipe Filer recommends this one, and RecipeSage's
     bookmarklet works the same way.
4. **Commercial apps all ship a native share extension.** Paprika, Mela, Crouton, Pestle, ReciMe,
   Honeydew and Samsung Food are listed under Share. The AI-led ones (Pestle, ReciMe, Honeydew)
   accept an Instagram or TikTok video link and read the caption, the audio, or a linked page.
   KitchenOwl is the only self-hosted app here with a native iOS share extension.
5. **Nobody found offers a "tap to install, token already filled in" link.** iOS makes it hard: a
   shared shortcut has to be signed by Apple, which happens only from the Shortcuts app or
   `shortcuts sign` on a Mac.

## App by app

### Mealie (self-hosted). Community Shortcut, token shape, the most complete one found

- **Path:** Apple Shortcut linked from the docs page "iOS Shortcut", which says it "was submitted by
  a community member" ([docs](https://mealie.io/documentation/community-guide/ios-shortcut/),
  [source md](https://github.com/mealie-recipes/mealie/blob/mealie-next/docs/docs/documentation/community-guide/ios-shortcut.md)).
  A maintainer: "We don't have an official shortcut… added by a community member"
  ([#4407](https://github.com/mealie-recipes/mealie/issues/4407)). Mealie's manifest declares a
  `share_target` (GET `/r/create/url`, params `url` and `text`, the latter "used by apps that share
  URLs as plain text") ([manifest.py](https://github.com/mealie-recipes/mealie/blob/mealie-next/mealie/routes/spa/manifest.py)),
  which works on Android only (see Platform).
- **Distribution:** an iCloud link,
  [icloud.com/shortcuts/52834724050b42aebe0f2efd8d067360](https://www.icloud.com/shortcuts/52834724050b42aebe0f2efd8d067360)
  ("Add to Mealie", last modified 2026-04-30 per its record).
- **Token:** two **import questions** at install: "Enter your Mealie API Key." (default
  `MEALIE_API_KEY`) and "Enter the address used to access Mealie" (default `MEALIE_URI`). They fill
  two Text actions at the top of the shortcut (decoded). The user first creates a key at
  `/user/profile/api-tokens` ([docs](https://mealie.io/documentation/community-guide/ios-shortcut/)).
  The key is sent as `Authorization: Bearer <key>`.
- **Accepts** (decoded `WFWorkflowInputContentItemClasses`): Images, URLs, Safari web pages. Shown in
  the share sheet (`ActionExtension`). Run from the Home Screen, it offers a menu: Take Photos,
  Select Photos, Scan URL (photograph a printed URL and OCR it), Paste URL.
- **What it sends** (decoded):
  1. `GET <uri>/api/users/self` with the Bearer header, which reads the user's `group`. That checks
     the key and builds the recipe link later.
  2. From Safari: "Run JavaScript on Web Page" runs
     `completion(new XMLSerializer().serializeToString(document))`, then
     `POST <uri>/api/recipes/create/html-or-json`, JSON `{"data": "<page html>"}`. The docs explain
     why: sites block scraping bots, so the shortcut sends the HTML the phone already loaded. This
     needs Settings → Shortcuts → Advanced → Allow Running Scripts.
  3. Images: combined into **one** image ("Combine Images", vertical or grid), then
     `POST <uri>/api/recipes/create/image` as a multipart **Form** with a file field `images`. The
     docs say why: "it is currently impossible to send multiple files in 1 web request via
     Shortcuts," and the combined image "can result in slightly less-accurate text recognition."
  4. Otherwise: `POST <uri>/api/recipes/create/url`, JSON `{"url": …}`.
- **Feedback** (decoded): on failure, a **Show Alert** reading "Error sending the recipe to Mealie:
  <response body>". On success, a **menu** asking "Would you like to open this recipe in Mealie
  now?" Yes opens `<uri>/g/<group>/r/<slug>` in the browser. The call waits for the import to finish
  (the response is the recipe slug).
- **Pain points:**
  - Endpoint renames break it. `/api/recipes/create-url` became `/api/recipes/create/url` in v2 and
    the shortcut failed with 405 and 404 ([#4407](https://github.com/mealie-recipes/mealie/issues/4407),
    [#5230](https://github.com/mealie-recipes/mealie/issues/5230)). It also broke when 1.0 made the
    API require auth: "I can not seem to figure out how to pass the API token to it… when the api
    was made more secure, these shortcuts broke" ([#2092](https://github.com/mealie-recipes/mealie/issues/2092)).
  - Sharing from Photos fails with a conversion error, "no conversion from photo media to URL". The
    maintainers answered: "The failure is inside the community contributed iOS shortcut, which lives
    outside this repo" ([#7069](https://github.com/mealie-recipes/mealie/issues/7069)).
  - Behind Cloudflare Zero Trust or any login portal, the shortcut cannot authenticate and hangs. A
    Shortcut can send a header but cannot do an interactive login
    ([#4742](https://github.com/mealie-recipes/mealie/issues/4742)). Apple confirms OAuth 2 is not
    supported (see Platform).
  - A half-failed image import left a "zombie" recipe that could not be opened or deleted
    ([#4222](https://github.com/mealie-recipes/mealie/issues/4222)).
- **Satellites:** InstagramToMealie has a community Shortcut that sends `instagram.com` links to a
  separate service and everything else to Mealie, with three setup variables
  ([discussion #4](https://github.com/JoTec2002/InstagramToMealie/discussions/4)). Recipe2Mealie is
  a native iOS 26 app built because "Shortcuts kept running into their limits". It takes shares of
  Reels, TikToks, YouTube Shorts and photos, and signs in by account, API token or OIDC, then mints
  its own revocable token ([repo](https://github.com/RainerZufall00/recipe2mealie)).

### Norish (self-hosted). Maintainer-shared Shortcut, token shape, minimal

- **Distribution:** the maintainer posted an iCloud link in an issue, not in the docs
  ([#126](https://github.com/norish-recipes/norish/issues/126)):
  [icloud.com/shortcuts/2331e61ecd6e4a4a8ffca9ac5e39fe84](https://www.icloud.com/shortcuts/2331e61ecd6e4a4a8ffca9ac5e39fe84)
  ("Send recipe to Norish 1", created 2025-12-23).
- **Token:** **no import questions** (decoded). The user edits the placeholder `<your_api_key>` in an
  `x-api-key` header and `https://<your_url>/api/import/recipe` in the URL by hand. Keys come from
  Settings → User → API keys and can go in `x-api-key` or `Authorization: Bearer`
  ([API docs](https://docs.norish.dev/reference/api/)). The settings screen suggests "e.g., iOS
  Shortcut" as a key name ([i18n](https://github.com/norish-recipes/norish/blob/main/packages/i18n/src/messages/en/settings.json)).
- **Accepts:** Apps and URLs (decoded).
- **Sends:** one `POST` with JSON `{"url": <shortcut input>}`. The current documented endpoint is
  `POST /api/v1/recipes/import/url` with `{url, forceAI}`, which answers `{"status":"queued", …}`.
  Pasted text goes to `POST /api/v1/recipes/import/paste` with `{text}`, at most 100,000 characters
  per recipe ([API docs](https://docs.norish.dev/reference/api/)).
- **Feedback** (decoded): **Show Notification**, "Recipe is importing" or "Failed to import". The
  import is queued, so the notification only reports that the request was accepted.
- **Pain points:**
  - The shared shortcut still points at the old path. A user's shortcut "not working since several
    versions" showed "Converting failed…". The fix was the new `/api/v1/…` path, and then a reverse
    proxy that blocked it ([#619](https://github.com/norish-recipes/norish/issues/619)).
  - A TikTok short link (`https://vt.tiktok.com/ZSQck453h/`) failed the server's URL validation
    ([#472](https://github.com/norish-recipes/norish/issues/472)).
  - The maintainer was "not sure if it works from an app", meaning shares from inside other apps
    rather than Safari ([#126](https://github.com/norish-recipes/norish/issues/126)). A Web Share
    Target request stays open for Android
    ([#570](https://github.com/norish-recipes/norish/issues/570)).

### Tandoor (self-hosted). Third-party Shortcut, token shape, bookmarklet behind it

- **Path:** "Add to Tandoor" by tomtjes, "not officially supported and… created independently of the
  Tandoor project," on [RoutineHub](https://routinehub.co/shortcut/12612/) and
  [ShareShortcuts](https://shareshortcuts.com/shortcuts/2315-add-to-tandoor.html) (35,186 unique
  downloads there). Tandoor's own mobile story is the PWA
  ([FAQ](https://docs.tandoor.dev/faq/)). Its manifest has a `share_target` (GET `/recipe/import`,
  `title`/`url`/`text`) ([views.py](https://github.com/TandoorRecipes/recipes/blob/develop/cookbook/views/views.py)),
  which helps Android only. An open request asks to extend it to pictures and PDFs
  ([#4220](https://github.com/TandoorRecipes/recipes/issues/4220)).
- **Accepts:** "Safari webpages, photos/screenshots, PDFs or text files." Where Tandoor cannot import
  natively, it hands the text to the ChatGPT or Perplexity iPhone apps, or Gemini with the user's own
  API key ([RoutineHub](https://routinehub.co/shortcut/12612/)).
- **Token:** "During initial setup you'll be asked to enter your Tandoor's URL and an API Token." The
  settings are kept in a file in the Shortcuts iCloud folder (a `config.json`; a different file name
  allows several copies) ([RoutineHub](https://routinehub.co/shortcut/12612/),
  [ShareShortcuts](https://shareshortcuts.com/shortcuts/2315-add-to-tandoor.html)).
- **Sends:** its embedded comments say it gets the page HTML by script and uses "regular bookmarklet
  import" ([ShareShortcuts](https://shareshortcuts.com/shortcuts/2315-add-to-tandoor.html)).
  Tandoor's bookmarklet flow, seen in server logs, is `POST /api/bookmarklet-import/` and then opens
  `/data/import/url?id=<n>` for review ([#3505](https://github.com/TandoorRecipes/recipes/issues/3505)).
  **Unverified** for the shortcut itself: RoutineHub's download sits behind a bot wall, so I did not
  decode it.
- **Pain points:** it needs iOS 18 and Tandoor 2, and dropped Tandoor 1 in v2.10. It asks for many
  permissions: scripts, a daily update check against a third-party server, contacts used for menus,
  and files ([RoutineHub](https://routinehub.co/shortcut/12612/)). Part of it was "CURRENTLY
  DYSFUNCTIONAL DUE TO CHANGES AT SCRAPER PROJECT"
  ([ShareShortcuts](https://shareshortcuts.com/shortcuts/2315-add-to-tandoor.html)).

### Nextcloud Cookbook (self-hosted). Third-party Shortcut, Basic auth

- "Add to Cookbook" by Schlagmichdoch on [RoutineHub](https://routinehub.co/shortcut/9512/), v0.1.2,
  last updated June 2021, 511 downloads. Setup asks for the instance URL, the username and a Nextcloud
  **app password**: "DO NOT USE YOUR ACTUAL PASSWORD." It "also works from apps that share more than
  the actual URL: 'Try this new recipe: https://url.me'", so it pulls the URL out of shared text.
- The server endpoint is `POST /apps/cookbook/api/v1/import`
  ([routes.php](https://github.com/nextcloud/cookbook/blob/master/appinfo/routes.php)).
- The third-party native client
  [Nextcloud-Cookbook-iOS](https://github.com/VincentMeilinger/Nextcloud-Cookbook-iOS) has no share
  extension in its source tree.

### RecipeSage (self-hosted or hosted). Bookmarklet, browser shape, nothing iOS-specific

- On iOS it installs as a Home Screen web app
  ([download page](https://recipesage.com/download/)). Its manifest's `share_target` maps
  `url` → `autofill-url` and `text` → `sharetarget-text`
  ([manifest.json](https://github.com/julianpoy/RecipeSage/blob/master/packages/frontend/src/manifest.json)).
  That helps Android only.
- The "Recipe auto-clip bookmark" is
  `location.href='https://recipesage.com/app/edit-recipe/new?autofill-url='+encodeURIComponent(location.href)`:
  it opens RecipeSage's own edit page with the link prefilled and uses the browser's session
  ([Download.astro](https://github.com/julianpoy/RecipeSage/blob/master/packages/www/src/components/Download.astro)).
  The page recommends the browser extension over it. I found no iOS Shortcut and no share-sheet issue
  in its tracker.

### KitchenOwl (self-hosted). Native app with a share extension

- The Flutter iOS app ships a `ShareExtension` whose activation rule accepts anything conforming to
  `public.text` or `public.url`
  ([Info.plist](https://github.com/TomBursch/kitchenowl/blob/main/kitchenowl/ios/ShareExtension/Info.plist)).
  Docs: "You can share recipe links to the KitchenOwl mobile app to directly add it to your recipe
  list" ([Quick Tips](https://docs.kitchenowl.org/latest/Tips-%26-Tricks/)). It handles links and
  text, not images.

### Recipya (self-hosted). Nothing found

- No `share_target`, no Shortcut, no share-sheet issue in the repo. One user asked to "share into
  the PWA from iOS and Android" ([#252](https://github.com/reaper47/recipya/issues/252), open).

### Grocy (self-hosted). No recipe import at all

- Recipe import requests are closed as duplicates. The maintainer says the hard part is mapping
  ingredients onto Grocy products and units
  ([#445](https://github.com/grocy/grocy/issues/445), [#1519](https://github.com/grocy/grocy/issues/1519)).
  The Grocy Shortcuts on RoutineHub manage stock, not recipe import
  ([RoutineHub 7061](https://routinehub.co/shortcut/7061/)).

### Recipe Filer (commercial web app, no native iOS app). The closest match to Kamosu

This is the one example found of a **web-only** recipe app that wires the iPhone share sheet with a
Shortcut and documents both shapes. Everything below comes from its
[setup page](https://recipefiler.com/ios-share-setup) and its shipped JavaScript bundle
(`https://recipefiler.com/static/js/main.a3e2888c.js`; the file name is a build hash and will change).

- **Recommended, "zero paste" browser shape.** Four actions: **Get URLs from Input** with "If
  there's no input" set to **Clipboard**, then **URL Encode**, then a **Text** action holding
  `https://recipefiler.com/share-import?url=<encoded>` ("Use Text, not URL — URL action drops the
  query string"), then **Open URLs**. Share Sheet Types: URLs only. The page tells users: "The
  shortcut recognises you via your Safari session — so please sign in before installing." Admin
  notes in the bundle: "No token, no editing, no per-user config — users just install and go."
- **Clipboard fallback for Instagram.** The bundle calls the clipboard setting "what makes Instagram
  Reels work", and Home Screen tap after Copy Link is "the reliable path for Instagram Reels". The
  share sheet is not reliable from Instagram.
  **Corrected in the design session:** Instagram's own Share button has a "Share to…" entry that
  opens the system Share menu, where Shortcuts are listed. Recipe Filer's claim is about the
  Shortcut not appearing in Instagram's own share panel, one step earlier.
- **Landing page.** `/share-import` takes `url`, `text` or `title`, finds the first `https?://` in
  any of them, and sends you to the login page if you are signed out. It POSTs to `/api/share-import`
  and shows a toast, "Saved to your Inbox from <source>", with a vibration. On error it explains in
  plain words and offers to paste the URL by hand.
- **Power-user token shape.** A Text action holding the user's personal `rfst_…` token, then `POST
  https://recipefiler.com/api/share-import-token` with JSON `{token, url}`, then **Show
  Notification** "Recipe saved!". The token goes in the JSON body, not a header. The admin screen
  rejects a pasted `rfst_` token where an iCloud link belongs ("Your rfst_… token belongs INSIDE the
  shortcut"), which suggests users confused the two.

### Commercial native apps (each has its own iOS share extension)

| App | What the share path accepts | What happens after the tap | Source |
|---|---|---|---|
| Paprika 3 | A Safari page: "press the Paprika 3 icon to download and save the recipe". It can also offer to open a URL found on the clipboard at launch. | Saves straight into Paprika | [User Guide](https://www.paprikaapp.com/help/ipad/) |
| Mela | "All it needs is a link to site containing a recipe." Separately, an in-app Scan reads a cookbook page by camera. | Opens Mela's recipe viewer, then "Add to Mela" | [Help](https://mela.recipes/help/). 2.5 added recipes from YouTube, TikTok and Instagram video descriptions ([MacStories](https://www.macstories.net/reviews/mela-1-6-adds-web-search-engine-and-recipe-import-from-youtube-instagram-and-tiktok-videos/)) |
| Crouton | Website links. Its troubleshooting page says Instagram and TikTok links often fail and to paste the caption or screenshot it instead, and warns against sharing "a share-sheet wrapper, or an app link". Photo, PDF and pasted-text "Smart Import" is paid (Discover). | n/a | [Troubleshooting guide](https://combustioninc.gorgias.help/en-US/crouton-recipe-import-troubleshooting-guide-7701731) |
| Pestle | Instagram Reels and TikTok video shares, processed on-device. Also "any other plain text source… anywhere where you can highlight text and share it". | A loading screen in the app. It asks for a name when unsure. | [Pestle blog](https://pestlechef.app/blog/save-recipes-from-instagram), [TechCrunch](https://techcrunch.com/2024/11/25/pestle-recipe-app-can-now-save-dishes-from-tiktok/) |
| ReciMe | Instagram post or reel via Share → Share to → ReciMe. Reads the caption, then the video's audio, then looks for the original site. | "imported straight into your ReciMe app" | [Help](https://recime.app/help/en/articles/11596425-import-from-instagram) |
| Honeydew | TikTok or Instagram via the share menu, or Copy Link and paste in the app. AI extraction. | In-app | [Honeydew blog](https://honeydewcook.com/blog/import-recipes-metadata-social-media) |
| Samsung Food | Share extension from the browser or "any of your favorite apps". Photo scan is in-app, 3 free scans, unlimited with Food+. | In-app | [Help](https://support.samsungfood.com/hc/en-us/articles/18756427379476-How-to-Save-Recipes-to-Your-Samsung-Food-Recipe-Box) |

Note on Paprika: a community thread reports Paprika is missing from the share sheet in some other
apps ([Reddit](https://www.reddit.com/r/PaprikaApp/comments/1caohmo/on_ios_how_to_get_a_recipe_from_another_app_into/),
search snippet only, not fetched).

## Platform facts

### Web Share Target on iOS: not supported (October 2026)

- MDN's compatibility data marks `share_target` as `version_added: false` for Safari, with iOS
  mirroring it, linking [WebKit bug 194593](https://bugs.webkit.org/show_bug.cgi?id=194593)
  ([BCD share_target.json](https://github.com/mdn/browser-compat-data/blob/main/manifests/webapp/share_target.json);
  BCD v8.1.7 released 2026-10-22). Chrome Android has it since 76.
- The WebKit bug "Add support for Web Share Target API" is still **NEW**, filed 2019-02-13, last
  touched 2026-05-23 with user +1s
  ([REST](https://bugs.webkit.org/rest/bug/194593?include_fields=status,resolution,last_change_time)).
  WebKit's standards position is **neutral**, with "concerns: security" and "concerns: integration"
  ([standards-positions #11](https://github.com/WebKit/standards-positions/issues/11)).
- "WebKit Features in Safari 26.0" does not mention it
  ([post](https://webkit.org/blog/17333/webkit-features-in-safari-26-0/), checked by text search).
- caniuse has no separate entry for share targets. Its `web-share` entry covers the *sending* API
  (`navigator.share`), which iOS Safari has supported since 12.2
  ([caniuse data](https://github.com/Fyrd/caniuse/blob/main/features-json/web-share.json)). That lets
  Kamosu share *out*, not receive.

### What a Shortcut receives from the share sheet

- Apple's input types: Images, Media, Files, Folder, PDFs, Text, Rich text ("including HTML from web
  pages"), Safari web pages ("Pages shared from the Safari app or from the Safari View Controller in
  third-party apps"), URLs, Articles, and others. The default is Any. Narrowing the types decides
  where the shortcut appears ("a shortcut that accepts only URLs appears… from Safari, but does not
  appear… from Photos"). Apple's tip: "To explore what an app passes into Shortcuts, set up a
  shortcut that accepts Anything and includes only a View Content Graph action"
  ([Input types](https://support.apple.com/en-gb/guide/shortcuts/apd7644168e1/ios),
  [Limit the input](https://support.apple.com/en-gb/guide/shortcuts/apd8195f96d6/ios)).
- Some apps share text that contains a URL rather than a URL. Mealie's manifest comment says so,
  KitchenOwl accepts `public.text`, the Nextcloud shortcut handles it, and Recipe Filer's page
  regex-extracts the link (sources above). TikTok shares a `vt.tiktok.com` short link
  ([Norish #472](https://github.com/norish-recipes/norish/issues/472)).
- "Run JavaScript on Web Page" works only on a Safari web-page input and needs Settings → Shortcuts
  → Advanced → **Allow Running Scripts**. It shows a per-shortcut, per-page permission prompt the
  first time ([Advanced settings](https://support.apple.com/en-gb/guide/shortcuts/apdfeb05586f/ios)).

### How "Get Contents of URL" sends

- Methods: GET, POST, PUT, PATCH, DELETE. With POST, PUT or PATCH a **Request Body** appears that
  "allows you to send JSON, a Form or a File"
  ([Request your first API](https://support.apple.com/en-gb/guide/shortcuts/apd58d46713f/ios)).
  Headers are free-form key/value pairs (Mealie's decoded shortcut sets `Authorization` and
  `Content-Type`).
- A **Form** body can carry a file field. Mealie sends the image as form field `images` (decoded,
  item type file). A **File** body sends the raw bytes as the whole request (Apple names the option;
  how it sets Content-Type is **unverified**).
- **One file per field.** Mealie's docs say sending several files in one request is "currently
  impossible", which is why its shortcut stitches photos into a single image
  ([Mealie docs](https://mealie.io/documentation/community-guide/ios-shortcut/)).
- **No OAuth 2**: Apple says it "is currently not supported"
  ([API limitations](https://support.apple.com/en-gb/guide/shortcuts/apd891a6c84e/ios)). A Shortcut
  authenticates with a static secret in a header or body, or not at all, as Recipe Filer's browser
  shape does.
- **Limits.** Apple documents no request timeout and no body-size cap. The "API limitations" page
  covers only OAuth, multi-call APIs and rate limiting. Settings → Shortcuts → Advanced has **Allow
  Sharing Large Amounts of Data**, for "shortcuts [that] require access to large amounts of data"
  ([privacy settings](https://support.apple.com/en-gb/guide/shortcuts/apd961a4fc65/ios)). A numeric
  timeout is **unverified**: I found no primary source, and one Tavily search turned up only forum
  threads with no figure. The prior art avoids the question by answering fast: Norish answers
  "queued".
- **Feedback options:** Show Notification "runs instantly and continues", while Show Alert "pauses
  the shortcut" ([Show Notification](https://support.apple.com/en-gb/guide/shortcuts/apd2175adcab/ios)).
  Norish and Recipe Filer's token shape notify. Mealie alerts and asks a question. The browser shape
  shows the app's own page.

### Sharing a Shortcut, and getting a secret into it

- **Distribution:** an **iCloud link** (`https://www.icloud.com/shortcuts/<id>`, made with Copy
  iCloud Link, opens a description with Get Shortcut) or a **file** (Export File, signed "Anyone" or
  "People Who Know Me"). For "Anyone", "Apple will receive a copy of your shortcut for validation."
  Stopping sharing is done from the Shortcuts app
  ([Share shortcuts](https://support.apple.com/en-gb/guide/shortcuts/apdf01f8c054/ios)).
- **Import questions:** the author marks fields that ask the recipient at install time, with
  optional default answers. "If an import question is present for a field, your personal information
  is not shared — that field is cleared when the shortcut is shared"
  ([Import questions](https://support.apple.com/en-gb/guide/shortcuts/apdf330fd3a0/ios)). So a field
  **without** a question goes out with whatever value it holds: a Shortcut **can** be shared with the
  secret already filled in, and anyone holding the link gets the secret (inference from Apple's
  wording).
- **A server cannot mint a per-user shortcut on its own.** Since iOS 15, `.shortcut` files must be
  signed. Apple's documented way to sign outside the app is `shortcuts sign` on a Mac, and "Apple
  receives a copy for validation"
  ([Run shortcuts from the command line](https://support.apple.com/en-gb/guide/shortcuts-mac/apd455c82f02/mac)).
  The requirement dates from iOS 15 per community format docs
  ([Cherri](https://cherrilang.org/compiler/signing.html), [0xilis](https://github.com/0xilis/blog/blob/main/shortcuts/fileformat.md)).
  RoutineHub sells signing ("HubSign") to members ([RoutineHub](https://routinehub.co/shortcut/9512/)).
  A token-prefilled install link per person therefore needs a Mac, or someone with the phone in hand,
  to make it.
- **Untrusted files:** receiving a shortcut privately (AirDrop, Messages) needs Settings → Shortcuts
  → **Private Sharing** turned on
  ([privacy settings](https://support.apple.com/en-gb/guide/shortcuts/apd961a4fc65/ios)).
- **Where prior art keeps the secret:** Mealie uses an import question that fills a Text action.
  Norish has the user edit the header by hand. Tandoor's shortcut keeps a config file in iCloud
  Drive's Shortcuts folder. Recipe Filer has the user paste a token into a Text action, or needs no
  token at all.

### The browser shape and Home Screen web apps

- A Home Screen web app "stores website data separately from Safari". Since iOS 17.2, "the cookies
  are copied over, including the information about login state" **once, when the web app is
  created**: "After a user adds a web app to the Home Screen or Dock, no other website data is
  shared" ([WebKit Safari 17.2](https://webkit.org/blog/14787/webkit-features-in-safari-17-2/),
  [WebKit tracking prevention](https://webkit.org/tracking-prevention/), summarised by
  [OpenPWA](https://openpwa.net/reference/storage/home-screen-storage-isolation/)).
- So a Shortcut that runs "Open URLs" on a Kamosu link lands in **Safari's** cookie jar (inference;
  that "Open URLs" opens the default browser and not the Home Screen web app is **unverified**). The
  browser shape works only if the person is signed in to Kamosu in Safari itself, which is why Recipe
  Filer says "please sign in before installing". Signing in to the Home Screen web app afterwards
  does not count. A device that has not opened Kamosu in Safari within the browser's 400 days has no
  Session there, and the browser shape sends that person to the login page, as Recipe Filer's page
  does.

## What this means for Kamosu (inference, not finding)

- The lean going in, "an iOS Shortcut that calls Kamosu's HTTP API", is the **token shape** that
  Mealie, Norish, Tandoor and Nextcloud Cookbook use. It is the established pattern for self-hosted
  apps with no native app, because a PWA cannot appear in the iOS share sheet.
- The failures above share two causes. First, the shortcut lives outside the project, so an endpoint
  rename breaks it silently (Mealie twice, Norish once). Second, it is hard to set up: tokens are
  pasted by hand, and placeholders are edited by hand with no import question. Kamosu sidesteps the
  first by keeping one stable Operation for "an agent, read this". The second is where the
  non-technical person gets stuck.
- The **browser shape** removes the token entirely, and Recipe Filer recommends it for exactly that
  reason. It costs three things. It opens Safari, needs a live Safari Session, and does not reach the Home Screen
  web app. In its favour, the person sees Kamosu's own page saying what happened, in Kamosu's words
  and languages, instead of a notification.
- For **photos** (a cookbook page), only the token shape works, since a URL cannot carry an image.
  Mealie's shortcut is the worked example: a multipart form with one file per field, and several
  pictures stitched into one. A Kamosu endpoint that takes one file per request, with the Shortcut
  repeating per picture, would avoid the loss of recognition accuracy Mealie reports. That is a
  design option, not prior art.
- For **Instagram**, Recipe Filer found the share sheet unreliable and added a clipboard fallback
  (Copy Link, then tap the shortcut on the Home Screen). Kamosu took no fallback, since "Share to…"
  reaches the system Share menu (corrected above). Crouton tells users to paste the caption or
  screenshot. ReciMe and Pestle handle the video link themselves, which Kamosu's agent could do with
  the link alone.
- Answering "queued" at once and saying so in a notification (Norish) avoids an unknown Shortcut
  timeout. Kamosu's Jobs already work this way.

## Fallback calls made

- Tavily search: 1 call ("iOS Shortcuts "Get Contents of URL" request timeout how many seconds"),
  after DonSeTch returned irrelevant results for the timeout and memory queries.
- bladebro: none. Firecrawl: none.
