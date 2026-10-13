//! Runners: the Runner Keys an Operator mints, a Runner asking for work, and
//! whether this instance has a Runner at all (ADR 0048).

use super::*;

/// The answer to minting a Runner Key: the raw Secret, shown once and never
/// stored, alongside what the Key is known by afterwards.
pub struct RunnerKey {
    pub id: String,
    pub secret: String,
    pub name: String,
}

impl Core {
    /// Mint a Runner Key. It belongs to the instance and to no Person, so
    /// nothing here says who asked: that an Operator did is the Catalogue's
    /// declaration, checked in `execute`.
    pub fn mint_runner_key(&self, key_name: &str) -> Result<RunnerKey, OpError> {
        let key_name = required_text(key_name, "name")?.to_string();
        let secret = generate_secret();
        let hash = hash_secret(&secret);
        let id = format!("rk_{}", hex::encode(random_bytes(8)));
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT INTO runner_keys (id, secret_hash, name) VALUES (?1, ?2, ?3)",
                params![id, hash, key_name],
            )
            .map_err(|e| OpError::internal(format!("cannot mint Runner Key: {e}")))
        })?;
        Ok(RunnerKey {
            id,
            secret,
            name: key_name,
        })
    }

    /// Every Runner Key by its name and last use, newest first. The Secret
    /// appears nowhere in it.
    pub fn runner_keys(&self) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT id, name, created_at, last_used_at, revoked FROM runner_keys
                      ORDER BY created_at DESC, id",
                )
                .map_err(|e| OpError::internal(format!("cannot list Runner Keys: {e}")))?;
            statement
                .query_map([], |row| {
                    Ok(json!({
                        "id": row.get::<_, String>(0)?,
                        "name": row.get::<_, String>(1)?,
                        "created_at": row.get::<_, String>(2)?,
                        "last_used_at": row.get::<_, Option<String>>(3)?,
                        "revoked": row.get::<_, i64>(4)? != 0,
                    }))
                })
                .map_err(|e| OpError::internal(format!("cannot read Runner Keys: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| OpError::internal(format!("cannot read Runner Keys: {e}")))
        })
    }

    pub fn revoke_runner_key(&self, key_id: &str) -> Result<(), OpError> {
        let changed = self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE runner_keys SET revoked = 1 WHERE id = ?1 AND revoked = 0",
                params![key_id],
            )
            .map_err(|e| OpError::internal(format!("cannot revoke Runner Key: {e}")))
        })?;
        if changed == 0 {
            return Err(OpError::not_found("no live Runner Key has that id"));
        }
        Ok(())
    }

    /// The Runner Key a Secret's hash names, if it names a live one. Reads
    /// only: `present` is what records the use.
    pub(super) fn lookup_runner_key(&self, hash: &str) -> Result<Option<String>, OpError> {
        self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT id FROM runner_keys WHERE secret_hash = ?1 AND revoked = 0",
                params![hash],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot resolve Credential: {e}")))
        })
    }

    /// A Runner asking for a Job to do. Nothing waits for a Runner yet, so the
    /// answer is always that there is none (#262 hands one over).
    ///
    /// The Runner is Kamosu's own program started in another mode, so one of
    /// another version is an install half upgraded. It is refused before the
    /// asking is recorded: a Runner that cannot work has not asked.
    pub fn ask_for_work(
        &self,
        runner: &RunnerCaller,
        runner_version: &str,
    ) -> Result<Value, OpError> {
        let kamosu_version = env!("CARGO_PKG_VERSION");
        if runner_version != kamosu_version {
            return Err(OpError::bad_request(format!(
                "this Runner is version {runner_version} and this Kamosu is version \
                 {kamosu_version}. Run the Runner of the same version as the Kamosu it asks"
            )));
        }
        self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE runner_keys SET last_asked_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                  WHERE id = ?1",
                params![runner.runner_key_id],
            )
            .map_err(|e| OpError::internal(format!("cannot record the asking: {e}")))
        })?;
        Ok(json!({ "job": null }))
    }

    /// Whether this instance has a Runner, which is to say a Runner Key that
    /// is not revoked, and when one last asked for work. The time is read
    /// across every Key, revoked ones included, so replacing a Key does not
    /// erase when the Runner was last heard from.
    pub fn runner_status(&self) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT COALESCE(SUM(revoked = 0), 0) > 0, MAX(last_asked_at) FROM runner_keys",
                [],
                |row| {
                    Ok(json!({
                        "has_runner": row.get::<_, bool>(0)?,
                        "last_asked_at": row.get::<_, Option<String>>(1)?,
                    }))
                },
            )
            .map_err(|e| OpError::internal(format!("cannot read the Runner's state: {e}")))
        })
    }
}
