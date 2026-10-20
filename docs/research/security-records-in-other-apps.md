# Security records in other apps

Research date: 2026-10-20. Asked for issue #371, which is about a security record a person can read in Kamosu. The issue needs prior art before its design question is asked: what other apps show a person about sign-ins and security changes, how failed sign-ins appear, how long rows are kept, and what an administrator sees that the person does not.

Ten apps were read: GitHub, Google, Nextcloud, Vaultwarden and Bitwarden, Mastodon, Immich, Jellyfin, Home Assistant, Gitea and authentik. For the open-source ones the source was read at the commits named in each section. For GitHub and Google only the vendor's help pages are public. Nothing was built or run. Every claim carries a confidence mark. **High** means read in source or an official specification. **Medium** means first-party documentation. **Not found** means I looked and saw nothing. Where a claim rests on a community forum volunteer I say so in the text. Forgejo was dropped: it lives on Codeberg, which the GitHub tooling used here cannot search, and it shares Gitea's code only up to its fork point.

## Summary

- **Retention is a setting or a size cap in most apps, and the defaults vary from 28 days to a year.** GitHub keeps 90 days (Medium). Google's "Recent security activity" shows 28 days (Medium at best: a community volunteer, not Google). Gitea's new audit log keeps 30 days by default, and 0 keeps forever (High). Jellyfin's activity log keeps 30 days by default (High). Mastodon's authentication history and IP data go after 1 year, set by an environment variable (High). authentik keeps events 365 days by default (Medium). Vaultwarden keeps events forever unless `EVENTS_DAYS_RETAIN` is set (High). Nextcloud's admin audit log has no age limit, only a 100 MB size cap (High). Nextcloud's activity stream, where its personal security events go, keeps 365 days (High).
- **A personal, readable record is the minority.** Only GitHub, Google, Mastodon and Gitea (audit log, merged to `main` on 2026-09-12) give the signed-in person a list of their own security events with times. Nextcloud gives a thin stream (two-factor and remote wipe only). Bitwarden, Immich and Home Assistant give a list of live sessions or devices, not a history. Jellyfin and authentik show a history to administrators only (High, Medium).
- **Failed sign-ins are one row each, never counted, wherever they are shown.** GitHub (`user.failed_login`), Mastodon, Jellyfin, authentik, Vaultwarden's event table, and Nextcloud's admin audit log all write a row per attempt (High, Medium). Home Assistant is the exception: it keeps one persistent notification slot that the next failure overwrites, and counts attempts per address only inside the ban mechanism (High).
- **Two apps record only the second-factor failure and say nothing about a wrong password.** Mastodon's authentication history writes a failed row only after the password was right, when the one-time code, security key or emailed code fails. Gitea's audit log defines only `user:authentication:fail:twofactor`. A failed row in those two means someone knew the password (High, read as absence of any other call site).
- **The row almost always carries a time and a way of signing in, and often an IP and a browser.** Mastodon: result, method, IP, browser, time. Gitea: actor, scope, action, origin, message, IP, time. Jellyfin: message, IP in a short overview, time, severity. authentik: action, user, client IP, context, time. GitHub's docs list country, not IP (Medium). Nextcloud's personal stream carries a sentence and a time only (High).
- **Instance-wide records separate from the personal one exist in six of ten.** Nextcloud (admin audit), Vaultwarden (organisation event log), Jellyfin (Dashboard activity), Gitea (admin audit view), authentik (Events, Logs) and Mastodon (per-account IP list, successful sign-ins only, no failures) (High). GitHub has organisation and enterprise audit logs, which are a different product from the personal log (Medium).
- **Logs written for fail2ban are a separate surface from the readable record, and every self-hosted app that offers one logs the IP and often the username in a plain text line.** Exact lines are in the sections below. Vaultwarden's is the cleanest to copy: `Username or password is incorrect. Try again. IP: <ip>. Username: <name>.` (High).
- **Logging the attempted username has a cost that two apps have already met.** Vaultwarden escapes the typed name with `escape_debug()`. Home Assistant says in a comment that the user agent is "unsanitized input so we only include it in the log". Immich and Nextcloud log the typed name unescaped (High).
- **A security log that a thief can erase defeats itself, and Google says it plainly.** A community volunteer answered that its activity "can't be deleted". GitHub's log has no delete control in its docs. No app read here lets a person remove a row (Medium).
- **Gitea's audit log is the closest match to a design for Kamosu's shape**: one table, four scopes (user, organisation, repository, administrator), the same columns at each scope, a filter by actor, action and origin, an export that shares the list's filters (the code comment says so; which pages offer it was not checked), and an off-by-default switch (High).

