# The Security Record and its log lines

Kamosu keeps a record of what happens at its doors: signing in, wrong
passwords, keys, passkeys and changes to the instance (#371). A Person reads
their own in **Settings › Your security record**, and an Operator reads the
instance's in **Settings › Instance security record**. An agent reads them through the
`list_security_record` and `list_instance_security_record` Operations.
Events are kept 90 days.

Each event is also written as **one line on standard output**. Kamosu sends
nothing anywhere. What happens to the lines is up to whoever runs it: read
them with `docker logs`, ship them to a log store, or have a watcher send you
a message when one appears.

## The line

```
kamosu-security event=signed_in at=2026-10-20T09:15:02.123Z person=p_3f2a9c0d1e4b5a6f subject=- credential=session credential_name="Safari on iPhone" count=1
```

Every line starts with `kamosu-security ` and has the same seven fields,
always in this order:

| Field | What it holds |
| --- | --- |
| `event` | What happened, one of the names below. |
| `at` | When, in UTC, as `YYYY-MM-DDTHH:MM:SS.mmmZ`. |
| `person` | The id of the Person whose account it happened to, or of the Person who changed the instance. `-` when there is none, as for a crowd at the sign-in. |
| `subject` | The id of the Person acted on: made an Operator, stood down, disabled or deleted. `-` otherwise. |
| `credential` | The kind of Credential the event names: `session`, `access_key`, `operator_key`, `job_key` or `passkey`. `-` when it names none. |
| `credential_name` | That Credential's name, as a JSON string, so a quote or a line break in a name cannot split the line or fake a field. `-` when there is none. |
| `count` | How many times so far. Above 1 only on `sign_in_failed` and `sign_in_crowd`. |

A line never holds an IP address, since Kamosu does not trust what a proxy
says about the visitor (ADR 0033). It never holds a Secret, a password, or a
name somebody typed at the sign-in. A Person is named by id, never by name:
`list_accounts` and Settings › People say who an id is.

## The events

A Person's own record:

| Event | When |
| --- | --- |
| `signed_in` | A Session was made: by password, passkey, Invite, recovery link or at setup. `credential_name` is the new Session's. A recovery link's says how many Sessions, Access Keys and Runner Keys it ended. |
| `sign_in_failed` | A wrong password at this Person's name, at the sign-in or where their own password is asked again. One run of them is one row, ended by the Person's next sign-in of any kind or an hour with none; each wrong password writes a line with the run's `count` so far. A name nobody holds writes nothing. |
| `passkey_added`, `passkey_removed` | |
| `password_changed` | `credential_name` is the Session that changed it. |
| `access_key_minted`, `access_key_ended` | `credential` is `operator_key` for an Operator key. |
| `session_ended` | `credential_name` is the Session ended. |

Every Operator's record:

| Event | When |
| --- | --- |
| `house_notes_changed`, `public_address_changed` | |
| `operator_raised`, `operator_stood_down` | `subject` is the Person. A Person who joins through an Operator Invite is noted as raised, with no `person`. |
| `account_disabled`, `account_deleted` | `subject` is the Person. |
| `operator_key_minted` | Recorded beside its Person's `access_key_minted`, which is the one line written. The row names the Session that minted it. |
| `sign_in_crowd` | Sign-ins refused because too many passwords were waiting to be checked at once: most likely a crowd of guesses at made-up names. One row while it lasts, ended by an hour with none. A line is written when it starts, then once a minute with the count so far while refusals keep coming, the last within a minute of the final refusal. A restart loses the refusals of its last minute. |
| `runner_key_minted`, `runner_key_revoked` | A Runner Key a recovery link ended is `runner_key_revoked` by the recovered Person, through the Session the link opened (#378). |
| `backup_fetched` | A Backup's archive was downloaded. It holds every password hash on the instance. |

For the Operators' events, `person` and `credential` say who made the change
and through what. A change made through an Operator key that nobody remembers
making may be an agent tricked by text it read: end that key.

## Matching the lines

The fixed start makes the lines easy to pick out of everything else Kamosu
logs:

```sh
docker logs -f kamosu 2>&1 | grep --line-buffered '^kamosu-security '
```

Some lines worth a message the moment they appear:

```sh
grep -E '^kamosu-security event=(sign_in_crowd|house_notes_changed|operator_raised|backup_fetched) '
grep -E '^kamosu-security event=sign_in_failed .* count=([5-9]|[1-9][0-9]+)$'
```

Since a line carries no address, a tool that bans addresses, such as
fail2ban, has nothing to ban from it. Ban at the reverse proxy in front of
Kamosu instead, which does see the address, on the answers it passes back:
a failed sign-in is a `401` to `POST /auth/login`.
