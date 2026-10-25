# How self-hosted apps run a public demo

Researched 2026-10-21 for a design session on a Kamosu demo mode. Every claim carries a link. Source code links point at the default branch as read on that day. "Live" means I fetched the demo itself. Where I found nothing, the entry says so.

## How to read this

- Four shapes turned up. **Blocked**: the server refuses writes. **Shared**: everyone writes into one instance, which is wiped on a schedule. **Private**: each visitor gets their own sandbox or their own account. **In the browser**: the app runs against fake data on the visitor's machine.
- "Flag" means the switch that turns demo behaviour on in the code. Many apps have none: the demo is an ordinary instance with a login hint and a reset job outside the repository.
- Reset schedules and abuse handling almost never live in the app's repository. They sit in the maintainers' hosting setup, so they are often undocumented. I say "not found" rather than guess.

## Summary table

| App | Shape | Visitor gets in by | Flag in code | Reset |
| --- | --- | --- | --- | --- |
| Mealie | Shared | Credentials shown on login page | `IS_DEMO` (narrow) | Not found |
| Tandoor | Shared, one Space | Auto-login link, or demo / demo | `Space.demo` database field | Not found |
| KitchenOwl | No demo found | n/a | n/a | n/a |
| Grocy | Shared, plus private on request | No login at all | `MODE` set to `demo` | Not found |
| Paperless-ngx | Shared | demo / demo in README | None | "Frequently" |
| Immich | Shared, uploads off | Message and one-click link on login page | None (admin message setting) | "Regularly" |
| Actual Budget | In the browser | "Try demo" button | Client `testMode` | Not needed |
| Vikunja | Shared, open sign-up | Sign-up | `service.demomode` | "Regular intervals" |
| Linkwarden | Blocked | "Login as demo user" button | `NEXT_PUBLIC_DEMO` | Not needed |
| Home Assistant | In the browser | None needed | Separate demo build | Not needed |
| Jellyfin | Shared | `demo` user, empty password | None | Daily 06:00 US Eastern |
| Nextcloud | Private, 60 minutes | Form with email address | Not public | Instance expires |
| Karakeep | Blocked | Credentials shown on sign-in page | `DEMO_MODE` | Not needed |
| Memos | Shared | Seeded users | `--demo` flag | "Periodically" |
| Firefly III | Shared, restricted role | Credentials in docs and on login page | `is_demo_site` setting, `demo` role | Not found |
| BookStack | Shared, restricted | Admin login shown on website | `APP_ENV=demo` | Every 30 minutes |
| HedgeDoc | Shared, now shut | n/a | None | n/a |

## Per app

### Mealie

