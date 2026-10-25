# Signing out in other apps

Research date: 2026-10-20. Asked for the ticket about signing out of Kamosu (branch `feature/380-sign-out`). Today sign-out is a button on one row of the Sessions list in Settings, Account, and people cannot find it. The design question needs prior art first: where other apps put "Sign out" on a phone and on a wide screen, whether they ask before doing it, whether they warn about changes that have not reached the server, and what the person sees afterwards.

Fourteen things were read: Mealie, Tandoor Recipes, Immich (web and mobile app), the Bitwarden web vault (which Vaultwarden serves), the Bitwarden browser extension and Android app, Nextcloud (web and Android), Gitea, Paprika, Standard Notes, Todoist, Evernote and Apple's iCloud sign-out. Apple's Human Interface Guidelines and Android's design guidance were read for platform rules. For the open-source apps the source was read at the commits named in each section. Nothing was built or run. Every claim carries a mark. **Source** means read in the project's code. **Docs** means first-party documentation. **Third-party** means a forum post or other outside account. **Not found** means I looked and saw nothing.

## Summary

- **Desktop: the account menu is the standard home, and sign-out is its last item.** Gitea, Nextcloud, Bitwarden's web vault, Immich's web app and Tandoor all put it in a menu opened from a picture or initial at the top right (Tandoor, Immich) or in the top bar (Gitea, Nextcloud, Bitwarden). In Gitea, Nextcloud and Bitwarden it sits after a divider at the foot of the menu, below the settings link. Mealie is the odd one: a bare "Logout" button in the top bar (Source).
- **Phone: three patterns, and the account menu is not one of them everywhere.** (1) The same top-bar avatar or button as on desktop: Mealie shows an icon-only button on a narrow screen, Immich's phone app opens a dialog from the avatar in its top bar, Gitea and Nextcloud keep their web menu (Source). (2) A row inside Settings: Bitwarden Android has "Log out" under Settings, Account security; Todoist is Browse tab, gear, "Log out"; Apple puts "Sign Out" at the very bottom of Settings, under your name; Paprika has a "Log out" button under Settings, Cloud Sync (Source for Bitwarden, Docs for the others). (3) A drawer entry: Nextcloud Android has "Log out" in its drawer menu (Source, file name only).
- **Tandoor shows what a bottom tab bar can get wrong.** Its phone bottom bar has a menu button that opens a sheet with Settings, Import, Database, Search, Pantry and Books, and no "Logout". Sign-out is only in the avatar menu in the top bar, which the phone layout keeps (Source). That is the same trap Kamosu is in: the tab bar is where people look, and the item is somewhere else.
- **Most web apps sign out with one tap and no question.** Mealie, Tandoor's app, Immich web, Bitwarden web, Gitea and Nextcloud do not ask. The apps that ask are the mobile ones and the ones that hold local data: Immich phone ("Are you sure you want to sign out?"), Bitwarden Android and extension ("Are you sure you want to log out?"), Standard Notes ("Sign out workspace?"), Apple (a passcode and a second tap) (Source, Docs).
- **Warnings about unsent changes exist in four apps and are missing in the rest.** Todoist, Evernote, Apple and Standard Notes (partly) warn. Immich's phone app, Bitwarden, Nextcloud Android, Mealie, Tandoor and Gitea do not (Source, Docs). Exact wording is in the next section.
- **Immich's phone app shows what silence costs.** Signing out there cancels uploads that are running, switches backup off, and clears the local sync data. It never asks whether anything is waiting to upload (Source).
- **Two apps tell you what stays on the device and let you choose.** Apple offers "Erase this [device]" or "Sign Out But Don't Erase". Evernote's desktop app has a checkbox to keep a copy of your data on the device, checked by default (Docs). Bitwarden's documentation states the cost of logging out plainly: it "completely removes all vault data from your device", so logging back in "can only be done when online" (Docs).
- **A "Lock" beside "Log out" is how a password manager keeps people from signing out when they only want to step away.** Bitwarden's web menu and its Android Account security screen both list "Lock now" above "Log out" (Source).
- **Afterwards, the sign-in page is the norm.** Mealie goes to `/login?direct=1`, Tandoor to its login page, Immich to `/auth/login`, Nextcloud to its login form with `clear=true`, Gitea to the home page (or to the identity provider's end-session address), Bitwarden to its login. None shows a "You have been signed out" message that I found in source (Source, Not found for a message).
- **Platform guidance says almost nothing about where to put sign-out.** Apple's "Managing accounts" page covers account deletion and Apple TV sign-out only. Android's settings guidance says to avoid account management in app settings and to put Settings in a top app bar or menu (Docs).

## Warnings about unsent or unsynced changes

These are the exact words found, with how each was confirmed.

| App | Wording | Where it appears | Mark |
| --- | --- | --- | --- |
| Todoist | "If you have unsynced changes, Todoist shows **Discard unsynced changes?** before logging you out. Select **Cancel** and wait for the sync to finish, or those changes are deleted when you log out." | Help page "Log in and out of Todoist" | Docs |
| Todoist | "Don't close Todoist, shut down your computer, log out, or clear local data before your offline changes have finished syncing, because those changes are deleted. If Todoist shows Discard unsynced changes?, select Cancel and wait until the sync warning at the top of the app disappears." | Help page "Use Todoist while offline" | Docs |
| Evernote (desktop, sign out and keep a backup) | "Unsynced changes - You have unsynced changes. We will attempt to sync them to your other devices the next time you log in." | Quoted by a user in the Evernote forum, 2024 | Third-party |
| Evernote (desktop, sign out and remove data) | "Unsynced changes - You have unsynced changes that will be lost if you exit. Make sure you're online and wait for all notes to sync before exiting." | Same post | Third-party |
| Evernote (Penultimate on iPad, 2014) | "Unsynced Changes. You have changes that have not been synced with Evernote. Any unsynced changes may be lost if you sign out" (the search snippet cuts the sentence here) | Forum post, archived | Third-party |
| Apple (iPhone, iPad, Mac, on "Sign Out But Don't Erase") | "In some cases, you might get a warning that data that hasn't yet synced with iCloud will be removed from the device if you choose Sign Out But Don't Erase." The page gives the steps: check the connection, check the storage quota, check the "Saved to iCloud" status icon is green, then check iCloud.com or another device. The warning's own text is not quoted. | Apple Support, "What happens after you choose Sign Out But Don't Erase" | Docs |
| Apple | "If you recently made changes to content synced with iCloud, wait before signing out, or check iCloud.com or another device to make sure that your changes have synced." | Apple Support, "Sign out of your Apple Account and iCloud on your devices" | Docs |
| Standard Notes | Title: "Sign out workspace?" Body: "This action will remove this workspace and its related data from this device. Your synced data will not be affected." If other workspaces are signed in: "Because you have other workspaces signed in, this sign out may leave logs and other metadata of your session on this device." If file backups are on: "Local backups are enabled for this workspace. Review your backup files manually to decide what to keep." Button: "Sign Out". It does not name unsynced changes; the words "Your synced data will not be affected" imply what is not covered. | `ConfirmSignoutModal.tsx` and `Constants/Strings.ts` in standardnotes/app at `8929036` | Source |
| Bitwarden (browser extension and Android) | Title "Log out", body "Are you sure you want to log out?", buttons "Yes" and "Cancel". On Android the body adds the account's email and environment on two more lines. No warning about pending changes. | See Bitwarden below | Source |
| Bitwarden (setting the timeout action to log out) | "Logging out will remove all access to your vault and requires online authentication after the timeout period. Are you sure you want to use this setting?" (web) and "After the timeout period, you will be logged out. You will need to be connected to the internet to log in and access your vault again. Your settings and PIN saved on this device won't change." (Android) | The timeout setting, not the log-out button | Source |
| Nextcloud Android (removing the account) | Check box "Remove local account", with "Remove account from device and delete all local files". A user issue says it cannot be unticked and does not say which files go. | Account removal dialog | Source (string), Third-party (the complaint) |

No warning of this kind was found for Mealie, Tandoor, Immich (web or phone), Gitea, Nextcloud web, Bitwarden web, or Paprika's log-out button (Not found). Notion, Obsidian Sync and Google Docs were searched for and nothing first-party turned up on a sign-out warning: their pages describe offline edits syncing on reconnect and say nothing at log-out (Not found).

## Mealie

Source: github.com/mealie-recipes/mealie `mealie-next` at `2f5f6d4febd3e7848fd71d1b9fef1a32a7b0e305`, read 2026-10-20.

1. **Where, desktop.** A text button "Logout" with an icon at the right of the top app bar, beside the search field (`frontend/app/components/Layout/LayoutParts/AppHeader.vue:73-83`; the string is `user.logout` = "Logout" in `frontend/app/lang/messages/en-US.json:897`). It is only shown when `loggedIn` (Source).
2. **Where, phone.** The same button, but on the smallest breakpoint (`xs`) it becomes an icon with no text: `:icon="xs"` and `{{ smAndUp ? $t("user.logout") : "" }}` (`AppHeader.vue:74-81`). The icon alone, in the top bar, is all a person sees on a phone (Source). The sidebar holds no sign-out; a search for logout in `components/Layout` finds only the header.
3. **Confirmation.** None. The click calls `logout()` directly (`AppHeader.vue:137-143`) (Source).
4. **Warning.** None. `signOut` posts to `/api/auth/logout`, ignores a failure ("Continue with logout even if API call fails"), clears the token, resets the auth state, clears every cached store, composable cache and Nuxt data "to prevent data leakage between users", then routes (`frontend/app/composables/use-auth-backend.ts:157-179`) (Source).
5. **Afterwards.** Goes to `/login?direct=1` (`AppHeader.vue:139`). The same `signOut` is also called with no address after a password change on the profile page (`pages/user/profile/edit.vue:309`) (Source).

## Tandoor Recipes

Source: github.com/TandoorRecipes/recipes `develop` at `e59f15348c74cd23e2b091bd12581aae364eb91f`, read 2026-10-20.

1. **Where, desktop.** The top app bar ends in an avatar showing the first letter of the display name. Clicking it opens a menu: user info, a divider, then Settings, Help, Admin (superusers), the spaces, plugin entries, and last "Logout" with an arrow-out icon (`vue3/src/apps/tandoor/Tandoor.vue:31-42`; the item itself is at line 39; the string is in `vue3/src/locales/en.json:403`) (Source).
2. **Where, phone.** Below the large breakpoint a bottom navigation bar replaces the drawer, with four buttons: recipes, meal plan, shopping, and a menu button (`Tandoor.vue:92-114`). The menu button opens a bottom sheet with the user info and `getBottomNavigation()`, which lists Settings, Import, Database, Search, Pantry and Books (`vue3/src/composables/useNavigation.ts:73-82`). **There is no Logout in that sheet.** The avatar in the top bar still shows on a phone, so sign-out is reached there (Source). A commented-out line in `useNavigation.ts:97` shows a Logout entry that once pointed at the Django `accounts/logout` page.
3. **Confirmation.** None in the app. `logout()` calls the sign-in library's "delete session" endpoint and, whether it succeeds or fails, marks the user unauthenticated and pushes the login page (`Tandoor.vue:202-212`). The older Django template `cookbook/templates/account/logout.html` does ask: title "Sign Out", body "Are you sure you want to sign out?", one button "Sign Out" (Source). The Django route is the one the new app stopped using.
4. **Warning.** None (Source).
5. **Afterwards.** The login page (`router.push({name: 'LoginPage'})`). The settings file also sets `LOGOUT_REDIRECT_URL = "index"` for the Django route (`recipes/settings.py:250-251`) (Source).

## Immich

Source: github.com/immich-app/immich `main` at `9e20805e21c72b95f8c23f4985c8a20b4c3da12a`, read 2026-10-20.

**Web.**

1. **Where.** The top bar has a round avatar button. It opens a panel (`AccountInfoPanel.svelte`) with the name and email, an account-settings button, and at the foot a ghost button "Sign Out" with a logout icon (`web/src/lib/components/shared-components/navigation-bar/AccountInfoPanel.svelte:94-101`; string `sign_out` = "Sign Out" in `i18n/en.json:2100`). The avatar button is at `NavigationBar.svelte:160-175` (Source).
2. **Confirmation and warning.** None. The button is a link to `/auth/logout`, a page that shows a spinner and calls `authManager.logout()` (`web/src/routes/auth/logout/+page.svelte`). That calls the server's logout, which deletes the session row, and goes to the returned address or the login page (`auth-manager.svelte.ts:93-115`; server side `server/src/services/auth.service.ts:88-101`) (Source).
3. **Afterwards.** The login page, or the identity provider's end-session address when the account signed in through OAuth. When a session is deleted from another device, a `SessionDelete` event sends the browser to the same logout route (`auth-manager.svelte.ts:46-48`) (Source).

**Phone app (Flutter).**

1. **Where.** The top bar has a profile picture button (`mobile/lib/widgets/common/immich_sliver_app_bar.dart:129-136`) that opens a dialog (`ImmichAppBarDialog`). The dialog lists, in order: profile box, storage, server info, app logs, "free up space", settings, **Sign Out**, footer (`mobile/lib/widgets/common/app_bar_dialog/app_bar_dialog.dart:270-300`; "Sign Out" is built at lines 111-143). The picture button carries a badge for backup status (Source).
2. **Confirmation.** Yes. A confirm dialog titled "Sign out" with "Are you sure you want to sign out?" and a button "Yes" (`i18n/en.json:569-570`) (Source).
3. **Warning.** None about pending uploads. What the code does: it deletes the stored PIN code, clears widget credentials, calls the server's logout, then cancels the background upload service and the foreground upload service (`mobile/lib/providers/auth.provider.dart:89-100`). The service clears the local sync data, deletes the stored user and access token, and writes `backupEnabled = false` (`mobile/lib/services/auth.service.dart:93-105`). The doc comment says the server call may fail and "local data cleanup is guaranteed to execute regardless" (Source). So a person with photos waiting to upload, signing out on a phone with no signal, loses the upload run and gets no warning.
4. **Afterwards.** The websocket is disconnected and the login screen replaces the route (`app_bar_dialog.dart:130-136`) (Source).

## Bitwarden and Vaultwarden

Source: github.com/bitwarden/clients `main` at `f2e0b46cd7fa0c6d72aa6f555f3f502eb4b94cc7`; github.com/bitwarden/android `main` at `9bdf4d222ed0916415973418052f0bd089507875`; bitwarden.com/help/vault-timeout. Vaultwarden serves the same web vault, so the web part applies to both. Read 2026-10-20.

**Web vault.**

1. **Where.** The header's account menu (avatar with the account's initials). Items in order: account switching and the account line, "Account settings", "Get help", "Get apps", a divider, "Lock now" (only if the account can lock), then "Log out" with a sign-out icon, last (`apps/web/src/app/layouts/header/account-menu.component.html:56-83`; the handler is at `account-menu.component.ts:51-56`) (Source).
2. **Confirmation and warning.** None in the web menu: the click calls `logoutService.logout(userId, "userInitiated")` at once. Before it finishes, the app uploads queued events, clears keys and the cipher and folder stores, and reloads the process "to clear active secrets in memory" (`apps/web/src/app/app.component.ts:221-262`) (Source).
3. **Afterwards.** The page reloads to the root, which shows the login (Source). The code says no toast is shown: "the process reload will prevent any toasts from being displayed long enough to be read" (`app.component.ts:226-228`).

