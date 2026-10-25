# An Operator looks after every Cookbook

An Operator may read, change, create and delete recipes in any Cookbook on the instance, and their Tags and Related Recipes with them, through the Operations every Person uses and under the Operator's own Hand. They do it one call at a time, by saying so: everywhere else an Operator is a Person like any other. Settled on 2026-10-15 on [#302](https://github.com/BatterWorks/Kamosu/issues/302). This supersedes the line of [ADR 0007](./0007-a-recipe-is-held-by-a-kitchen-not-a-person.md) that said the Operator administers and does not read.

_(Amended by [#369](https://github.com/BatterWorks/Kamosu/issues/369), 2026-10-19: an Operator acts as one through a login Session or an **Operator key**, and through no other Access Key. Until then every Access Key an Operator minted carried the Operator's powers, so every agent an Operator handed a key to could rewrite the House Notes that every later Job reads. An Operator key is minted as one on purpose, only from a Session of an Operator, and stops acting as Operator for good when its Person stops being one. Keys minted before the change became Operator keys, so nothing an Operator had set up stopped working. A read-only key fetches no Backup, Operator key or not, because a Backup holds every password and Secret hash. Agents keep their parity with people: an Operator key does everything an Operator's key did before, and nothing became browser-only.)_

It started with a person whose link import landed as an empty recipe ([#301](https://github.com/BatterWorks/Kamosu/issues/301)). They pasted the same recipe by hand and were left with two copies, one of them empty. The Operator wanted to tidy that up: delete both and put one checked recipe in their Cookbook. Kamosu had no way to do it. Every recipe Operation was held to the caller's own Cookbook and the Kitchens they cook in, and no Operator Operation reached a recipe. What was left was a Share Link the other person had to open and keep, or writing to the database by hand, which skips everything a save does.

## What was decided

- **An Operator is a Person like any other until a call says otherwise.** Their shelf, search and lists show what they see. A Cookbook they cannot see is not there, and changing a Kitchen-mate's recipe starts a Copy, as it does for anybody.
- **A call that sends `as_operator: true` acts as the Operator.** `get_recipe`, `get_thread`, `edit_recipe`, `save_recipe_version`, `delete_recipe`, `set_recipe_tag` and `set_related_recipe` then reach a recipe in any Cookbook. `create_recipe`, `create_tag`, `list_tags` and the Imports (`import`, `import_crouton`, `import_web_link`, `import_by_agent` and `import_bundle`) take a `cookbook_id` to work in somebody else's Cookbook. Left out, they work in the Operator's own. `list_cookbooks`, an Operator Operation, names every Cookbook on the instance, who writes it and the recipes in it.
- **One switch, in the Core.** `Core::execute` accepts `as_operator` only from an Operator acting themselves, through a login Session or an Operator key that may write, and refuses it from anybody else. Only the Operations above declare it, so any other refuses it by its shape. `looks_after` in `src/core/cookbooks.rs` is the one rule the recipe checks ask. No Door checks anything.
- **A change made as the Operator is made where the recipe is.** It is never a Copy. The Version names the Operator as its Hand, as every Version names who wrote it, and the History shows it. Nothing else on the recipe marks it.
- **Recipes and their Tags, and nothing else.** Attempts, shopping lists and who writes a Cookbook stay their owners'. So do the rest of a recipe's acts that make something of the Operator's own, such as starting a variation or a Translation.
- **Nothing else gains anything.** A read-only Access Key, a Runner Key and a Job Key cannot act as the Operator. A Job Key reaches its one recipe and no other, as [ADR 0048](./0048-an-agent-beside-kamosu-reads-what-kamosu-cannot.md) says. Where an Operator acting as one put that recipe in somebody else's Cookbook, through an Import, the Job Key reaches it there: it reads it with `get_recipe` and `get_thread`, writes it, corrects its Readings and reads that Cookbook's Tags, and its create lands there. Only while the Person who asked is still an Operator. Comparing it with another Branch (`branch_point`, `divergence`, `changed_since`) still keeps to what the asker sees. A Job Key never names a Cookbook itself.
- **No screen for it.** An Operator acts as one through the Operations, an agent at the MCP door first. The screens never send `as_operator`, so they behave for an Operator as they do for everybody. A screen for it is a ticket of its own if one is wanted.
- **The people on an instance hear about it in the changelog.** Nothing in the app announces it.

## Why

- **The disk was always the real boundary.** ADR 0007 called "administers and does not read" a courtesy and not a wall, because whoever holds the disk can reset any password and read the database. Keeping the courtesy pushed an Operator who needed to help to the disk, where nothing records who changed what. A supported path is honest about the same power and leaves a Hand on every change.
- **One rule in the one central check is how the closest prior art does it.** KitchenOwl's server admin skips the household check in the one place every household route is authorised. Mealie lets an admin edit and delete inside their own group. Tandoor's superuser edits raw records through Django's admin panel, which is the "go to the database" route this replaces. The comparison is on #302.
- **The Hand is enough to tell the owner.** Every Version already names who wrote it, and the History shows it. A separate "changed by an Operator" mark would need a seen-or-not state Kamosu keeps for nothing else.
- **Most of what an Operator does, they do as a cook.** Being able to act on any recipe does not mean every act is an administrator's. Asking for it per call keeps an Operator's shelf, their Copies of a Kitchen-mate's recipe and their screens what they were, and makes every reach into somebody else's Cookbook something somebody meant.

## Considered options

- **View as user.** Signing in as another Person. Rejected: every write would carry the other Person's Hand, which is the opposite of saying who changed what.
- **Reach always on for an Operator.** The first build of this. An Operator's every edit of a Kitchen-mate's recipe then landed on theirs instead of starting a Copy, and the screens offered acts on other people's recipes. Rejected for the switch.
- **A Cookbook switcher on the shelf or in Settings.** Deferred, not rejected. The tidy-up that started this runs through an agent, and the Operations come first.
- **Reach into Attempts, shopping lists and Cookbook membership too.** Rejected. They are personal, and nothing in them needs tidying by somebody else.
- **An Operator's reach stopping at a Kitchen.** Mealie stops an admin at their own group. On an instance of friends with separate Cookbooks, the whole instance is the useful answer.

## Consequences

- **Everyone on an instance trusts its Operators with their recipes**, as they always trusted them with the disk. The changelog says so in words for the people on an instance as much as for the Operator.
- **An Operator's Operator key may act as the Operator**, and their other Access Keys reach what any Person reaches (#369). A read-only one may not.
- **`GLOSSARY.md`'s Operator entry lists looking after Cookbooks among the Operator's powers.**