## What stays unverified

- **Google's "Recent security activity" has no first-party help page I could reach.** The 28 days comes from a community volunteer's answer on a locked thread (Jan 2022). What one row shows is described by a third-party blog and by Gmail's own "Last account activity" page, which does say dates, times and IP addresses. Google's "Your devices" page is first-party and is quoted below.
- **GitHub's actual screen.** I read the docs, which list the fields an entry shows, and did not see the page. Whether the IP shows in the web view is not stated; the docs say country.
- **Gitea's audit log is new.** Pull request #38189 "feat: Add audit logging" merged to `main` on 2026-09-12. Gitea v28.0.0 was published 2026-09-29 and v28.2.0 on 2026-10-18. I did not confirm which release first carries it, so it may not have shipped to anyone's server yet.
- **Which Nextcloud message goes where.** The fail2ban page in Nextcloud's manual names `nextcloud.log` and level 2. The audit log is a second file with its own messages, and I read both in source but did not run either.
- **authentik and Immich have no documented fail2ban filter.** Not found in the Immich repository or in authentik's docs. The log lines below are from source.
- **Whether Home Assistant's frontend shows the IP column on a narrow screen.** I read the component and saw the `last_used_ip` field in its text.
- **Mastodon's wrong-password case** is a finding from absence: the only two call sites of the failure writer are in the two-factor concern. A Devise hook somewhere I did not search could write one.
- **Vaultwarden's event table is off by default and I did not see it populated.** Everything is from source.

## GitHub

Source: docs.github.com pages "Reviewing your security log" and "Security log events", fetched 2026-10-20. No source code (GitHub is not open).

1. **Where.** Click your profile picture, then Settings. In the "Archives" section of the sidebar, click "Security log" (Medium).
2. **One row.** An entry is named by a category and an operation, like `repo.create`. The docs say each entry shows, where applicable: the organisation, the actor, the user affected, the repository, the action, "which country the action took place in", and the date and time. The page also says you cannot search by text, only by qualifiers: `operation:`, `repo:`, `actor:`, and an action category (Medium). Sixteen categories are listed, including `passkey`, `personal_access_token`, `public_key`, `two_factor_authentication` and `user`. The export keys include `actor_location.country_code` and `created_at` (Medium). The event reference lists a `user_agent` field on every event, and `passkey_nickname` on `user.login` (Medium).
3. **Failed sign-ins.** One row each. `user.failed_login` is described as "A user tried to sign in with an incorrect username, password, or two-factor authentication code." There are also `user.login`, `user.two_factor_requested` and `user.two_factor_challenge_failure` (Medium).
4. **Retention.** "The security log lists all actions performed within the last 90 days." (Medium).
5. **Instance-wide record.** Organisations and enterprises have their own audit logs with a separate event list. The security-log events page says so in a note (Medium).
6. **Log line for fail2ban.** None. GitHub is a hosted service.

Also: the log can be exported as JSON or CSV from an Export menu, filtered first by qualifiers (Medium).

## Google Account

Sources: support.google.com/accounts/answer/3067630 "See devices with account access"; the Gmail Help page answer/45938 "Last account activity"; community threads 147589165 and 185577389; fetched 2026-10-20.