**Browser extension.**

1. **Where.** The account switcher in the popup, and the "Account security" settings page. Both ask first.
2. **Confirmation.** A simple dialog, title "Log out", body "Are you sure you want to log out?" (`apps/browser/src/auth/popup/account-switching/account-switcher.component.ts:140-152`; `settings/account-security.component.ts:560-575`; strings in `apps/web/src/locales/en/messages.json:2181-2185`) (Source).

**Android app.**

1. **Where.** Settings, then Account security. The screen lists "Lock now" and, under it, "Log out" (`app/src/main/kotlin/com/x8bit/bitwarden/ui/platform/feature/settings/accountsecurity/AccountSecurityScreen.kt:405-424`). It is also on the unlock screen (`VaultUnlockScreen.kt:214`) and in the account switcher (`BitwardenAccountSwitcher.kt:159`) (Source).
2. **Confirmation.** A two-button dialog: title "Log out", body "Are you sure you want to log out?" plus, when known, the account's email and server label on two further lines, buttons "Yes" and "Cancel" (`ui/src/main/kotlin/com/bitwarden/ui/platform/components/account/dialog/BitwardenLogoutConfirmationDialog.kt:17-36`; strings in `ui/src/main/res/values/strings.xml:37-38`) (Source). Showing which account you are about to leave matters because the app holds several.
3. **Warning.** None about pending changes; no string in the Android resources names unsynced or offline edits (Not found). The PIN setup text warns: "Your PIN settings will be reset if you manually log out of the Bitwarden app." (`strings.xml:376`) (Source).
4. **What a log out does, in Bitwarden's words.** "Locking your vault will maintain vault data on the device, so unlocking your vault can be done offline." and "Logging out of your vault completely removes all vault data from your device. Logging back in will require you to re-authenticate your identity, so logging in can only be done when online." (bitwarden.com/help/vault-timeout) (Docs).

