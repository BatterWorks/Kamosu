# Changelog

## Unreleased

## v0.3.0 - 2026-10-18

### Added

- An Operator signed in directly can mint a Runner Key, for a Runner: a program beside Kamosu that will have an agent read what Kamosu cannot. Kamosu shows its Secret once. It acts as nobody, and can ask for work, keep hold of the Job it is working, report on it and nothing else. Both doors gain `mint_runner_key`, `list_runner_keys` and `revoke_runner_key` for an Operator, `runner_status`, which tells anyone signed in whether the instance has a Runner and when one was last heard from, asking for work or keeping hold of a Job, and `ask_for_work`, which answers a Runner Key alone. [#260] [#266] [#277]
- The Operator screen's Instance tab has a Runner section. It says what a Runner is and how long ago one last asked for work, or that none ever has, and lists each Runner Key by name and last use. An Operator makes a Runner Key there and sees its Secret once, and revokes one after a question saying what it stops. Until now this took `mint_runner_key` at either door. [#275]
- While a Runner Key stands, a link Kamosu cannot read is kept for an agent: a page with no recipe data, or one that refused Kamosu's fetch, lands as a bare recipe and a Job waits on it. Before, the first landed bare with no Job and the second landed nothing. A Runner takes the oldest waiting Job with `ask_for_work`, which names the link and the recipe, and the Job reads as running while that Runner holds it. The hold lasts two minutes, and the Runner keeps it by calling `renew_hold` in time, every 40 seconds as the answer tells it. A Job whose Runner went quiet reads as waiting again, and the next Runner that asks is handed it. The Runner that lost it is refused when it renews or reports, and told its hold is gone. The Job is read with `get_job`, or through the tasks extension at the MCP door, by the Person who asked and by the Co-authors of the Cookbook its recipe is in. Anyone else is answered as if the Job were not there, someone who sees the recipe through a Kitchen included. Kamosu's own Jobs read as before. Both doors gain `get_recipe_job`, which names the Job that waits or is being worked on a recipe to those same people, and answers `null` once it has ended and to anyone else who sees the recipe. The Person who asked can cancel the Job with `cancel_job`, or through the tasks extension at the MCP door, while it waits and while it is being worked, which Kamosu's own Jobs allow only while they wait. Nobody else can, a Co-author who reads it included. A cancelled Job reads as cancelled at once and keeps what the agent had saved. The Runner holding it is refused its next `renew_hold`, and told the Job was cancelled. A Runner holds one Job at a time. `ask_for_work` takes a mark in `ask`, which the Runner makes up for each new ask and sends again when a reply never arrived, so asking is safe to repeat. The Runner says how the Job ended with `report_on_job`: completed, failed, or could not be read, with the agent's own report. A Job reported failed waits to be tried again with what the agent had saved, as one whose Runner went quiet does. The third time a Job is taken and does not finish, it ends as failed and says it was tried three times. A Job the agent says could not be read ends at once, with the agent's reason. A waiting Job is handed to nobody while the Person who asked is disabled, and ends as failed if they are deleted. Revoking a Runner Key sends the Job it holds back to waiting. The agent writes the recipe meanwhile, with the Job Key below, and however many times it saves, the Job leaves one new Version after the bare one. The Import Report and `list_imports` name the Job in a new field, `agent_jobs`, with its state now. Once the Job has completed, the Report says an agent made the recipe, what the agent left undone, what it was unsure of, likely duplicates and the Foods left unused. A Job that failed is listed under what could not be read, with the reason, and its bare recipe is still listed as arrived. The same link handed over again while its Job waits or is being worked answers that Job. Once an agent has read it, the link matches the recipe the agent wrote: Kamosu's own bare reading of the page is not offered as a change, and no Job is made. After a Job that failed, the link makes a new Job on the same recipe. [#261] [#262] [#266] [#267] [#268] [#271]
- `ask_for_work` hands the Runner a Job Key with the Job, in a new field, `job_key`. It is a new kind of Credential, for the agent that does the Job. It acts as the Person who asked for the Job. It reads recipes with the Reading of each Ingredient Line, their Threads, search, Tags and Foods, as that Person sees them: `get_recipe`, `get_thread`, `branch_point`, `divergence`, `changed_since`, `search_recipes`, `list_tags`, `list_foods` and `get_food`. It writes the one recipe the Job is for, under that Person's Hand, and nothing on the recipe says an agent wrote it: `save_recipe_version` and `edit_recipe` write it, `create_recipe` creates it where the Job has none yet (below), `set_reading` corrects a Reading, and `set_recipe_tag` puts on a Tag the Cookbook already has. However many times the agent saves, its Job leaves one new Version in the recipe's History. Its saves fold into that one however far apart and never into the Version that was there before, so the bare recipe an Import landed stays as a Version of its own. A person's later save starts a new Version, even within the hour. If someone saves the recipe between two of the agent's saves, or a Copy, a variation, a Translation or a Bundle sent from the recipe holds the Version the agent is still shaping, the agent's next save is refused and the Job fails with that reason. Its save names no `translates_version_id`, and it corrects no Reading while another recipe reads word for word the same, since the two share their Readings. Aimed at any other recipe, each of those answers as if the recipe were not there. `upload_photograph` and `POST /api/photographs` store a Photograph for it. Every other Operation, and every other address outside the Operations, answers a Job Key as if it were not there, and the MCP door lists it those fifteen tools alone. A Job never starts a Copy: where a change by the Person who asked would start one in their own Cookbook, as it does once they have left the Cookbook that holds the recipe, the write is refused and the Job fails with that reason. A Job that fails keeps what it had saved. Kamosu shows a Job Key once and keeps only its hash. It stops working when the Job ends, a cancel included, and when the Person who asked is disabled or deleted. Each Runner that takes the Job is handed a Job Key of its own. The one from a hold that lapsed goes on working until the Job ends, and a save with it shapes the same one Version. Revoking a Runner Key ends the Job Keys it was handed. A Runner that repeats an ask gets a new Job Key in place of the one in the reply it lost. [#263] [#264] [#265] [#266] [#267]
- While a Runner Key stands, both doors take pasted text, a PDF or a picture for an agent to read, with `import_by_agent`. A scanned PDF is taken too. Until now a scan or a picture was refused, and pasted text or a PDF could only be split into a draft to sort by hand, which `read_pasted_recipe` and `read_recipe_pdf` still do. It answers the id of a Job that waits for a Runner, and no recipe exists until the agent creates one with `create_recipe`, in the asker's own Cookbook and under their Hand. A Job Key creates one recipe for its Job, and only for a Job that has none. A file that is none of the three is refused in words, and so is the whole Import on an instance with no Runner. What was handed over is kept as the Job's Original, beside the database and outside it: no Backup holds it, and no Bundle or Share Link carries it. A picture is remade first, which takes out where it was taken. The Runner is told where to fetch the Original, at `GET /api/originals/<job_id>`, and only that Job's Job Key fetches it. An Original is deleted 30 days after its Job ended, whether it completed, failed or was cancelled, and never while the Job waits or is being worked. The Job still reads afterwards, and its Import Report says so in a new field, `original_gone`, and still names what kind of thing was handed over. The Job's result is its Import Report, which lists the recipe once the agent has created it, says an agent made it, and names the reason when it could not be read. `list_imports` lists the arrival under `pasted_text`, `pdf` or `picture`. The same PDF or picture handed over again is known by its contents. While the first Job on it waits or is being worked, the answer is that Job, and once the agent has created the recipe, the answer is that recipe, as `branch_id` and `title`, with no Job. A file whose Job made no recipe goes to a new Job, and one whose Job made a recipe and then failed answers that recipe. After the Import is forgotten the file arrives as new. Pasted text is never matched. `ask_for_work` gains a field, `original`. On such a Job its `link` is null, and so is its `branch_id` until the agent has created the recipe. [#269] [#270] [#271]
- While a Runner Key stands, a Co-author can ask for an agent to read a recipe again, with `read_recipe_again`. A recipe an agent read from a PDF, a picture or pasted text is read again from what Kamosu kept of it, even where its Source also has a link. Once Kamosu has deleted that, 30 days after its Job, the ask says it needs the file, and takes it handed over with the ask. Any other recipe is read from the link in its Source, whichever way it arrived: a link, another app's export or typed by hand. It is the one way a page Kamosu read itself goes to an agent. The Job waits for a Runner like any agent's Job, and asking while one waits or is being worked on the recipe answers that one. A recipe is read again once: after a reading that finished, or one stopped while a Runner held it, the ask is refused with reason `already_read_again`. One that failed, or was stopped while it waited, does not count, and a variation has a reading of its own. What the agent saves becomes one new Version under the Hand of whoever asked, so the earlier one stays in the History. The Job's result is an Import Report that lists the recipe as an arrival and says what the agent left undone or was unsure of, and `list_imports` lists it under `read_again`. Someone who sees the recipe through a Kitchen and does not write its Cookbook is refused. So is a recipe received in a Bundle and never changed, since a change by the Person asking would start a Copy. An instance with no Runner is refused in words, and so is a recipe that came from no file and whose Source holds no http or https link on the internet. `can_read_recipe_again` says beforehand whether a recipe can be read again, from its link or from what was handed over, and if not, why. Kamosu remembers that a recipe came from a file through its Import, so a recipe with no link has nothing to read again once that Import is forgotten. [#272] [#273] [#279]
- While a Runner Key stands, a PDF, a picture or pasted text that an agent failed on before it made a recipe can be handed to an agent again with `read_job_again`, from what Kamosu kept of it, by the Person who handed it over. Asking again while that Job waits answers the same Job. Once Kamosu has deleted what it kept, 30 days after the Job, the file is handed over again. [#273]
- The Import Report shows what an agent did. Its title names what was brought in: a link, a picture, a PDF, pasted text, or a recipe read again. Under Needs you it flags each recipe the agent thinks may be the same dish, with a link to it, and lists what the agent left undone, what it was unsure of and the Foods its corrections left unused, three of each before a tap shows the rest. Under Arrived, the agent's recipe reads as written by Kamosu's AI agent, in place of a bare name and link. A Job the agent failed on is listed under Couldn't be read, quoting the agent where it said there was no recipe, and its bare recipe is still listed as arrived. A picture, PDF or pasted text the agent failed on now shows that Report, where it used to show only that the import stopped. The Report's `agent_jobs` gains `could_not_be_read`, true where the reason is the agent's own words. [#276]
- While an agent's Job waits or is being read, the recipe's page, its Import Report and its row in Brought in say so, and follow it without being reloaded. On the recipe's page and the Report a line of four steps, Sent, Waiting, Reading and Ready, sits above "Cook this", which stays, and nothing on the recipe is greyed out. Its Co-authors see it too. The cook who asked can stop Kamosu's AI agent there, after a question, and what it already wrote stays. When the Job ends, the page shows the recipe as it now is. When Kamosu's AI agent has not answered for five minutes, a waiting Job says so, and points the Operator at the Runner Keys. A row in Brought in carries a Waiting or Reading tag. A PDF, a picture or pasted text handed to an agent has a Report titled for it from the start, where it used to say "Your Crouton library" and "Waiting to start", and Brought in names those sources Pasted text, PDFs, Pictures and Read again. `get_recipe_job` says in a new field, `yours`, whether you asked for the Job. [#277]
- On an instance with a Runner, the + beside the Recipes search box offers a photo of a page: a cookbook page or a recipe card, taken with the phone's camera, chosen from its photos, or a file at a computer. It sits with a PDF and pasted text under "Kamosu's AI agent makes the recipe · a few minutes", and all three go to the agent in place of the draft sorted by hand, a scanned PDF included, where it used to be refused. A photo or a PDF is shown once before it is sent, and Send opens its Import, which says whether it waits or is being read. A photo or a PDF the agent already read opens the recipe it made, saying so. A photo or a PDF dropped on Recipes does the same. With no Runner the + is as it was. [#278] [#349]
- On an instance with a Runner, the + beside the Recipes search box offers a video: the link to a cooking video on YouTube, Instagram or TikTok, which goes straight to Kamosu's AI agent. Kamosu does not read the page first, so no recipe named after the video's page lands before the agent's recipe. Send opens its Import, as it does for a photo. A video's link pasted in From a link works as before. Both doors take the link in `import_by_agent` as `link`, refused where it is not an http or https address on the internet, and `list_imports` lists the arrival under `web`. [#349]
- On an instance with a Runner, a recipe's page offers a Co-author to have Kamosu's AI agent read it again, in a panel above the recipe's other buttons. It says what it would read, the recipe's site or the photo, PDF or text it came from, and asks once, saying it takes a few minutes and costs whoever runs the AI agent. Where Kamosu has deleted that photo, PDF or text, it asks for it again. Nobody else is offered it, and a recipe already read again is not offered it again. The page then follows the reading, and when it ends a line leads to its Import Report. On the Import Report of a photo, PDF or pasted text the AI agent failed on, the cook who sent it can send the same file again while Kamosu keeps it, without finding it. [#279]
- Kamosu runs as a Runner with `kamosu runner --kamosu <address> -- <agent command>`, beside an agent of the Operator's choosing. It reads its Runner Key from `KAMOSU_RUNNER_KEY`, asks that one address for work every 10 seconds, and follows no redirect. An address holding a name or a password is refused, since the agent is handed it. It listens on no port and opens no database. For each Job it makes a fresh folder holding `job.json`, with the Job, its Job Key and Kamosu's MCP door, and starts the agent command there. The Runner Key is in no file of that folder and not in the command's environment, and the Runner blanks it in its own, where another program of the same user could read it. It keeps its hold while the command runs, and reports the Job with the agent's own `report.json` when the command ends. A command that ends in an error or leaves no report fails the Job and says why. One still running after 30 minutes is stopped with everything it started, and so is one whose Job is cancelled. The folder is removed afterwards. With Kamosu out of reach it says so and asks again, more slowly each time, up to every two minutes. A Runner Key that was revoked, and a Runner of another version than its Kamosu, stop it with a sentence saying so, and `ask_for_work` names the second refusal with reason `another_version`. Given `--agent-socket <path>` in place of a command, it hands each Job to an agent in a container of its own, listening on that Unix socket, so the Runner Key is in another container altogether. It asks Kamosu for work only while an agent waits there, and hangs up on the agent to stop it. Kamosu's address and the agent's socket can come from `KAMOSU_RUNNER_INSTANCE` and `KAMOSU_RUNNER_AGENT_SOCKET` in place of the two flags, so a Runner beside an agent's container is set up from its environment alone, and a flag given as well wins. `docs/runner.md` says how to set one up beside an agent. [#274] [#317] [#348]
- Every release publishes the agent that works beside a Runner as `battermanz/kamosu-agent`, under the same version tags as Kamosu's image, and a release that did not change it gives its new tags to the image already published. Its instructions can be replaced by mounting a file over them. It is opencode, with yt-dlp, ffmpeg and poppler for a video, a picture or a scanned PDF, and the instructions it follows to read one recipe alone. Nothing in it names a provider or a model: the Operator gives both when the container starts, as `KAMOSU_AGENT_MODEL`, with the key for whichever provider that is in `KAMOSU_AGENT_API_KEY` or the provider's own variable, and how hard it thinks in `KAMOSU_AGENT_EFFORT`. `KAMOSU_AGENT_BASE_URL` reaches a service opencode does not know that offers an OpenAI-compatible API, and sends that model a photo or a scanned PDF as pictures. It takes one Job from the Runner through a socket, in a read-only container that starts afresh for each, runs as its own user and never as root, and never holds the Runner Key. yt-dlp takes its nightly build each time the container starts. After each Job its log says how long the agent worked and what the Job cost, as opencode counts it, and the largest single request it sent the model, with how many went past 100,000 tokens, where some providers charge more. It reads a recipe's own words, then the cook's written recipe, then the video's frames and every caption over them, and gives a recipe from a video a photograph of each step and of the finished dish. It writes one recipe under that recipe's own name, never a page's headline, and from a source holding several it takes the first and names the others in its report. It checks each ingredient line's Food against the Foods the library already has, reads an old card's "c" and "#" as a cup and a pound, and names the Foods its corrections leave unused. It copies a number the source gets plainly wrong and says so rather than correcting it, keeps a cook's correction when it reads a recipe again, reads a page that refuses it from the Wayback Machine's copy of that same page and never from another cook's recipe, and ignores orders a page gives an agent. [#281] [#315] [#317] [#318] [#324] [#325] [#345]
- An Operator can add House Notes to the instructions Kamosu's AI agent follows, such as "grams, never cups", without a terminal. In the Runner section of the Operator screen, a row shows the first line and who saved them last and when, and opens a sheet to write them, up to 4,000 characters. Every Job carries them as they stand when a Runner takes it, in a new `ask_for_work` field, `house_notes`, and the published agent puts them after its own instructions and follows them where the two disagree. Its own instructions still change with each release, under the notes. An agent of your own follows them only if it reads that field. Both doors gain `get_house_notes` and `set_house_notes` for an Operator. Nobody else reads them, and a Job Key reaches neither. [#344]
- An Operator can open, change, create and delete the recipes in anyone's Cookbook, and their Tags and Related Recipes, to help tidy a library. Until now an Operator could not reach anybody else's recipes at all. It is asked for one call at a time, with `as_operator`, at either door: without it an Operator is a cook like any other, and the screens never ask for it. What an Operator changes this way shows under their own name in the recipe's History, and is never a copy of their own. Cooking diaries and shopping lists stay their owners' alone. `list_cookbooks` names every Cookbook and its recipes, and `create_recipe`, `create_tag`, `list_tags` and the Imports take a `cookbook_id`. A read-only Access Key, a Runner Key and a Job Key cannot act as the Operator. [#302]
- An older Version opened from a recipe's History can be taken whole. Whoever writes the recipe can put it back, which makes the recipe what it was then as a new Version on top, with its corrected Readings, and keeps the Version stepped away from in the History, even one saved a minute ago. They can also start a named variation from it, leaving the recipe as it is. Anyone else who sees the recipe can keep that Version as their own copy, and names it where they already have one. Until now an older Version could only be carried back line by line. Both doors gain `put_back_version` and `keep_version_as_copy`, `start_variation` takes a `version_id`, and `changed_since` says in a new field, `keeping_needs_name`, whether keeping needs a name. [#354]
- `create_recipe`, `save_recipe_version`, `edit_recipe` and `start_translation` answer `foods_created`: each Food the save created reading a line whose word no Food answered to, with its id, the word, its Language and the lines read as it. It is empty when the save created none. Until now finding one meant comparing `list_foods` before and after the save. [#358]

### Changed

- On an instance with no Runner, a link to a page with no recipe data is read from the page's own text, the way pasted text is, and opens on the sheet a paste is checked on, saying why. The page is the Source of the recipe made from it. Before, it landed as a bare recipe with the page's title and link and nothing else. A page whose text holds no ingredients still lands bare. `import_web_link` then saves nothing and answers `to_check` alone: what `read_pasted_recipe` would answer, and the page as `source`. [#301]
- Pasted text or a PDF with an `Ingredients` heading (`Ingrédients`, `Ingredientes`) reads nothing above it as an Ingredient Line. The short line just above the heading is the title, past a bare web address and never a line saying how many it serves or who wrote it, and everything else above it goes to the note word for word. A blog post's photo caption, chatter and credit link used to land among the ingredients. [#301]
- In the History, a line comes out of the dot of the Version it started from, and the band saying "Splits into 2 Branches here" is gone. A Translation sits directly under the Version it translates, whenever it was written, where it used to leave the top of the page as if the recipe had split before its first Version. [#250]
- In the History, a Version shows two badges at most, however often it was cooked: yours, as the pot and the last rating you gave it, and everyone else's together, by name. Each opens the cookings it stands for, your older ratings among them. It used to show one badge per cooking, so a Version rated "Again" three times read "Again" three times. `get_thread` says who cooked each Attempt in two new fields, `person_name` and `mine`. [#249]
- A Translation's Ingredient Line reads as the Food of the line it translates when it is made, and when a save brings it up to a newer Version of its source, so `2 c. à soupe de gingembre frais râpé` under `2 tbsp grated fresh ginger` is *ginger*, a correction made by hand on the recipe included. The two lists must have the same shape, and the two lines the same amount and unit. The line keeps its own words. Before, every line of a Translation was read on its own words and made a Food of its own beside its source line's. [#356]

### Fixed

- The History shows a variation nothing has been saved on yet, and a recipe that arrived and was never edited, under the Version it starts from. A recipe left as it was while a variation of it went on changing is shown under its last Version. It left all three out. [#250]
- The History draws five Branches in five colours. The fourth used to take the first one's colour. [#250]
- The MCP door's instructions say that a collapsed save keeps the name and change note the Version already has unless it sends new ones. They said the opposite, that the ones already there were erased, which has not been true since those were kept. [#265]
- `branch_point` and `divergence` asked about a recipe and its Translation, or two Translations of one recipe, refuse the call as a bad request and say where the link is held: `translation` on the Branch, and `translates_version_id` on its Versions. They used to answer an internal error calling the recipe a damaged Bundle. [#251]
- A Bundle that would unpack to more than 256 MB is refused whole, in its Import Report and when a Share Link's recipe is previewed, and says so. [#296]
- The Import Report of a link is titled "The link you brought in". A link Kamosu read on its own used to be titled "Your Crouton library". [#308]
- A phone shows the recipes it holds while Kamosu restarts behind a proxy or answers that it is busy. It used to show the proxy's error page, or Kamosu's error, in their place. [#291]
- Opening a recipe no longer stops a phone answering the next screens from the recipes it holds. Until the server answered, a slow server showed nothing at all. [#291]
- Ending another device's Session in Settings leaves this phone's library on it. It used to be deleted, and downloaded again in full at the next start. `revoke_session` says in a new field, `current`, whether the Session it ended is the one the call was made with. [#291]
- A Bundle whose Food answers to names that two Foods here hold apart makes one new Food and one set of merge suggestions, however many of its Versions carry the line. It used to make a new Food for each Version, and a suggestion between nearly every pair of them. [#292]
- On the writing screen, a short line or Step dragged over a taller one settles where the finger is. The two used to swap back and forth for as long as the finger stayed over part of the taller row. [#293]
- A Step that waits until something looks right and then adds an ingredient links that ingredient, as "cook the onions until soft and add the garlic" now links the garlic. Everything after "until" up to the next comma or full stop used to link nothing. [#294]
- A recipe that arrives as nothing but a title takes the Language of the first ingredients or steps saved into it, whoever saves it, Kamosu's AI agent included. A link to a French page that went to the agent used to stay labelled in the cook's own Language once the agent had filled it in French, since the agent cannot change a Language. A Language somebody said, on arrival or with `set_recipe_language`, is still only offered against. A recipe that arrived bare before this upgrade keeps its Language. [#321]
- On a recipe page marked against another version, and on an older Version's page, every line is drawn in that page's own order. A Step or Ingredient Line moved without rewording is marked with where the other has it: its step number there, or "higher up" or "lower down". A moved or reworded line used to be drawn where the other recipe had it, so a variation's last Step could show as Step 1, and carrying any line across then saved your Steps in the other recipe's order. `divergence`, `changed_since` and an As Cooked's `against` give each row a new field, `place`, saying where each page draws it, and a new state, `moved`. [#303] [#337]
- An Ingredient Line from an old American recipe card reads a lone `c` after an amount as a cup and `#` as a pound, as `t` and `T` already read as spoons: `3/4 c brown sugar` is three quarters of a cup of brown sugar. It used to name a Food "c brown sugar", and an import with no agent kept it. A French `c. à` is still a spoon, and `180 C` in a Step is still an oven. Foods already made this way stay until an Operator merges or deletes them. [#326]
- Three more shapes of Ingredient Line read with a Food. A pack size after the amount, `2 x 400 g tins of whole peeled tomatoes`, reads as two tins of peeled tomatoes; it used to read with no Food, or with `400 g` inside the Food's name. A long French name, `lait de coco non sucré en conserve`, reads whole; it used to be taken for a sentence and named no Food. A line that opens with an aside in brackets, `(white part), finely sliced spring onion`, reads what follows the bracket; it used to read as nothing at all. A Reading already stored stays as it is. [#353]
- A save within the hour after a recipe was exported, as a file or through a Share Link, starts a new Version. It used to fold into the Version the file had carried, and every later file of that recipe was then refused by whoever had received the first, as disagreeing with the recipe they held. `save_recipe_version` and `edit_recipe` answer `collapsed: false` for it, and a name or change note sent alone for the Version that left is refused, as it is once the hour is over. [#328]
- An Ingredient Line moved without rewording keeps its Reading when the recipe is saved, a correction made with `set_reading` included, wherever the line now sits. Adding, removing or reordering a line used to read every line below it again, which lost their corrections and could name new Foods after the misreads. A Reading already lost stays as it is. [#355]
- "Start my own copy" pressed on a recipe in someone else's Cookbook with nothing changed says "Nothing changed, so no copy was made." It used to say "Saved.", though no copy is made of a recipe left as it was. [#329]
- The MCP door no longer lists to an Access Key the eight tools an Access Key is always refused: `mint_access_key`, `mint_runner_key`, `mint_invite`, `mint_recovery_link`, `set_operator`, `disable_account`, `delete_account` and `list_accounts`. An agent used to be shown them, and was refused each time it tried one, as it still is. A Session is listed them as before. [#347]
- A Step that serves the dish no longer links an ingredient named after it. "Ladle the curry into a soup tureen" in a Vietnamese beef curry used to link the curry powder, because "curry" is part of that ingredient's name and the title also holds it. A Step naming an ingredient in full still links it, as "add the curry powder" does, and so does a short name the title does not hold, such as "the cocoa" in a chocolate cake. A short name the title holds now links nothing even where it means the ingredient: "add the chicken" in a roast chicken no longer links the chicken thighs, nor "add the lemon" in a lemon tart the lemon zest. [#357]

[#249]: https://github.com/BatterWorks/Kamosu/issues/249
[#250]: https://github.com/BatterWorks/Kamosu/issues/250
[#251]: https://github.com/BatterWorks/Kamosu/issues/251
[#260]: https://github.com/BatterWorks/Kamosu/issues/260
[#261]: https://github.com/BatterWorks/Kamosu/issues/261
[#262]: https://github.com/BatterWorks/Kamosu/issues/262
[#263]: https://github.com/BatterWorks/Kamosu/issues/263
[#264]: https://github.com/BatterWorks/Kamosu/issues/264
[#265]: https://github.com/BatterWorks/Kamosu/issues/265
[#266]: https://github.com/BatterWorks/Kamosu/issues/266
[#267]: https://github.com/BatterWorks/Kamosu/issues/267
[#268]: https://github.com/BatterWorks/Kamosu/issues/268
[#269]: https://github.com/BatterWorks/Kamosu/issues/269
[#270]: https://github.com/BatterWorks/Kamosu/issues/270
[#271]: https://github.com/BatterWorks/Kamosu/issues/271
[#272]: https://github.com/BatterWorks/Kamosu/issues/272
[#273]: https://github.com/BatterWorks/Kamosu/issues/273
[#274]: https://github.com/BatterWorks/Kamosu/issues/274
[#275]: https://github.com/BatterWorks/Kamosu/issues/275
[#276]: https://github.com/BatterWorks/Kamosu/issues/276
[#277]: https://github.com/BatterWorks/Kamosu/issues/277
[#278]: https://github.com/BatterWorks/Kamosu/issues/278
[#279]: https://github.com/BatterWorks/Kamosu/issues/279
[#281]: https://github.com/BatterWorks/Kamosu/issues/281
[#291]: https://github.com/BatterWorks/Kamosu/issues/291
[#292]: https://github.com/BatterWorks/Kamosu/issues/292
[#293]: https://github.com/BatterWorks/Kamosu/issues/293
[#294]: https://github.com/BatterWorks/Kamosu/issues/294
[#296]: https://github.com/BatterWorks/Kamosu/issues/296
[#301]: https://github.com/BatterWorks/Kamosu/issues/301
[#302]: https://github.com/BatterWorks/Kamosu/issues/302
[#303]: https://github.com/BatterWorks/Kamosu/issues/303
[#308]: https://github.com/BatterWorks/Kamosu/issues/308
[#315]: https://github.com/BatterWorks/Kamosu/issues/315
[#317]: https://github.com/BatterWorks/Kamosu/issues/317
[#318]: https://github.com/BatterWorks/Kamosu/issues/318
[#321]: https://github.com/BatterWorks/Kamosu/issues/321
[#324]: https://github.com/BatterWorks/Kamosu/issues/324
[#325]: https://github.com/BatterWorks/Kamosu/issues/325
[#326]: https://github.com/BatterWorks/Kamosu/issues/326
[#328]: https://github.com/BatterWorks/Kamosu/issues/328
[#329]: https://github.com/BatterWorks/Kamosu/issues/329
[#337]: https://github.com/BatterWorks/Kamosu/issues/337
[#344]: https://github.com/BatterWorks/Kamosu/issues/344
[#345]: https://github.com/BatterWorks/Kamosu/issues/345
[#347]: https://github.com/BatterWorks/Kamosu/issues/347
[#348]: https://github.com/BatterWorks/Kamosu/issues/348
[#349]: https://github.com/BatterWorks/Kamosu/issues/349
[#353]: https://github.com/BatterWorks/Kamosu/issues/353
[#354]: https://github.com/BatterWorks/Kamosu/issues/354
[#355]: https://github.com/BatterWorks/Kamosu/issues/355
[#356]: https://github.com/BatterWorks/Kamosu/issues/356
[#357]: https://github.com/BatterWorks/Kamosu/issues/357
[#358]: https://github.com/BatterWorks/Kamosu/issues/358

## v0.2.0 - 2026-10-12

### After upgrading

- An Operator presses "Read every line again" on the Operator page once. The reader of Ingredient Lines was fixed in four places, listed under Fixed, and a line already saved keeps its old Reading until it is read again. A Reading somebody set by hand is left alone. [#207] [#208] [#213] [#230]

### Added

- Kamosu has a layout for tablets and computers. A window at least 700 pixels wide and 560 tall gets a sidebar down the left edge with Home, Recipes, Shopping, Cooked and Settings, where the phone has its tab bar. The phone layout stays. [#193] [#194]
- On that layout a bottom sheet opens as a window in the middle of the screen. Escape closes every one, and Enter confirms one that asks a single thing. [#196]
- A recipe and its editor are two columns where the window is at least 1000 pixels wide: the Ingredients stay in view while the Method scrolls. [#198]
- The cooking screen is set larger on a tablet or computer and shows the next Step, small, under or beside the one being cooked. [#197]
- Foods keeps its list beside the open Food on a tablet or computer, and the up and down arrow keys move through it. Brought in keeps its list of imports beside the open Report the same way. [#199] [#200]
- Home, Recipes and Cooked fill the window on a tablet or computer, and Shopping is two columns there, with a − and a + on each recipe whose amount can be counted, for how much of it you are making. [#201] [#202]
- A round back arrow sits at the top of every page reached from another page, on both layouts. It never walks back out to another site. [#195] [#219]
- With a mouse, every link, button and list row changes shade under the pointer, and a shelf gets arrows for earlier and later. A touch screen shows none of it. [#203] [#223]
- `/` puts the caret in the recipe search from anywhere but a text field, an open bottom sheet, the cooking screen and the pages read before signing in. [#203]
- A recipe PDF, a Kamosu recipe file or a link dropped onto Recipes is imported, as its row under the + does. [#204]
- A photograph dropped on a recipe's photograph, a Step's or a cooking's sets it, and a Crouton export dropped on its import starts that import. A file dropped anywhere else does nothing, so the browser no longer opens it in place of Kamosu. [#205]
- An older Version opens from the History as a page laid out like the recipe, with every line that has changed since marked on it. You can cook from it, and whoever writes the recipe can carry a line back. Agents get the same comparison from a new operation, `changed_since`. [#211]
- A cooking in Cooked whose lines you changed at the stove lists them under "What you changed" and offers to keep them as a Version. `list_attempts` says the same in a new field, `unkept`. [#210]
- A rating on the recipe page says "on an older Version" or "on another Version" when it was not given on the recipe as it stands, and `get_recipe` says which Version each rating was given on. [#229]
- The Recipes shelf filters to one Cookbook. `search_recipes` and `list_tags` take a `cookbook_id`. [#231]
- Type follows the text size set on the phone or tablet, up to 135% of the size it was drawn at. [#228]
- An image built with `--build-arg VERSION=<version> --build-arg GIT_SHA=<commit>` carries both as its `org.opencontainers.image.version` and `org.opencontainers.image.revision` labels, so `docker inspect` says which release it is. Built with neither, both labels are empty. [#233]

### Changed

- The filters on Recipes are three headed lines of chips: Cookbooks, Kitchens and Tags. A tap holds a chip and a second tap lets it go, so the All chip is gone. "My recipes" is now a tick beside the count, "Cooked or written by me". [#231]
- On a recipe, Cook this stands alone under the servings. The other acts are a grid of small buttons at the foot of the page, and a button is filled once its act has happened: the recipe is on the shopping list, or its Sheet is ready to share. [#220]
- Settings is a menu and four pages: Account, This device, Kitchens and cookbooks, This instance. On the phone it opens from a gear in the header, the header scrolls away with the page, and the tab bar is indigo where it was paper. [#218] [#221]
- A phone turned sideways cooks in two columns, the amounts beside the Step. On every size the switch that keeps the screen awake and "Not really cooking" sit under one ⋯, and the chip saying how much you are making wraps where it was cut short. [#222]
- A Cover is drawn from sixteen pasta shapes where there were eight, so about half the recipes with no photograph change shape once. [#228]
- Words that acted when tapped are now drawn as buttons, and small labels are a size larger. In the editor the tools under each Ingredient Line are small marks beside it. A control reached with the keyboard wears a clear outline where it had the browser's blue ring. [#225]
- Dates are written the same way on every screen, in the interface's language, and every page has a title of its own in the browser tab. A count of one reads "1 recipe", and a tablet is no longer called a phone. [#224]
- The pages read before signing in draw no tab bar and no gear, and the card offering to install Kamosu waits until somebody is signed in. A signed-in person who opens a Cookbook Invite keeps their navigation. [#214] [#226]
- Enter saves a Food's names, a Food's cup weight, a line's Reading and the public address, each of which needed its button before. [#203]
- The source moved to `github.com/BatterWorks/Kamosu`. The old address redirects, and the image is still `battermanz/kamosu`. [#237]

### Fixed

- A line with the amount after the Food is read: "Oignon jaune : 2", "Eau : 800 ml" and "Pâtes orzo 200 g" each used to be stored whole as a Food with no amount. [#207]
- An amount written against its Unit is read, as in "100g Crème". `cs` and `c.s.` are tablespoons, `cc` and `c.c.` teaspoons, `pince` is a pinch, and after an amount a lone `t` is a teaspoon and a lone `T` a tablespoon. "Un peu de", "a little", "a bit of" and "un poco de" opening a line are dropped from the Food's name. [#208]
- French "thé", "dés" and "dé" are words of a Food's name. They were dropped as "the", "des" and "de", so "1 thé vert" named the Food "vert". The Quebec teaspoon, "cuillère à thé", still reads as a teaspoon. [#213]
- An accent typed as its own character stays on a Food's name, so one name typed on two keyboards is one Food. [#230]
- A Step links an ingredient only by the main word of its name. "Add the chicken" no longer shows the chicken broth, and "steamed rice" no longer shows the rice vinegar. A Step saying "le thé" now links the tea. [#212] [#230]
- A cooking kept as a Version shows, with its rating, under that Version in the History. It used to sit under the Version the cooking started from, and the Version that was eaten looked untried. `get_thread` answers `eaten_version_id` on every Attempt. [#209]
- A recipe on the phone's shopping list shows its whole title on one row. The title was squeezed beside "shopping for 4 servings" and wrapped to four lines. [#226]
- The library Kamosu makes its outgoing HTTPS connections with, to fetch a web page to import or the Meaning Search model, is past a published flaw in how it checked the TLS handshake (RUSTSEC-2026-0285). [#244]

[#193]: https://github.com/BatterWorks/Kamosu/issues/193
[#194]: https://github.com/BatterWorks/Kamosu/issues/194
[#195]: https://github.com/BatterWorks/Kamosu/issues/195
[#196]: https://github.com/BatterWorks/Kamosu/issues/196
[#197]: https://github.com/BatterWorks/Kamosu/issues/197
[#198]: https://github.com/BatterWorks/Kamosu/issues/198
[#199]: https://github.com/BatterWorks/Kamosu/issues/199
[#200]: https://github.com/BatterWorks/Kamosu/issues/200
[#201]: https://github.com/BatterWorks/Kamosu/issues/201
[#202]: https://github.com/BatterWorks/Kamosu/issues/202
[#203]: https://github.com/BatterWorks/Kamosu/issues/203
[#204]: https://github.com/BatterWorks/Kamosu/issues/204
[#205]: https://github.com/BatterWorks/Kamosu/issues/205
[#207]: https://github.com/BatterWorks/Kamosu/issues/207
[#208]: https://github.com/BatterWorks/Kamosu/issues/208
[#209]: https://github.com/BatterWorks/Kamosu/issues/209
[#210]: https://github.com/BatterWorks/Kamosu/issues/210
[#211]: https://github.com/BatterWorks/Kamosu/issues/211
[#212]: https://github.com/BatterWorks/Kamosu/issues/212
[#213]: https://github.com/BatterWorks/Kamosu/issues/213
[#214]: https://github.com/BatterWorks/Kamosu/issues/214
[#218]: https://github.com/BatterWorks/Kamosu/issues/218
[#219]: https://github.com/BatterWorks/Kamosu/issues/219
[#220]: https://github.com/BatterWorks/Kamosu/issues/220
[#221]: https://github.com/BatterWorks/Kamosu/issues/221
[#222]: https://github.com/BatterWorks/Kamosu/issues/222
[#223]: https://github.com/BatterWorks/Kamosu/issues/223
[#224]: https://github.com/BatterWorks/Kamosu/issues/224
[#225]: https://github.com/BatterWorks/Kamosu/issues/225
[#226]: https://github.com/BatterWorks/Kamosu/issues/226
[#228]: https://github.com/BatterWorks/Kamosu/issues/228
[#229]: https://github.com/BatterWorks/Kamosu/issues/229
[#230]: https://github.com/BatterWorks/Kamosu/issues/230
[#231]: https://github.com/BatterWorks/Kamosu/issues/231
[#233]: https://github.com/BatterWorks/Kamosu/issues/233
[#237]: https://github.com/BatterWorks/Kamosu/pull/237
[#244]: https://github.com/BatterWorks/Kamosu/pull/244

## v0.1.0 - 2026-09-28

The first release. Kamosu has been cooked from in one household since August
2026. This is the first time anyone else can run it: it is now
public, licensed under the AGPL-3.0, and published as a ready-made image,
`battermanz/kamosu`, for both amd64 and arm64.

Nothing is listed here as added or fixed, since there was no earlier release to
compare with. What 0.1.0 contains:

- **Recipes that keep their history.** Every save is a new Version and none is
  ever lost. Variations and Translations are Branches of the same recipe, and
  the History screen shows where two of them parted and every line that
  differs.
- **Cooking mode.** Step by step, with the amounts each step uses, ticked
  ingredients, timers read out of the step text, and the recipe scaled to what
  you are making. It works offline, and a cooking started on the phone carries
  on on a tablet.
- **Attempts.** A diary of each cooking, with a rating, photos, notes and what
  was actually done, any of which can be promoted into the recipe.
- **Ingredient lines kept as written**, read into amount, unit and food for
  scaling, metric and US conversions, oven temperatures and the shopping list.
- **A shopping list** worked out from the recipes you picked, merging the same
  food across recipes, plus loose items you type.
- **Imports** from a web page's schema.org data, a pasted recipe, a recipe PDF,
  a Crouton export, and another Kamosu's recipe files and Share Links.
- **Sharing.** A Share Link per recipe for anyone who holds it, a printable PDF
  Sheet, and a recipe file carrying the full history.
- **People.** Invites instead of signup, a Cookbook per person that two people
  can join, Kitchens for cooking from each other's Cookbooks, and Operators who
  run the instance without being able to read anyone's recipes through the app.
- **Agents.** Every operation is offered over MCP at `/mcp` exactly as the web
  app gets it, with per-person Access Keys that can be read-only.
- **Search** by words from the start, and by meaning once an Operator turns on
  the optional EmbeddingGemma model, which Kamosu does not ship.
- **Backups** taken automatically, three kept (a day, a week and a month back),
  and a Snapshot before every upgrade migrates the database.
- **English, French and Spanish**, for the interface and for recipes.

The things Kamosu does not defend against are listed in
[SECURITY.md](SECURITY.md).