1. **Where.** Google Account, then "Security & sign-in". Two panels: "Recent security activity" with a "Review security activity" button, and "Your devices" with "Manage all devices" (Medium for the devices panel; the activity panel's location comes from the devices page and the community threads).
2. **One row.** For devices: a device or a session, with a "Signed out" tag, and details on selection. The times are "the last time there was communication between the device or session and Google's systems", so a time may be more recent than the last time you used it. Location is approximate ("a nearby place instead of an exact location"). A session starts "when you sign in on a new device", "when you re-enter your password to verify it's you", on a new browser, when you grant an app access, and in a private window (Medium). For security activity, a user thread asked whether the IP address should appear with each entry; no answer is on record. Gmail's separate "Last account activity" page does list dates, times and IP addresses (Medium).
3. **Failed sign-ins.** Not found. The security alert mail says "device type, time, and location" for an unusual sign-in (Medium). Whether a failed attempt appears as a row is not stated.
4. **Retention.** Devices: "currently signed in ... or have been in the last few weeks" (Medium). Security activity: "You can view security-related activity from the past 28 days." That sentence is from a community volunteer ("Not a Google Employee"), so it is the weakest claim in this note. The same reply says the alerts "can't be deleted".
5. **Instance-wide record.** Not applicable. Google Workspace has an admin audit, which was not read.
6. **Log line for fail2ban.** None.

## Nextcloud

Source: github.com/nextcloud/server `master` at `766b24fc31956995a3fbded0b5ac98ad9cca276e` (read 2026-10-20); github.com/nextcloud/activity `master`; the Nextcloud 36 admin manual pages "Hardening and security guidance" and "Logging".

1. **Where.** Two places. Personal: the Activity app has a filter named "Security" (`SecurityFilter`, getName returns "Security"). Administrator: the optional `admin_audit` app writes a log file, by default `<datadirectory>/audit.log` (High, Medium).
2. **One row.**
   - **Personal "Security" stream** holds four kinds of entry only: "You successfully logged in using two-factor authentication (%1$s)", "A login attempt using two-factor authentication failed (%1$s)", "Remote wipe was started on %1$s" and "Remote wipe has finished on %1$s" (`apps/settings/lib/Activity/SecurityProvider.php`). Two-factor events are emitted from `lib/private/Authentication/TwoFactorAuth/Manager.php` and wipes from `RemoteWipeActivityListener.php`. No IP or device is in the parsed subject. Password sign-in, password change and token creation are not in this stream (High).
   - **Admin audit log** has one line per event, from `AuthEventListener.php`: `Login attempt: "%s"`, `Login successful: "%s"`, `Logout occurred`, and `Login failed: "%s"`, each tagged `app: admin_audit` and an `operation` value. Other listeners cover files, sharing, users and groups, apps, tags, cache, the console, and "critical actions" from other apps. Each line goes through Nextcloud's JSON logger, whose fields the fail2ban filter above relies on (`remoteAddr`, `message`, `time`). That the audit file carries the same fields as `nextcloud.log` follows from the shared logger and was not checked against a real file (Medium).
3. **Failed sign-ins.** One line each in the admin audit log (`Operation::LoginFailed`). In the ordinary `nextcloud.log` a failed password is a level-2 warning `Login failed: <name> (Remote IP: <ip>)` (`lib/private/Authentication/Login/LoggedInCheckCommand.php:31`). In the personal stream, only a failed second factor appears (High).
4. **Retention.**
   - Admin audit log: no age limit. A background job runs every 3 hours and, when the file reaches `log_rotate_size` (100 MB by default, 0 turns rotation off), renames it to `audit.log.1`, overwriting any earlier copy. So at most about two files exist (High, `Rotate.php` and `RotationTrait::rotate`).
   - Personal Activity stream: a daily job deletes entries older than `activity_expire_days`, default 365, minimum 1 (High, `activity/lib/BackgroundJob/ExpireActivities.php`).
5. **Instance-wide record.** Yes, and it is a different file in a different place from the personal stream: `audit.log`, or syslog, with its own `logfile_audit`, `log_type_audit` and `syslog_tag_audit` keys (High).
6. **Log line for fail2ban.** The manual gives this filter, which needs `loglevel` 2 or lower:

```
[Definition]
_groupsre = (?:(?:,?\s*"\w+":(?:"[^"]+"|\w+))*)
failregex = ^\{%(_groupsre)s,?\s*"remoteAddr":"<HOST>"%(_groupsre)s,?\s*"message":"Login failed:
            ^\{%(_groupsre)s,?\s*"remoteAddr":"<HOST>"%(_groupsre)s,?\s*"message":"Two-factor challenge failed:
            ^\{%(_groupsre)s,?\s*"remoteAddr":"<HOST>"%(_groupsre)s,?\s*"message":"Trusted domain error.
datepattern = ,?\s*"time"\s*:\s*"%%Y-%%m-%%d[T ]%%H:%%M:%%S(%%z)?"
```

The filter reads three keys of the one-line JSON: `remoteAddr`, `message` and `time` (Medium).

## Vaultwarden, and the Bitwarden web vault it serves

Sources: github.com/dani-garcia/vaultwarden at `58a980f850464f7d9cdc99a765df36f0a3308b91` and its wiki page "Fail2Ban Setup"; github.com/bitwarden/clients at `86f41566aebe5a024796df8abfb3d4d89ec509c1`; bitwarden.com/help/event-logs. Read 2026-10-20.

1. **Where.**
   - **Personal:** the web vault's Settings, Security, "Devices" (`device-management` route in `apps/web/src/app/auth/settings/security/security-routing.module.ts`). It is a list of devices, not a history (High).
   - **Event log:** only for organisations, in the organisation admin area. Vaultwarden has it behind `ORG_EVENTS_ENABLED`, off by default. A person who is not an organisation admin cannot read their own events (High, `src/api/core/events.rs`: the routes are `/organizations/<id>/events`, `/ciphers/<id>/events` and `/organizations/<id>/users/<id>/events`).
2. **One row.**
   - Devices (Bitwarden's component): a device name, `firstLogin`, `lastActivityDate` shown as "recently active" text, a login status ("Current session", "Request pending") and a pending-approval action. No IP (High).
   - Event table (Vaultwarden `event.rs`): event type, user, organisation, acting user, device type, IP address and event date. User events are 1000 to 1007: logged in, changed password, updated 2FA, disabled 2FA, recovered 2FA, failed login, failed 2FA login, exported vault (High).
3. **Failed sign-ins.** One event row each, type 1005 for a wrong password and 1006 for a wrong second step. Every failure branch in `src/api/identity.rs` carries `UserFailedLogIn` (about twenty places) (High). All of this applies only when `ORG_EVENTS_ENABLED` is on.
4. **Retention.** The event table is kept forever by default. `EVENTS_DAYS_RETAIN` sets a day count and a daily job deletes older rows; with it unset "events are kept indefinitely and the scheduled job is disabled" (High, `.env.template` and `clean_events`). For Bitwarden's cloud, its help says data is "retained indefinitely" and a view shows at most 367 days at a time (Medium).
5. **Instance-wide record.** Separate and richer. The event table is the instance-wide record, written once without an organisation and once per organisation the user belongs to (`log_user_event_impl`). The admin panel is separate and has no event view (High).
6. **Log lines for fail2ban.** These go to the ordinary log, not the event table, and work without `ORG_EVENTS_ENABLED`. The wiki's sample and filter:

```
[YYYY-MM-DD hh:mm:ss][vaultwarden::api::identity][ERROR] Username or password is incorrect. Try again. IP: XXX.XXX.XXX.XXX. Username: email@domain.com.

failregex = ^.*?Username or password is incorrect\. Try again\. IP: <ADDR>\. Username:.*$
```

Source has the same text in `src/api/identity.rs` (`format!("IP: {}. Username: {log_username}.", ip.ip)`) and `src/api/core/two_factor/email.rs`, where the name goes through `email.escape_debug()`. Two more lines in the same wiki: `Invalid admin token. IP: <ADDR>` for the admin panel, and `[ERROR] Invalid TOTP code! Server time: YYYY-MM-DD hh:mm:ss UTC IP: 1.2.3.4` for a wrong authenticator code (High). The wiki warns that behind a reverse proxy the IP reads `127.0.0.1` unless the proxy header is configured (Medium).

## Mastodon

Source: github.com/mastodon/mastodon `main` at `5204e8ab940f870dbf13863ee92c48febfc4eedb`, read 2026-10-20.

1. **Where.** Settings, Account settings (the page for password, two-factor and sessions), where the "Sessions" section carries a link "View authentication history" to `/settings/login_activities` (`app/views/auth/registrations/_sessions.html.haml`). Its title is "Authentication history" (High).
2. **One row.** A green check or a red cross, then one sentence, then the time. Success: "Successful sign-in with %{method} from %{ip} (%{browser})". Failure: "Failed sign-in attempt with %{method} from %{ip} (%{browser})". The method is one of password, two-factor authentication app, email security code, security keys, or the single sign-on provider. The table also stores `failure_reason`, `user_agent` and `provider`; the row text does not show the reason. Newest first, paginated. The page opens with "If you see activity that you don't recognize, consider changing your password and enabling two-factor authentication." (High).
3. **Failed sign-ins.** One row each, but only after the password was right. The failure writer `on_authentication_failure` is called from the one-time code and security key branches only (`app/controllers/concerns/auth/two_factor_authentication_concern.rb:63, 77`). A wrong password writes no row (High, from absence). Failed second-factor attempts also send an email, at most one an hour (`redis.set("2fa_failure_notification:#{user.id}", ..., ex: 1.hour)`). A "suspicious sign-in" email goes out on a successful sign-in from a new place (High).
4. **Retention.** `Scheduler::IpCleanupScheduler` deletes rows older than `IP_RETENTION_PERIOD` (default 1 year, in seconds) and clears the IP on older sessions. It also deletes session records after `SESSION_RETENTION_PERIOD` (default 1 year) (High).
5. **Instance-wide record.** Partly. An administrator sees, per account, a list of IPs with a last-used time, built from a database view `user_ips` that unions the sign-up IP, sessions and login activities where `success = 't'`. Failures are not in that list. Administrators also have a separate moderation action log, which is a record of staff actions, not of sign-ins (High for the view; the action log was not read).
6. **Log line for fail2ban.** Not found in source. Mastodon's built-in throttling is Rack::Attack, which was not read.

## Immich

Source: github.com/immich-app/immich `main` at `fff2bd73f9b38a5a5b510ea69a77a470dd1acf56`, read 2026-10-20.

1. **Where.** Account settings, "Authorized Devices" (the i18n key `authorized_devices`; component `DeviceCard.svelte`). A list of live sessions, with "Current device" marked, a "Last seen" time and a "Logged out device" action (High).
2. **One row.** The `session` table holds: `deviceType`, `deviceOS`, `appVersion`, `createdAt`, `updatedAt`, `expiresAt`, a token and an optional parent session. There is no IP column (High).
3. **Failed sign-ins.** Not shown to anyone in the interface. They are logged only (High).
4. **Retention.** A session lives until it expires or the person logs it out. There is no history to expire (High).
5. **Instance-wide record.** None found. Administrators have no sign-in or security view (Not found).
6. **Log line for fail2ban.** From `server/src/services/auth.service.ts`:

```
Failed login attempt for user ${dto.email} from ip address ${details.clientIp}
```

logged at warn level. Immich's repository has no fail2ban filter (Not found, High for the line itself).

## Jellyfin

Source: github.com/jellyfin/jellyfin `master` at `08c2ba6e0b6499122822f1c5b18bc426104538b4`; jellyfin.org fail2ban page. Read 2026-10-20.

1. **Where.** Administrator only: Dashboard, Activity. A person without the administrator role has no such page. Dashboard, Devices lists devices by user (High, Medium).
2. **One row.** `ActivityLogEntry`: name (a sentence), type, severity, date, user id, an `Overview` and a `ShortOverview`. For a failed sign-in the name is "Failed login attempt from {0}" with the typed username in the blank, and `ShortOverview` is "IP address: {0}", severity Error (High, `AuthenticationFailedLogger.cs` and the English strings). Success is "{0} successfully authenticated". Other rows: "Password has been changed for user {0}" and "User {0} has been locked out" (High).
3. **Failed sign-ins.** One row each (High).
4. **Retention.** `ActivityLogRetentionDays`, default 30. A scheduled task named "TaskCleanActivityLog" deletes older entries; a null value means keep forever (High, `ServerConfiguration.cs:241`, `CleanActivityLogTask.cs`).
5. **Instance-wide record.** The activity log is the instance-wide record. No personal one exists (High).
6. **Log line for fail2ban.** From `UserManager.cs`:

```
Authentication request for {UserName} has been denied (IP: {IP}).
```

at information level. The docs filter is `failregex = ^.*Authentication request for .* has been denied \(IP: "<ADDR>"\)\.` (Medium). Two other lines use the same start: one with no username and one for a disabled account (`... has been denied because this account is currently disabled (IP: {IP}).`) (High). One thing to check before copying the regex: the source format string has no quote marks around the IP, while the docs filter expects them. I did not run Jellyfin, so which is right is unchecked.

## Home Assistant

Source: github.com/home-assistant/core `dev` at `bb97fc8a10ab1ac06d0d46302b47c8083a904cb1`; github.com/home-assistant/frontend `dev`; home-assistant.io "HTTP" integration page. Read 2026-10-20.

1. **Where.** Profile, Security: a card "Refresh tokens" (`ha-refresh-tokens-card.ts`). Each signed-in client is a token, with Delete. Long-lived access tokens sit beside it. For failures, an administrator sees a persistent notification titled "Login attempt failed" (High).
2. **One row.** Token: client name, created time, and "last used" with the time and location, where the location is `last_used_ip` (High, `auth/models.py:105-125` and the card). Sorted by last use. Notification text: "Login attempt or request with invalid authentication from {host} ({address}). See the log for details."
3. **Failed sign-ins.** Not one row each. The notification uses a fixed ID (`NOTIFICATION_ID_LOGIN`), so a new failure replaces the old one. Counts are kept per address only to decide a ban (High). When `login_attempts_threshold` is reached, the address goes into `ip_bans.yaml` with its time, and a second notification, "Banning IP address: Too many login attempts from {address}", appears.
4. **Retention.** Tokens: until deleted. The notification: until dismissed. Bans: "IP bans do not expire automatically" (Medium). The log file follows the normal log rotation, which was not read.
5. **Instance-wide record.** The log file and the notification are the instance's record; no screen aggregates them (High).
6. **Log line for fail2ban.** From `homeassistant/components/http/ban.py`, warning level:

```
Login attempt or request with invalid authentication from {remote_host} ({remote_addr}). Requested URL: '{request.rel_url}'. ({user_agent})
```

and `Banned IP {remote_addr} for too many login attempts`. The built-in ban makes an outside tool unneeded, and the HTTP integration page documents the built-in route, not fail2ban (Medium). The code comment on the user agent reads: "The user-agent is unsanitized input so we only include it in the log".

## Gitea

Source: github.com/go-gitea/gitea `main` at `7610336d74fd74a1a74abfe4c4667c182d6d5652`; docs.gitea.com "Fail2ban Setup". Read 2026-10-20. The Go module path in this commit reads `gitea.dev/...`, so the project has moved its import path; I followed it by commit.

1. **Where.** Personal: Settings, with an "Audit" page (`routers/web/user/setting/audit.go`, `templates/user/settings/audit_logs.tmpl`, title key `audit.title`). The same page exists for an organisation's settings, a repository's settings and the site administration area (`routers/web/{org/setting,repo/setting,admin}/audit.go`). It records nothing until an administrator sets `[audit] RECORD_OUTPUT = database` in `app.ini`; the default is `disabled` (High).
2. **One row.** Seven columns: actor (with an "impersonating as" note when an administrator acted as them, and the credential used, like `access-token:<id>`), scope, action (a code like `user:password`), origin (`ui`, `api`, `cli` or `system`), details (a sentence like "Changed password of user {scope}."), IP address, and timestamp. Filters: actor name, action, origin; sort by time (High). User-scope actions include password change and reset request, email add, change and remove, two-factor enable, regenerate and disable, WebAuthn add, rename and remove, external login add and remove, access token add and remove, OAuth2 grant, and SSH key add and remove.
3. **Failed sign-ins.** Only a failed second factor is defined: `user:authentication:fail:twofactor`, "Failed two-factor authentication for user {scope}." I found no action for a wrong password and none for a successful sign-in (High, from absence in `models/audit/action.go`). The ordinary log does record wrong passwords (below).
4. **Retention.** `RETENTION_DAYS`, default 30; a negative value is treated as 0, which keeps forever. A cron task `delete_old_audit_events`, daily, deletes older rows (High, `modules/setting/audit.go`, `custom/conf/app.example.ini`).
5. **Instance-wide record.** Yes: the administrator view lists every event and leaves the scope blank (`ViewOptions` comment: "The admin view leaves ScopeType empty to list every event"). A person sees only rows in their own scope (High). The old long-open request for this is issue #5312, "User session managment and audit", created 2018-11-10 and still open.
6. **Log lines for fail2ban.** The docs' filter:

```
failregex = .*(Failed authentication attempt|invalid credentials|Attempted access of unknown user).* from <HOST>
```

with sample lines `[I] Failed authentication attempt for user from xxx.xxx.xxx.xxx` (a 2018 line), `[W] Failed authentication attempt from xxx.xxx.xxx.xxx` (SSH) and `[E] invalid credentials from xxx.xxx.xxx.xxx` (Medium). In source the web sign-in logs `Failed authentication attempt for %s from %s: %v` with the typed name, the address and the error (`routers/web/auth/auth.go:316-321`) (High).

## authentik

Source: github.com/goauthentik/authentik `main` at `57e2ec1b677f8882f4e5c8134b79dcd6d4d36b6e`; docs.goauthentik.io "Events" and "Event actions" (version 2026.8). Read 2026-10-20.

1. **Where.** Administrator interface: Events, Logs, and a "Recent events" panel on the Overview dashboard. The user interface settings folder holds `details`, `mfa` and `tokens` and no events page (High, listing of `web/src/user/user-settings`). An enterprise-only map shows logins, failed logins and logouts by location (Medium).
2. **One row.** The `Event` model: action, user (a JSON snapshot with pk, email and username), context (JSON), `client_ip`, created, expires, and the brand. For `login` the context holds `auth_method` and the request path; for `login_failed` it holds the stage that failed, the typed username, a masked password (`"password": "********************"`) and the request. Passwords and credentials are stripped before storing (Medium).
3. **Failed sign-ins.** One event each, action `login_failed` (High). Among event actions there are also `login`, `logout` and a large set for model changes (`model_created` and so on).
4. **Retention.** System, Settings, "event retention", default 365 days. Each event gets an `expires` time from the setting at creation (High for the expiry mechanism, Medium for the default and where it is set). The docs suggest setting it short, like `days=1`, if events are forwarded elsewhere.
5. **Instance-wide record.** The Events list is itself instance-wide. Notification rules can send chosen events to a local notice, an email or a webhook (Medium).
6. **Log line for fail2ban.** None documented. Every event creation is also logged at info level with the message `Created Event` and the fields `action`, `context`, `client_ip` and `user` (`authentik/events/models.py:292-300`), so a filter would match `login_failed` and `client_ip` in authentik's JSON logs (High for the line, Not found for any documented filter).

## Answers by question

| App | Personal screen | Failed sign-in | Kept | Instance-wide |
| --- | --- | --- | --- | --- |
| GitHub | Settings, Archives, Security log | One row each | 90 days (Medium) | Org and enterprise logs |
| Google | Security, Recent security activity; Your devices | Not found | 28 days (volunteer); devices "last few weeks" | Not applicable |
| Nextcloud | Activity, Security filter | Second factor only | 365 days | `audit.log`, 100 MB cap |
| Vaultwarden | Settings, Security, Devices (sessions) | One event row each (org events on) | Forever unless set | Event table, org admins |
| Mastodon | Account settings, Authentication history | Second factor only, one row each | 1 year | Per-account IP list, successes only |
| Immich | Account settings, Authorized Devices | Log only | Until session ends | None found |
| Jellyfin | None | One row each | 30 days | Dashboard, Activity |
| Home Assistant | Profile, Security, Refresh tokens | One notification, overwritten | Until dismissed | Log file |
| Gitea | Settings, Audit (needs a switch) | Second factor only | 30 days | Admin audit view |
| authentik | None | One event each | 365 days | Events, Logs |