## Nextcloud

Source: github.com/nextcloud/server `master` at `83d7ed1f78f2027c927db76ab3427b429dc64339`; github.com/nextcloud/android `master` at `c6551ff3f5363ddc9e7d4dd9d4984d02c2cd8e2b`; github.com/nextcloud/android issue 15794. Read 2026-10-20.

**Web.**

1. **Where.** The account menu at the top right. "Log out" is registered as a settings entry with `'order' => 99999`, so it is always last (`core/Listener/LoadAdditionalEntriesListener.php:44-55`). The same menu is used on a phone browser (Source).
2. **Confirmation and warning.** None. The controller deletes the login token, ends the session, and redirects (`core/Controller/LoginController.php:81-108`) (Source).
3. **Afterwards.** The login form, with `clear=true` in the address. Over HTTPS, for browsers other than Chrome and Android Chrome, the response carries `Clear-Site-Data: "cache", "storage"`, which asks the browser to wipe the site's cached files and local storage (`LoginController.php:89-101`) (Source).

**Android app.**

1. **Where.** "Log out" is a string called `drawer_logout` ("Log out") used by the navigation drawer (`app/src/main/res/menu/partial_drawer_entries.xml`, `DrawerActivity.java`; found by code search, file contents not read) (Source, file names only). Removing an account is another route: the account switcher, "Manage accounts", three dots, "remove account".
2. **Confirmation and warning.** Removing the account shows a check box "Remove local account" with the text "Remove account from device and delete all local files" (`app/src/main/res/values/strings.xml:297-298`). A user filed issue 15794 in October 2025 that the box cannot be left unticked and does not say which files are deleted, and asks whether a photo uploaded from the phone's own folder would go too (Source for the string, Third-party for the complaint). The issue was open when read.