1. **Writes.** Shared and allowed. The flag blocks very little: only the default user's update, delete and password change are skipped ([`repository_users.py`](https://github.com/mealie-recipes/mealie/blob/mealie-next/mealie/repos/repository_users.py)). Sign-up is open on the live demo (`allowSignup: true` in [`/api/app/about`](https://demo.mealie.io/api/app/about)).
2. **Reset and abuse.** Not found in the repository or docs. An issue about the demo returning 404 ends with a maintainer saying "reset the instance seems to be good now", which suggests a manual reset ([issue 2667](https://github.com/mealie-recipes/mealie/issues/2667)).
3. **Entry.** The login page shows the default account, `changeme@example.com` / `MyPassword`, in an info box and pre-fills the form ([`login.vue`](https://github.com/mealie-recipes/mealie/blob/mealie-next/frontend/app/pages/login.vue)). That box is Mealie's normal first-run hint. It shows on the demo because the demo keeps the default user. A maintainer points people to the page for the credentials ([discussion 3051](https://github.com/mealie-recipes/mealie/discussions/3051)).
4. **Flag.** `IS_DEMO: bool = False` in [`settings.py`](https://github.com/mealie-recipes/mealie/blob/mealie-next/mealie/core/settings/settings.py). It is read in a handful of places: the three user-repository guards above, the startup-info and about endpoints ([`app_about.py`](https://github.com/mealie-recipes/mealie/blob/mealie-next/mealie/routes/app/app_about.py)), and two frontend files that skip the first-run setup redirect when it is on ([`index.vue`](https://github.com/mealie-recipes/mealie/blob/mealie-next/frontend/app/pages/index.vue)). The admin settings page shows "Demo Status".
5. **Costly features.** Nothing demo-specific. The flag is not read anywhere near AI, import or upload code, and the AI docs link their "Group Settings" screen to the demo ([`ai-providers.md`](https://github.com/mealie-recipes/mealie/blob/mealie-next/docs/docs/documentation/getting-started/installation/ai-providers.md)). I could not log in to see whether a provider is configured there.
6. **Banner.** "This is a Demo for version: {version}" in a sticky banner, but only in the `blank` layout, which is the login, register and setup pages ([`blank.vue`](https://github.com/mealie-recipes/mealie/blob/mealie-next/frontend/app/layouts/blank.vue), [`en-US.json`](https://github.com/mealie-recipes/mealie/blob/mealie-next/frontend/app/lang/messages/en-US.json)). Once logged in there is none.

### Tandoor Recipes

1. **Writes.** Shared and allowed. Two endpoints are refused in a demo Space with `PermissionDenied('Not available in demo')`: adding a food or a recipe to the shopping list ([`api.py`](https://github.com/TandoorRecipes/recipes/blob/develop/cookbook/views/api.py)). The storage sync view is also refused for demo Spaces ([same file](https://github.com/TandoorRecipes/recipes/blob/develop/cookbook/views/api.py)). Translations carry an older "This feature is not available in the demo version!" string.
2. **Reset and abuse.** Not found. The demo is the hosted service itself, in a demo Space: the maintainer says "the demo is at the same time also the hosted instance which means I cannot just remove the login" ([issue 4073](https://github.com/TandoorRecipes/recipes/issues/4073)).
3. **Entry.** A link, `app.tandoor.dev/e/demo-auto-login/`, signs the visitor in ([issue 4073](https://github.com/TandoorRecipes/recipes/issues/4073)). Credentials `demo` / `demo` also exist, and a login fault on them was fixed ([issue 4740](https://github.com/TandoorRecipes/recipes/issues/4740)). The auto-login route is not in the open-source repository (I searched `urls.py`), so it likely belongs to the hosted layer.
4. **Flag.** A database field, `Space.demo = BooleanField(default=False)` ([`models.py`](https://github.com/TandoorRecipes/recipes/blob/develop/cookbook/models.py)), exposed read-only through the API. Checked per request through `request.space.demo`. It is set in the admin, not by environment variable.
5. **Costly features.** Spaces carry `ai_enabled` and a monthly credit budget ([`models.py`](https://github.com/TandoorRecipes/recipes/blob/develop/cookbook/models.py), [`ai_helper.py`](https://github.com/TandoorRecipes/recipes/blob/develop/cookbook/helper/ai_helper.py)). Nothing there checks `demo`, so a demo Space is limited the same way any hosted Space is, by credits. The help view hides the self-hosting section in a demo Space ([`HelpView.vue`](https://github.com/TandoorRecipes/recipes/blob/develop/vue3/src/components/display/HelpView.vue)). Recipe import by URL runs for real in the demo: a visitor reports it parsing badly ([issue 4668](https://github.com/TandoorRecipes/recipes/issues/4668)).
6. **Banner.** Not found in code.

### KitchenOwl

No public demo found. The [README](https://github.com/TomBursch/kitchenowl) and [kitchenowl.org](https://kitchenowl.org) link none, and a code search for "demo" finds only translation strings. A hosted cloud answers at `app.kitchenowl.org` (HTTP 200 when I fetched it), but I found nothing calling it a demo, so I treat it as a normal service.

### Grocy

1. **Writes.** Shared and allowed, by design. In demo mode authentication is off and every visitor is user 1 ([`app.php`](https://github.com/grocy/grocy/blob/master/app.php)). Permission edits are overridden so every user keeps ADMIN ([`UsersApiController.php`](https://github.com/grocy/grocy/blob/master/controllers/Api/UsersApiController.php)). There is one database per language (`grocy_<locale>.db`) ([`DatabaseService.php`](https://github.com/grocy/grocy/blob/master/services/DatabaseService.php)).
2. **Reset and abuse.** The reset job is outside the repository and I found no stated schedule. The pre-release demo pulls translations 10 past every hour ([README](https://github.com/grocy/grocy)).
3. **Entry.** No login: the live demo redirects `/` to `/stockoverview` ([live](https://demo.grocy.info/)).
4. **Flag.** `MODE` in `config.php` set to `demo`, `dev` or `prerelease`. The README says it means "authentication is disabled and some demo data will be generated during the database schema migration" ([README, Demo mode](https://github.com/grocy/grocy#demo-mode), [`config-dist.php`](https://github.com/grocy/grocy/blob/master/config-dist.php)). `?nodemodata` skips seeding. A seeder, [`DemoDataGeneratorService.php`](https://github.com/grocy/grocy/blob/master/services/DemoDataGeneratorService.php), writes translated sample data and downloads sample pictures.
5. **Costly features.** Not found. The only barcode plugin is a commented example (the README names `plugins/DemoBarcodeLookupPlugin.php` as an example, [README](https://github.com/grocy/grocy)).
6. **Banner and private option.** The live demo injects a script (not from the repository) that adds a language switcher and a **"Create a private demo instance"** button. Its tooltip says to use it "without interfering with others". The dialog creates a random subdomain, `test-<random>.demo.grocy.info`, with an optional default-data tick, and says instances not accessed for more than a year are removed ([live page source](https://demo.grocy.info/)). Private instances are labelled "Private Demo" next to the logo. Visitors can add `?nodemoadditions` to hide the extras.

### Paperless-ngx

1. **Writes.** Shared and allowed. The README says: "a demo is available at demo.paperless-ngx.com using login `demo` / `demo`. Note: demo content is reset frequently and confidential information should not be uploaded." ([README](https://github.com/paperless-ngx/paperless-ngx#readme)).
2. **Reset and abuse.** "Frequently", with no schedule or mechanism published. The demo runs on donated free-tier hosting, which has been unstable: one thread blames Cloudflare errors on "free tier" ([issue 2070](https://github.com/paperless-ngx/paperless-ngx/issues/2070)). Hosting moved because the previous sponsor ended open-source credits ([PR 14399](https://github.com/paperless-ngx/paperless-ngx/pull/14399)). A cronitor badge in the README shows uptime monitoring.
3. **Entry.** Credentials printed in the README. Nothing found on the login page.
4. **Flag.** None. A search of `src/` finds no demo setting. It is an ordinary instance.
5. **Costly features.** Not documented. I did not log in, so I cannot say whether OCR or the optional AI parts run.
6. **Banner.** None found.

### Immich

1. **Writes.** Shared. Uploads are off. The login-page message on the live demo reads: "This is a demo instance of Immich. It is regularly reset. **Due to abuse, uploads to this instance are disabled.**" ([live `/api/server/config`](https://demo.immich.app/api/server/config)). Other edits are not blocked in what I could see. The demo user is an admin (`isAdmin: true` from `/api/users/me` after logging in with the public credentials, no writes made).
2. **Reset and abuse.** "Regularly reset" per the message. The upload block is the abuse response, and the mechanism is not visible from outside. Since the account is admin and has no quota (`quotaSizeInBytes: null`), the block is probably at the proxy, but that is my inference.
3. **Entry.** The message includes a "Login as demo user" link that fills in the form and submits it ([live `/api/server/config`](https://demo.immich.app/api/server/config)). The README also prints `demo@immich.app` / `demo`, and the mobile app takes `https://demo.immich.app` as its server ([README](https://github.com/immich-app/immich#demo), [dev setup](https://github.com/immich-app/immich/blob/main/docs/docs/developer/setup.md)). The mobile code also has a "populate test login" helper ([`login_form.dart`](https://github.com/immich-app/immich/blob/main/mobile/lib/widgets/forms/login/login_form.dart)).
4. **Flag.** None in code. The message is the ordinary admin setting `server.loginPageMessage`, which accepts HTML ([config file docs](https://docs.immich.app/install/config-file)).
5. **Costly features.** Machine learning appears to run for real: the demo reports smart search, facial recognition, duplicate detection and OCR enabled ([live `/api/server/features`](https://demo.immich.app/api/server/features)). Email is off.
6. **Banner.** The login-page message only.

### Actual Budget

1. **Writes.** Allowed, and local. "Try Demo" creates a budget named "Demo Budget" filled with generated data by `createTestBudget`, inside the client-side server worker that the web build runs in the browser ([`app.ts`](https://github.com/actualbudget/actual/blob/master/packages/loot-core/src/server/budgetfiles/app.ts), [`browser-server/coordinator.ts`](https://github.com/actualbudget/actual/blob/master/packages/loot-core/src/platform/client/browser-server/coordinator.ts)). In test mode it is **not uploaded**: the upload runs only `if (!avoidUpload && !testMode)`.
2. **Reset and abuse.** Not needed. Creating the demo again deletes the old test budget directory first.
3. **Entry.** A "Try Demo" button on the bootstrap screen ([`Bootstrap.tsx`](https://github.com/actualbudget/actual/blob/master/packages/desktop-client/src/components/manager/subscribe/Bootstrap.tsx)) and a "Try the demo" button on the welcome screen ([`WelcomeScreen.tsx`](https://github.com/actualbudget/actual/blob/master/packages/desktop-client/src/components/manager/WelcomeScreen.tsx)).
4. **Flag.** A client option, `createBudget({ testMode: true })` or `{ demoMode: true }`. No server setting.
5. **Costly features.** None involved. No server holds the data.
6. **Banner.** Not found.

### Vikunja

1. **Writes.** Shared and allowed. Sign-up is open, and the sign-up page warns "Your account will be deleted, including all projects, tasks and attachments you might create" ([`Register.vue`](https://github.com/go-vikunja/vikunja/blob/main/frontend/src/views/user/Register.vue), [`en.json`](https://github.com/go-vikunja/vikunja/blob/main/frontend/src/i18n/lang/en.json)). Changing language is dropped from a settings save in demo mode ([`GeneralSettingsForm.vue`](https://github.com/go-vikunja/vikunja/blob/main/frontend/src/views/user/settings/GeneralSettingsForm.vue)).
2. **Reset and abuse.** "Everything will be deleted in regular intervals!" The cadence is not in the repository. The demo, `try.vikunja.io`, "automatically updates and always runs the last unstable build" (from the [versions docs](https://vikunja.io/docs/versions), seen as a search snippet only).
3. **Entry.** Visitors register their own account.
4. **Flag.** `service.demomode`, default false ([`config.go`](https://github.com/go-vikunja/vikunja/blob/main/pkg/config/config.go)). The config doc says it makes the frontend "show a big red warning not to use this instance for real data as it will be cleared out" and that it "was created specifically for usage on try" ([`config-raw.json`](https://github.com/go-vikunja/vikunja/blob/main/config-raw.json)). The only server use is exposing `demo_mode_enabled` in the info endpoint ([`info.go`](https://github.com/go-vikunja/vikunja/blob/main/pkg/routes/api/shared/info.go)). It changes no server behaviour.
5. **Costly features.** Not found.
6. **Banner.** A fixed red bar at the foot of every page, "This instance is in demo mode. Do not use this for real data! Everything will be deleted in regular intervals!", dismissible for the visit ([`DemoMode.vue`](https://github.com/go-vikunja/vikunja/blob/main/frontend/src/components/home/DemoMode.vue)), plus a warning on the login and sign-up pages (seen in [live search results](https://try.vikunja.io/login)).

### Linkwarden

1. **Writes.** Blocked. 29 API route files start their write branches with `if (process.env.NEXT_PUBLIC_DEMO === "true")` and answer 400 with: **"This action is disabled because this is a read-only demo of Linkwarden."** ([example: `links/index.ts`](https://github.com/linkwarden/linkwarden/blob/main/apps/web/pages/api/v1/links/index.ts)). The same guard covers user creation, tags, collections, tokens, migration, archives and the dashboard.
2. **Reset and abuse.** Not needed, because nothing changes.
3. **Entry.** The login page shows a "Demo Only" card with a "Login as demo user" button that signs in with `DEMO_USERNAME` and `DEMO_PASSWORD` ([`login.tsx`](https://github.com/linkwarden/linkwarden/blob/main/apps/web/pages/login.tsx)).
4. **Flag.** Three environment variables, `NEXT_PUBLIC_DEMO`, `NEXT_PUBLIC_DEMO_USERNAME`, `NEXT_PUBLIC_DEMO_PASSWORD` ([`.env.sample`](https://github.com/linkwarden/linkwarden/blob/main/.env.sample)), wrapped by [`isDemoMode.ts`](https://github.com/linkwarden/linkwarden/blob/main/apps/web/lib/api/isDemoMode.ts). The check is repeated by hand in each route.
5. **Costly features.** Blocked with everything else. The card says "uploads are disabled" and points to a free trial of the paid version ([`common.json`](https://github.com/linkwarden/linkwarden/blob/main/apps/web/public/locales/en/common.json)).
6. **Banner.** The card on the login page. It is also an upsell: "If you want to try out the full version, you can sign up for a free trial".

### Home Assistant

1. **Writes.** In the browser. [demo.home-assistant.io](https://demo.home-assistant.io/) is the real frontend with a fake backend: dozens of `mock*` stubs for the registries, history, energy, media player, todo and more ([`demo/src/stubs`](https://github.com/home-assistant/frontend/tree/dev/demo/src/stubs), [`ha-demo.ts`](https://github.com/home-assistant/frontend/blob/dev/demo/src/ha-demo.ts)). Changes live and die in the page.
2. **Reset and abuse.** A reload resets it. No server holds visitor data.
3. **Entry.** None.
4. **Flag.** A separate build entry point (`demo/` in the frontend repository).
5. **Costly features.** None: nothing real is called.
6. **Banner.** Not found. A separate, older idea also exists in core: a `demo` integration that creates fake devices on a real instance for developers ([manifest](https://github.com/home-assistant/core/blob/dev/homeassistant/components/demo/manifest.json)).

### Jellyfin

1. **Writes.** Shared and allowed ("any progress/changes are lost").
2. **Reset and abuse.** The login notice says: "THIS SERVER IS RESET DAILY AT 06:00 Eastern Time; any progress/changes are lost." It also states the media is free-culture or permission-granted, which is the abuse and copyright answer: the content is chosen so nothing needs policing ([live `/Branding/Configuration`](https://demo.jellyfin.org/stable/Branding/Configuration)).
3. **Entry.** User `demo`, no password, written in the login disclaimer. A maintainer closed a complaint that credentials were unclear, saying the disclaimer was there and the reporter's browser blocked it ([jellyfin.org issue 874](https://github.com/jellyfin/jellyfin.org/issues/874)). A proposal for log-in-by-URL-parameter for demos was rejected as "terribly insecure" ([jellyfin-web PR 7650](https://github.com/jellyfin/jellyfin-web/pull/7650)).
4. **Flag.** None. It is a normal server with the standard custom login disclaimer. The public user list shows a plain `demo` user ([live `/Users/Public`](https://demo.jellyfin.org/stable/Users/Public)).
5. **Costly features.** Not found.
6. **Banner.** The login disclaimer only.

### Nextcloud

1. **Writes.** Private. "Take Nextcloud for a 1-hour test drive. Get access to your own Nextcloud installation." ([instant trial page](https://nextcloud.com/instant-trial/)).
2. **Reset and abuse.** "Your account is valid for 60 minutes." The visitor must give a valid business email address, and a link is mailed, which doubles as lead capture and an abuse brake. A forum thread complains that it needs real user data ([forum](https://help.nextcloud.com/t/try-nextcloud-com-needs-user-data/142374), seen as a search snippet).
3. **Entry.** Form, then email link.
4. **Flag.** Not public. It looks like provisioning of separate instances, not an app mode.
5. **Costly features.** Not found.
6. **Banner.** Not found. Note: `try.nextcloud.com` has only an IPv6 address and I could not reach it from here. The page above is the marketing page for the same trial. My attempts at a Tavily extract and a Firecrawl scrape on `try.nextcloud.com` also failed.

### Karakeep (formerly Hoarder)

1. **Writes.** Blocked. Mutations answer: **"Mutations are not allowed in demo mode"** ([`readOnlyMode.ts`](https://github.com/karakeep-app/karakeep/blob/main/packages/shared/readOnlyMode.ts)). Enforced in three layers: a tRPC middleware rejects every mutation ([`trpc/index.ts`](https://github.com/karakeep-app/karakeep/blob/main/packages/trpc/index.ts)), a Hono middleware does the same on REST routes such as asset upload ([`readOnlyMode.ts`](https://github.com/karakeep-app/karakeep/blob/main/packages/api/middlewares/readOnlyMode.ts), [`assets.ts`](https://github.com/karakeep-app/karakeep/blob/main/packages/api/routes/assets.ts)), and the sign-in library refuses all writes except sign-in and sign-out ([`auth.ts`](https://github.com/karakeep-app/karakeep/blob/main/apps/web/server/auth.ts)). In the web app every `ActionButton` disables itself unless it opts out with `ignoreDemoMode` (only the sign-in button does) ([`action-button.tsx`](https://github.com/karakeep-app/karakeep/blob/main/apps/web/components/ui/action-button.tsx)).
2. **Reset and abuse.** Not needed.
3. **Entry.** The sign-in page prints the email and password from the config in an alert ([`SignInForm.tsx`](https://github.com/karakeep-app/karakeep/blob/main/apps/web/components/signin/SignInForm.tsx)).
4. **Flag.** `DEMO_MODE` (default false), with `DEMO_MODE_EMAIL` and `DEMO_MODE_PASSWORD` ([`config.ts`](https://github.com/karakeep-app/karakeep/blob/main/packages/shared/config.ts)). The same helper also serves a separate `DEGRADED_MODE`, so one "read-only" function has two reasons.
5. **Costly features.** Fetching, AI tagging and uploads are all things a write starts, so blocking writes blocks them. No separate switch.
6. **Banner.** A yellow bar: "Demo mode is on. All modifications are disabled." ([`DemoModeBanner.tsx`](https://github.com/karakeep-app/karakeep/blob/main/apps/web/components/DemoModeBanner.tsx)).

### Memos

1. **Writes.** Shared and allowed.
2. **Reset and abuse.** The banner says "Feel free to explore! This demo is reset periodically." ([`en.json`](https://github.com/usememos/memos/blob/main/web/src/locales/en.json)). Mechanism not found.
3. **Entry.** Seeded users (an admin and three others) exist in a SQL dump ([`01__dump.sql`](https://github.com/usememos/memos/blob/main/store/seed/sqlite/01__dump.sql)). I did not find how visitors are told which to use.
4. **Flag.** The command-line flag `--demo` (or the matching setting) ([`main.go`](https://github.com/usememos/memos/blob/main/cmd/memos/main.go)). It requires SQLite ([`profile.go`](https://github.com/usememos/memos/blob/main/internal/profile/profile.go)), writes a separate file `memos_demo.db` instead of `memos_prod.db`, **seeds** the database on every start ([`migrator.go`](https://github.com/usememos/memos/blob/main/store/migrator.go)), uses a fixed token secret, `usememos`, instead of the instance's own ([`server.go`](https://github.com/usememos/memos/blob/main/server/server.go)), and turns on stack-trace logging.
5. **Costly features.** Not found.
6. **Banner.** "Memos demo. Feel free to explore! This demo is reset periodically. Deploy your own Memos" ([`RootLayout.tsx`](https://github.com/usememos/memos/blob/main/web/src/layouts/RootLayout.tsx)).

### Firefly III

1. **Writes.** Shared, with a restricted role. A user with the `demo` role is bounced back with "The feature you try to access is not available to demo users" on admin, profile, export, user-group and MFA pages ([`IsDemoUser.php`](https://github.com/firefly-iii/firefly-iii/blob/main/app/Http/Middleware/IsDemoUser.php), [`firefly.php`](https://github.com/firefly-iii/firefly-iii/blob/main/resources/lang/en_US/firefly.php)). The API answers an empty 403 to that role ([`ApiDemoUser.php`](https://github.com/firefly-iii/firefly-iii/blob/main/app/Api/V1/Middleware/ApiDemoUser.php)). Normal bookkeeping is open. Attachments and export are refused ("The demo user can't upload attachments").
2. **Reset and abuse.** Not found. A "Demo account expired" block reason exists in the language file, hinting at expiring demo accounts.
3. **Entry.** `demo@firefly` / `demo` in the docs ([demo page](https://github.com/firefly-iii/docs/blob/main/docs/docs/explanation/firefly-iii/about/demo.md)). With the flag on, the login page shows a welcome box with the credentials and pre-fills the email ([`login.blade.php`](https://github.com/firefly-iii/firefly-iii/blob/main/resources/views/auth/login.blade.php)).
4. **Flag.** Two things. A site setting `is_demo_site` (a checkbox in the admin configuration page: "this installation will behave as if it is the demo site, which can have weird side effects"), and a `demo` role on the user. Env: `DEMO_USERNAME`, `DEMO_PASSWORD` ([`firefly.php` config](https://github.com/firefly-iii/firefly-iii/blob/main/config/firefly.php)).
5. **Costly features.** Email: the new-access-token notification drops its mail channel on a demo site ([`NewAccessToken.php`](https://github.com/firefly-iii/firefly-iii/blob/main/app/Notifications/User/NewAccessToken.php)).
6. **Banner.** An info alert on every page, with page-specific help text from a `demo` language file, falling back to a generic text ([`session.blade.php`](https://github.com/firefly-iii/firefly-iii/blob/main/resources/views/layout/v3/session.blade.php), [`demo.php`](https://github.com/firefly-iii/firefly-iii/blob/main/resources/lang/en_US/demo.php)). The live site returned 403 to my request, so I did not see it.

### BookStack

1. **Writes.** Shared, mostly open. The website says: "Most standard actions are available using the provided admin login but some actions... have been restricted to keep the demo instance open & available." The restricted ones are user deletion, user updates and setting updates ([bookstackapp.com](https://www.bookstackapp.com/)). In code, `preventAccessInDemoMode()` guards the profile, password, notification and interface updates, settings saves, and user create, update and delete ([`Controller.php`](https://github.com/BookStackApp/BookStack/blob/development/app/Http/Controller.php), [`UserAccountController.php`](https://github.com/BookStackApp/BookStack/blob/development/app/Users/Controllers/UserAccountController.php)). All options stay visible to the admin, they just fail.
2. **Reset and abuse.** "The demo database & image storage is automatically reset every half hour." ([bookstackapp.com](https://www.bookstackapp.com/)). The admin cannot lock others out or change settings, which is the abuse model.
3. **Entry.** Admin email and password printed on the website. The login controller keeps the typed password in the form when demo mode is on ([`LoginController.php`](https://github.com/BookStackApp/BookStack/blob/development/app/Access/Controllers/LoginController.php)).
4. **Flag.** `APP_ENV=demo` ("Options: production, development, demo, testing") ([`app.php`](https://github.com/BookStackApp/BookStack/blob/development/app/Config/app.php), [`.env.example.complete`](https://github.com/BookStackApp/BookStack/blob/development/.env.example.complete)).
5. **Costly features.** Uploads and images stay on, and are covered by the half-hour wipe.
6. **Banner.** None found. The live demo answered 500 when I fetched it on 2026-10-21, which may be a passing fault.

### HedgeDoc (a failure, not a design)

The public demo let anyone make notes anonymously. In June 2024 the maintainers saw "bots are misusing the demo instance to distribute malware or other shady downloads", disabled anonymous note creation, and on 17 July 2024 "due to more abuse of the demo instance, we're forced to temporarily shut down the instance" ([forum thread](https://community.hedgedoc.org/t/status-of-the-demo-instance/1634/4)). Users who kept real notes there lost access and had to wait. The thread closed in February 2025. A [maintainer comment](https://github.com/hedgedoc/hedgedoc/issues/5587) says the instance "died and didn't recover" under spam and bot traffic, and reminds people the demo "is just for trying out". Today the host answers 401. I did not establish whether it returned in a different form.

## Apps skipped

- **KitchenOwl**: no demo, see above.
- No other app was researched. Others with known demos (BookStack and Firefly III aside) were left out for time.

## Patterns

**1. The demo is usually a plain instance plus a reset job outside the repository.** Paperless-ngx, Immich and Jellyfin need no demo code at all. The login-page message is the whole "feature". Where a flag exists, it is mostly cosmetic: Vikunja's only changes a banner, Mealie's protects one account and hides a banner.

**2. Credentials are shown, not hidden.** Nine of the apps put them on the login page, the website or the README (Mealie, Immich, Linkwarden, Karakeep, Firefly III, BookStack, Paperless-ngx, Jellyfin, Tandoor). Immich, Linkwarden and Tandoor go one step further with a one-click or auto-login link. Grocy removes login entirely.

**3. A banner or login message is near universal, and it states the reset.** The wording that recurs is "reset periodically", "reset daily at X" or "everything will be deleted". Cadences found: every 30 minutes (BookStack), daily (Jellyfin), "regularly" or "frequently" (Immich, Paperless-ngx, Memos, Vikunja).

**4. Two designs avoid abuse by construction.**
- *Block writes in the server.* Linkwarden and Karakeep. Karakeep is the most thorough: a single `getReadOnlyModeError` helper, enforced at every API layer, plus disabled buttons in the interface. This also removes every costly path, since AI tagging, link fetching and uploads all start from a write. No reset is needed.
- *Keep the data off the server.* Actual Budget and Home Assistant run on the visitor's machine against generated or stubbed data.

**5. The shared writable demo is the most common and the most fragile.** Mealie, Tandoor, Grocy, Paperless-ngx, Immich, Vikunja, Jellyfin, Memos, Firefly III and BookStack all take real writes from strangers. The cost shows up as abuse: Immich switched uploads off "due to abuse", Paperless-ngx tells visitors not to upload anything confidential, and HedgeDoc was shut for months after malware distribution. Jellyfin sidesteps it by only holding free-culture media. BookStack and Firefly III narrow the damage by stripping the role: the demo admin cannot edit users or settings, and Firefly III's demo user cannot upload or export.

**6. Per-visitor sandboxes exist but are rare and gated.** Nextcloud makes a 60-minute instance behind an email form. Grocy offers an optional "private demo" on a random subdomain, kept a year. Vikunja's sign-up creates a per-visitor account in a wiped shared instance. Nobody I found runs per-visitor sandboxes with an automatic hand-off and no form or click.

**7. How the switch looks.** Four approaches: an environment variable or flag (Karakeep, Linkwarden, Vikunja, Mealie, Memos, BookStack, Grocy's `MODE`), a per-tenant database field (Tandoor), a role on the user (Firefly III, with a site flag beside it), or nothing. Tandoor and Firefly III show that a per-user or per-Space marker can sit beside real accounts on one instance, which suits a hosted demo next to real use.

**8. The costly features are rarely handled explicitly.** Only three apps treat them: Karakeep (blocked with all writes), Linkwarden (blocked, with a "uploads are disabled" notice) and Firefly III (no mail, no attachments, no export for the demo user). Immich runs machine learning for real. For AI, Mealie, Tandoor and Paperless-ngx show no demo-specific switch in code; Tandoor limits it by a per-Space credit balance, which is a general mechanism, not a demo one.

## Trade-offs worth carrying into the session

- **Read-only is cheap to run and honest, but it cannot show the main journey.** Linkwarden's demo cannot add a link, and Karakeep's cannot save a bookmark. Both rely on seeded content. A visitor cannot see an import happen.
- **Shared writable shows everything and needs a wipe, a banner and an abuse plan.** BookStack's half-hour wipe plus a restricted admin is the most complete answer found. Wipe frequency matters: a long gap lets abuse sit visible.
- **Real login is a barrier, and projects get issues about it.** Tandoor and Jellyfin each had reports from visitors who could not find or use the credentials, and Mealie had a "demo does not work, wrong password" thread. Credentials on the page (Immich, Karakeep, Firefly III, Linkwarden) and a one-click login (Immich, Linkwarden, Tandoor) are the fixes.
- **A demo on free hosting is fragile.** Paperless-ngx and Mealie both had outage issues, and Paperless-ngx has changed host as credits ended.
- **A demo flag that is only a banner leaves the real defence elsewhere.** Vikunja's flag is a label. Whether the instance is safe depends on its hosting and wipe.
- **Demo-only code paths are a small attack and maintenance surface.** Linkwarden repeats the same guard by hand in 29 files, which is easy to miss in a new route. Karakeep and BookStack route it through one function, which is easier to audit.

## Method and fallbacks

- Source code was read with `gh` and raw GitHub files. Public demo configuration endpoints were read with `curl` (Mealie, Immich, Jellyfin, Grocy). I logged in once to the Immich demo with its published credentials to read the account type, and made no writes. I did not log in anywhere else.
- DonSeTch fetches of `demo.mealie.io/login` and `try.vikunja.io/login` returned an auth wall, so I used their public JSON endpoints and the repositories.
- Fallback calls made (both failed, both on `try.nextcloud.com`, which is IPv6-only and unreachable from this host):
  1. Tavily extract on `https://try.nextcloud.com` returned HTTP 429 (rate limited).
  2. Firecrawl scrape on `https://try.nextcloud.com/` returned a proxy tunnel error.
- The Nextcloud claim rests on `nextcloud.com/instant-trial/`, fetched with DonSeTch.
- Two claims rest on search snippets and were not fetched: Vikunja's "always runs the last unstable build" and the Nextcloud forum complaint.