## Gitea

Source: github.com/go-gitea/gitea `main` at `edd0c0e2cee34e5c13d563ceef83fe65c0e1d089`, read 2026-10-20. Forgejo shares this template up to its fork and was not read.

1. **Where.** A drop-down on the avatar at the top right. The menu reads: "Signed in as {name}", a divider, Your profile, Your starred, Subscriptions, Your settings, Help, (for admins, a divider and Admin panel), a divider, then "Sign Out" with a sign-out icon, last (`templates/base/head_navbar.tmpl:100-135`; the sign-out item is lines 128-131; string `sign_out` = "Sign Out" in `options/locale/locale_en-US.json:10`). On a phone the avatar item also shows the user name beside it (`only-mobile` span) (Source).
2. **Confirmation and warning.** None. It is a plain link to `/user/logout` (`routers/web/web.go:759`) handled by `SignOut`, which flushes and destroys the session, deletes the remember-me cookie, and redirects (`routers/web/auth/auth.go:453-485`) (Source).
3. **Afterwards.** The home page, or the identity provider's end-session address when the person signed in through OAuth2, or a configured reverse-proxy logout address (`auth.go:487-500`) (Source). An administrator who was impersonating a user is sent back to the admin area instead (`auth.go:470-479`).

## Paprika

Source: Paprika's support site, `paprikaapp.zendesk.com` article "I synced and my recipes disappeared!" (2019, updated), and paprikaapp.com's password reset page, read 2026-10-20.

1. **Where.** Settings, then Cloud Sync, then a "Log out" button (Docs). The same path is named on the reset-password page: "Please make sure to log out under Paprika's Settings -> Cloud Sync and then log back in with your new password on all of your devices." (Docs).
2. **Confirmation, warning and what stays.** Not stated in the articles read (Not found). The support page says a synced recipe "becomes permanently linked to that account" and that switching to a new, blank account hides the old account's recipes until you log back in.
3. **Why this matters to Kamosu.** Paprika puts the log-out button with the sync account in a settings sub-page and not on its main screens; the support article exists because people use it by accident or to switch accounts. A recipe app that syncs offline treats sign-out as a sync-settings act (Docs).

## Todoist

Source: todoist.com help pages "Log in and out of Todoist" and "Use Todoist while offline" (the second last updated 2026-09-04), read 2026-10-20.

1. **Where, desktop and web.** "Click your avatar in the top-left. Select Log out." (Docs).
2. **Where, phone.** "Tap Browse in the bottom-right. Tap the gear icon in the top-right. Tap Log out." On iPad the Browse step is skipped and the gear is at the top left (Docs). So the phone's tab bar has a "Browse" tab that leads to the gear, then to the log-out row.
3. **Confirmation.** Not for a signed-up account in sync; the dialog below appears only when changes are waiting (Docs).
4. **Warning.** The wording is in the table above. The behaviour: a dialog "Discard unsynced changes?" with Cancel, which sends you back to wait until "the sync warning at the top of the app disappears". Without a pause the changes "are deleted" (Docs). This is the closest match to what Kamosu's offline outbox would need.

## Evernote

Source: Evernote's help article "Remove Evernote data from your Mac or Windows device" (updated 2024-10-09) and a forum thread of 2024-09-11 where a user quotes the dialogs. Read 2026-10-20.

1. **Where.** Not read for this app; the help page says only "When you sign out of the Mac and Windows Evernote apps, you may be asked if you would like to save your Evernote data on your device. This option is checked by default, which allows you to quickly access your data when you sign in without having to re-sync your entire account each time." (Docs).
2. **Two sign-out paths, two warnings.** With "keep a backup", a message promises to sync later on the next sign-in. With "remove data", the message says the changes "will be lost if you exit". See the table (Third-party).
3. **What the same thread teaches.** An expert answering says unsynced notes cannot be listed on the desktop app ("It is assumed (rightfully or not) that everything syncs in the moment it happens") and that on mobile, a green triangle marks notes that still need to sync. A person who got the warning could not see what it referred to (Third-party). A warning with no list of what is waiting leaves the person unable to act.

## Standard Notes

Source: github.com/standardnotes/app `main` at `8929036bd13900789528f6b00ecedecf1b3c7652`, `packages/web/src/javascripts/Components/ConfirmSignoutModal/ConfirmSignoutModal.tsx`, `Constants/Strings.ts:135-138`, `AccountMenu/GeneralAccountMenu.tsx`. Read 2026-10-20.

1. **Where.** The account menu, item "Sign out workspace" (`GeneralAccountMenu.tsx:229-231`) (Source).
2. **Confirmation.** A modal "Sign out workspace?" with the text in the table, and a "Sign Out" button. It adds a line when other workspaces stay signed in, and another with a "View backup files" link when file backups are on (`ConfirmSignoutModal.tsx:19-95`) (Source).
3. **Warning.** About local data, not about unsent changes. The sentence "Your synced data will not be affected." leaves the unsynced part for the reader to infer. A user in the project's forum lost about a fifth of their notes after an app update asked them to sign out; the maintainer answered that "asking you to sign out due to invalid sync state doesn't correctly communicate that you will lose any unsynced changes" and said a fix was pushed (issue 1171, closed; the first quote is from a search snippet, not the page body, so read it as Third-party).

## Apple and Android guidance

- **Apple Human Interface Guidelines, "Managing accounts"** (developer.apple.com, fetched 2026-10-20 through the page's JSON). The page is about creating and deleting accounts. On sign-out it says only, in the Apple TV section: "Avoid displaying a sign-out option when people are signed in at the system level. If your app must include a sign-out option, invoking it needs to prompt people to navigate to Settings > TV Provider to sign out of their account." The page says "No additional considerations for iOS, iPadOS, macOS, or visionOS." Where to put a sign-out row: Not found (Docs).
- **Apple's own Settings, as practice.** iPhone: Settings, tap your name, scroll to the bottom, "Sign Out". Then a choice, "Erase this [Device]" or "Sign Out But Don't Erase", then a passcode or password, then a confirming tap (Apple Support, "Sign out of your Apple Account and iCloud on your devices") (Docs). Mac: System Settings, your name, "Sign Out", with check boxes to "Keep a Copy" of iCloud data first (Docs).
- **Android design guidance, "Settings"** (developer.android.com, last updated 2025-12-10). It says to "Avoid account management" among the things to put in app settings, to put Settings "in the top app bar menu after all other items, except Help & Feedback", and that Settings can be combined with other destinations such as Account, with the rule "Make sure that settings are always accessible, even in a signed out state, if combining sections." It does not mention sign-out by name. Material Design's app-bar pages were searched and none mentions it (Not found) (Docs).

## Answers by question

| App | Desktop | Phone | Asks first | Warns about unsent changes | Afterwards |
| --- | --- | --- | --- | --- | --- |
| Mealie | "Logout" button in top bar | Same button, icon only | No | No | `/login?direct=1` |
| Tandoor | Top-bar avatar menu, last item | Same avatar menu; not in the bottom-tab sheet | No (the old Django page did) | No | Login page |
| Immich web | Avatar panel, "Sign Out" at the foot | Same | No | No | Login, or the provider's end-session address |
| Immich phone | Not applicable | Avatar in top bar, dialog, "Sign Out" | Yes, "Are you sure you want to sign out?" | No; cancels uploads and turns backup off | Login screen |
| Bitwarden web | Account menu, last item, after "Lock now" | Same | No | No | Login |
| Bitwarden extension and Android | Account switcher or Account security | Settings, Account security, "Log out" | Yes, "Are you sure you want to log out?" with the account named on Android | No | Login |
| Nextcloud web | Account menu, last item | Same | No | No | Login form; asks the browser to clear cache and storage |
| Nextcloud Android | Not applicable | Drawer "Log out"; account removal in the switcher | Removal has a "delete all local files" box | No | Not read |
| Gitea | Avatar menu, last item after a divider | Same | No | No | Home page |
| Paprika | Not applicable | Settings, Cloud Sync, "Log out" | Not stated | Not stated | Not stated |
| Todoist | Avatar, top left, "Log out" | Browse, gear, "Log out" | Only if changes are waiting | Yes, "Discard unsynced changes?" | Login |
| Evernote | Not read | Not read | Offers to keep data on the device | Yes, two wordings | Not read |
| Standard Notes | Account menu, "Sign out workspace" | Same | Yes, "Sign out workspace?" | Partly: "Your synced data will not be affected." | Not read |
| Apple (iCloud) | System Settings, your name, "Sign Out" | Settings, your name, bottom row | Yes, with a passcode | Yes, a warning in some cases | Not read |

## What this suggests for Kamosu

These are readings of the evidence, not decisions.

- **On a phone, put "Sign out" where Settings is, as a row of its own near the top, not inside the Sessions list.** Todoist (gear, then "Log out"), Apple (name, then "Sign Out"), Paprika (Settings, then "Log out") and Bitwarden Android (Settings, then "Log out") all keep it one step from the settings entry, as a plain row. Kamosu already has a gear icon in the phone header, so Settings is one tap away. The Tandoor case argues against hiding it in a menu the tab bar does not lead to.
- **On a wide screen, the sidebar foot beside Settings is the nearest equivalent to an account menu.** Gitea, Nextcloud and Bitwarden all put it last, after a divider, below the settings link. Kamosu has no avatar, so the foot of the sidebar plays that part. The same row on Settings would serve both layouts, as it does in Immich's web panel.
- **Ask first on a phone, but keep the question small.** Immich's phone app and Bitwarden both ask "Are you sure?" on a phone only, where a mistaken tap is easier. Bitwarden's Android dialog names the account. Kamosu has one account per sign-in, so a plain question with the person's name would match.
- **Warn only when something would be lost.** Todoist's dialog shows up only when changes are waiting, and it says what happens and offers "Cancel". That fits Kamosu's offline outbox: if writes are queued, the dialog says they have not reached the server and would be deleted, and offers to stay and wait. No other recipe app in this survey does it, and the apps that skip it (Immich phone, Bitwarden, Nextcloud) are the ones with open issues about lost data or unclear deletion.
- **A warning should say how many and what.** The Evernote thread shows the cost of a warning with no list: the person cannot tell what is waiting or whether it will clear. Kamosu's outbox can state a count and the kinds of change (a shopping list tick, a cooked entry).
- **Say what stays on the device.** Apple, Evernote and Bitwarden all state what is kept or removed. Kamosu's service worker caches the interface and the phone cache holds reads. A line saying signing out also clears what this device saved for offline use would close the same gap Nextcloud Android's check box leaves open.
- **Do not copy the "Lock" button.** Bitwarden needs one because a locked vault opens offline with a PIN. Kamosu has no local unlock, so there is nothing to put beside "Sign out".
- **Afterwards, the sign-in page with a short line saying the person signed out.** Every app read goes to a login screen. None was found to say "You have been signed out" in its code, which is a gap a household app can fill, since a person who lands on a login page right after tapping a button may think the app lost them.
- **Keep the Sessions list for other devices.** Immich, Bitwarden and Home Assistant keep a device list for revoking other sessions and give "sign out this one" its own entry elsewhere. The two jobs differ: "I am done here" and "a device I lost".

## Fallback calls made

None. Every search and fetch went through DonSeTch (MCP `web_fetch` with `deadline_ms: 45000`, and `donsetch search` over Bash). Source files came from shallow clones and `raw.githubusercontent.com`, and `gh api` code search was used for file names. Two fetches returned a different page than asked for (an Evernote "Sign out" help address returned an Enterprise billing article, and a guessed Todoist address returned the help-centre home page); both were retried with an address taken from search results, and no paid fallback was used.
