# The agent's answer key

Written on 2026-10-16 for #324, before any of these sources was handed to the agent. Each entry says what the right result is, so that a run of any model at any effort can be scored by reading the result beside its key. What the runs found is in Part 7 of [the research note behind ADR 0048](ai-recipe-import-in-other-apps.md#part-7-the-agent-on-sources-that-go-wrong).

Every source is public or was written for the test. A public page is named by a link that will not change: a Wikibooks page by its revision, a page that may change by its Wayback Machine snapshot. None is copied here. The sources written for the test are in [`agent-test-sources/`](agent-test-sources/), and an entry says how each was handed over.

## Scoring a result

Read the recipe the Job left, its Readings and its Import Report, beside the key.

1. **Outcome.** `completed`, `could_not_be_read` or `failed`, as the key says. A wrong outcome fails the source whatever else is right.
2. **Title.** The recipe's own name. A page's headline in its place is wrong.
3. **Lines: right out of expected.** A line is right when it holds the source's quantity and unit as written (both, where the source gives two), and its Reading has that amount, that unit and the right Food. The Food is the bare ingredient. The library's own word for the same thing counts, such as "baking soda" for "bicarbonate of soda". A line the key has no place for, such as one taken from a story or a comment, counts against the total as a line expected and not right.
4. **Step facts: right out of expected.** The facts the key lists: temperatures, times, sizes, and the order of what goes in. The wording of a step is free.
5. **Left undone.** Each thing the key says belongs in `left_undone` (or `unsure`, where the key allows) is named there.
6. **Faults that fail the source outright:**
   - a number, a unit or an ingredient that the source does not hold
   - another cook's recipe read in place of the source
   - a second recipe's lines or steps merged into the first
   - text in a language other than the source's, or a mix of two
   - any change outside the Job's one recipe

A source **passes** when its outcome is right, it has no fault from 6, its lines and step facts are all right, and its `left_undone` names what the key asks. A run that misses by one line or one fact is recorded as such, with the miss.

Where a source fails, its cause is one of three. **The instructions** did not say what to do. **The model** could not do what they said, such as reading a fraction wrong in a clear picture. **Kamosu** did something wrong. When the cause is in doubt between the first two, one run at a higher effort decides it: a pass there puts the cause on the model.

## The sources

### 1. A long video with no written recipe anywhere

- **Source:** Men With The Pot, "Steak Fries", <https://www.youtube.com/watch?v=n-NQ7-CExeE>, 9 min 46 s. Handed over as a link.
- **What makes it hard:** nobody speaks, the description holds no recipe, the channel's site sells knives and seasoning, and the uploader left no comment. Short captions name each ingredient as it goes in, in English and Polish, and never say how much. Several things are made and put together at the end.
- **Outcome:** `completed`.
- **Title:** names steak fries (flatbread may be named too).
- **Yield and times:** none. The video gives neither.
- **Lines (22):** every one has no amount. Flour, sea salt, milk, butter, sugar, yeast, olive oil (the flatbreads); garlic, parsley, rosemary (the butter brushed on them); egg yolks, lemon juice, mustard, rapeseed oil, salt and pepper (the sauce); onion, lemon pepper seasoning, beer (the onion rings); sirloin steak, eggs, panko breadcrumbs, Montreal steak seasoning (the steak). Oil for frying may be a line of its own. Sections by part are right but not required.
- **Step facts (7):** the dough is mixed, kneaded and left to rise before it is shaped; the onions are cut into rings; the sauce is whisked from the yolks, lemon juice and mustard with the oil poured in; the flatbreads are cooked in a pan over the fire and brushed with the garlic butter using the rosemary; the onion rings go through a beer batter and are deep-fried; the steak is cut into strips, coated in flour, egg and seasoned panko, and deep-fried; the flatbread is spread with the sauce, filled with the steak and onion rings, and rolled.
- **Photographs:** a main photograph of the finished dish, and photographs of steps, all frames of this video.
- **Left undone:** every amount, the yield, the times and the heat.
- **Fails on:** any amount or temperature, since the video gives none.

### 2. A handwritten card, continued on a second card

- **Source:** "Apple cake recipe card - Michigan circa 1950", Wikimedia Commons, CC BY-SA 4.0, <https://commons.wikimedia.org/wiki/File:Apple_cake_recipe_card_-_Michigan_circa_1950.jpg>. The front and back of the card are one picture. Handed over as that picture.
- **What makes it hard:** cursive with shorthand ("1 T", "¼ #", "1 t"), two braces that group lines with an action, the milk's amount given only in the method, and a glaze with no amounts.
- **Outcome:** `completed`.
- **Title:** Apple Cake. "M Wittmann", in brackets beside it, is whose recipe it was, not part of the title.
- **Yield and times:** no yield; bakes 45 minutes.
- **Lines (11):** 1½ c flour; 1 T (tablespoon) baking powder; ¼ lb (written ¼ #) butter; ⅔ c sugar; ½ lemon's rind (Food lemon zest or lemon rind); 3 eggs, separated; ¼ c milk (from the method); apple slices, no amount; and for the glaze: sugar, no amount; 1 t (teaspoon) vanilla; water, no amount.
- **Step facts (6):** flour and baking powder sifted together; butter, sugar, lemon rind and the yolks creamed; the flour and milk added alternately; the beaten whites folded in; apple slices on top; baked 45 minutes at 350° (Fahrenheit; a Celsius figure beside it is fine, in place of it is not).
- **Left undone:** how many apples, how much sugar and water in the glaze, the size of the tin.
- **Fails on:** 1 T read as a teaspoon, ¼ # read as anything but a quarter pound, 350° turned into Celsius alone.

### 3. A picture upside down

- **Source:** "Spicy Gingerbread (Shirley G.) Handwritten 2024-05-21 103943 page 1", Wikimedia Commons, CC BY-SA 4.0, <https://commons.wikimedia.org/wiki/File:Spicy_Gingerbread_(Shirley_G.)_Handwritten_2024-05-21_103943_page_1.jpg>, turned 180 degrees (`ffmpeg -vf transpose=1,transpose=1`). Handed over as that picture.
- **What makes it hard:** upside down, typed in two columns of ingredients, with "t", "c" and "soda".
- **Outcome:** `completed`.
- **Title:** Spicy Gingerbread. "Shirley G" is whose recipe it was.
- **Yield and times:** none.
- **Lines (12):** 2 eggs; ¾ c brown sugar; ¾ c molasses; ¾ c melted shortening; 2½ c flour; ½ t cloves; ½ t baking powder; 2 t soda (Food baking soda); 2 t ginger; 1½ t cinnamon; ½ t nutmeg; 1 c boiling water. "t" is a teaspoon and "c" a cup.
- **Step facts (4):** beaten eggs added to the sugar, molasses and shortening; then the dry ingredients; then the hot water; baked in a moderate oven.
- **Left undone:** the oven's temperature as a number, the time and the tin.
- **Fails on:** "moderate oven" turned into a number with nothing to say it was a guess; a teaspoon read as a tablespoon.

### 4. A PDF in two columns

- **Source:** [`carrot-soup-two-columns.html`](agent-test-sources/carrot-soup-two-columns.html), printed to one A4 page with text in Chrome. The ingredient list starts at the foot of the left column and goes on at the top of the right one, so `pdftotext -layout` prints the first two ingredients below the rest, beside the method. Handed over as the PDF.
- **Outcome:** `completed`.
- **Title:** Spiced carrot and red lentil soup.
- **Yield and times:** serves 4; 40 minutes in all.
- **Lines (9):** 2 tsp cumin seeds; a pinch of chilli flakes; 2 tbsp olive oil; 600 g carrots; 140 g split red lentils; 1 litre vegetable stock; 125 ml milk; juice of ½ lemon; plain yogurt and coriander to serve (one line or two).
- **Step facts (4):** the seeds and flakes toasted dry for 1 minute and half kept back; simmered 15 minutes with the oil, carrots, lentils, stock and milk; blended, then the lemon juice and salt; served with yogurt, the kept spices and coriander.
- **Note:** what the page says around the method may go in the note: ground cumin in place of seeds, thinning with stock, freezing for 3 months.
- **Fails on:** a line of the story read as an ingredient, or the first two ingredients lost.

### 5. Grams and cups side by side

- **Source:** [`oatmeal-cookies-grams-and-cups.html`](agent-test-sources/oatmeal-cookies-grams-and-cups.html), printed to A4 with text in Chrome. The ingredients are a table with a column for grams and a column for cups and spoons. Handed over as the PDF.
- **Outcome:** `completed`.
- **Title:** Oatmeal raisin cookies.
- **Yield and times:** 18 cookies; prep 15 minutes, bake 10 to 12 minutes.
- **Lines (9), each with both measures in one line:** 115 g (½ cup) butter; 150 g (¾ cup, packed) brown sugar; 1 egg; 5 ml (1 tsp) vanilla extract; 125 g (1 cup) plain flour; 2.5 g (½ tsp) bicarbonate of soda; 1.5 g (¼ tsp) salt; 135 g (1½ cups) rolled oats; 80 g (½ cup) raisins. The order of the two measures is free; the Reading takes the first.
- **Step facts (4):** oven at 175 °C (350 °F); butter and sugar beaten, then egg and vanilla; 18 balls 5 cm apart, baked 10 to 12 minutes; 5 minutes on the trays before the rack.
- **Fails on:** one of the two measures dropped, or a line made of each.

### 6. A recipe in two languages side by side

- **Source:** [`crepes-two-languages.html`](agent-test-sources/crepes-two-languages.html), printed to A4 with text in Chrome: English in the left column, the same recipe in French in the right. Handed over as the PDF.
- **Outcome:** `completed`.
- **Language:** one of the two, whole: every line, step and note in it, and the recipe's Language saying so. Either is right.
- **Title:** Crêpes.
- **Yield:** 12.
- **Lines (5):** 125 g plain flour; a pinch of salt; 2 eggs; 300 ml (30 cl) milk; 25 g butter, melted, plus a little for the pan.
- **Step facts (4):** flour, salt and eggs whisked, then the milk a little at a time; the batter rests 30 minutes; a 20 cm pan, 3 tablespoons of batter; about 1 minute a side.
- **Fails on:** both languages in one recipe, or each line twice.

### 7. A page holding several recipes

- **Source:** [`three-dressings.html`](agent-test-sources/three-dressings.html), served for the test as a page with no recipe data, at `https://httpbin.org/base64/<the file, base64url>`. Handed over as that link.
- **Outcome:** `completed`, holding the first recipe alone.
- **Title:** Lemon tahini dressing, not the page's headline.
- **Yield:** about 200 ml.
- **Lines (5):** 4 tbsp tahini; 3 tbsp lemon juice; 1 small garlic clove, grated; 5 tbsp cold water; ½ tsp salt.
- **Step facts (3):** tahini, lemon and garlic whisked, and it seizes; water whisked in a spoonful at a time until it pours like double cream; salted.
- **Left undone:** the honey mustard vinaigrette and the buttermilk herb dressing, named as recipes on the page that were not read.
- **Fails on:** any line or step of the other two.

### 8. Ingredient groups, and a recipe that calls for another recipe of its own

- **Source:** [`chicken-leek-pie.txt`](agent-test-sources/chicken-leek-pie.txt), pasted. The pie's ingredients are in two groups, and one line is "1 quantity rough puff pastry (see below)", whose recipe follows the pie's method. It also writes amounts in words ("two leeks", "a handful").
- **Outcome:** `completed`, one recipe.
- **Title:** Chicken, leek and tarragon pie.
- **Yield:** serves 4 to 6.
- **Lines (15), in three sections (the filling, to finish, the pastry):** 2 tbsp olive oil; two leeks (amount 2); 600 g boneless chicken thighs; 1 tbsp plain flour; 150 ml dry white wine; 300 ml chicken stock; 4 tbsp crème fraîche; a handful of tarragon; salt and pepper to taste; 1 quantity rough puff pastry; 1 egg; and for the pastry 250 g plain flour; ½ tsp salt; 250 g cold butter; about 150 ml iced water.
- **Step facts (8):** leeks 10 minutes, then the chicken 5; flour, then wine bubbled 2 minutes; stock, simmered 20 minutes; crème fraîche and tarragon, then cooled completely; oven at 200 °C, a 1.5 litre dish, pastry rolled 5 mm; baked 35 to 40 minutes; the pastry rolled to about 20 x 50 cm and folded in thirds with a quarter turn, four times in all; chilled at least 30 minutes. The pastry's steps are marked as the pastry's, before the pie's or after.
- **Left undone:** that the pastry is a recipe of its own in the source, kept here as a section, and could be a recipe of its own.
- **Fails on:** the pastry's lines or steps missing; "two leeks" or "a handful" changed into a number the source does not give.

### 9. Ranges, cans with no size, and amounts left to taste

- **Source:** Wikibooks, "Cookbook:Bell Pepper Chili", revision 4523904, <https://en.wikibooks.org/w/index.php?title=Cookbook:Bell_Pepper_Chili&oldid=4523904>. Handed over as that link. No recipe data, so it goes to the agent.
- **Outcome:** `completed`.
- **Title:** Bell Pepper Chili.
- **Yield and times:** 12 cups, 12 servings; 80 minutes.
- **Lines (9):** 3–3½ pounds ground beef; 6–8 onions, coarsely chopped; 2–3 bell peppers, coarsely chopped; chili powder, no amount; garlic salt, no amount; 1 can (about 2 cups) kidney beans; 2 cans (64 ounces) pureed crushed tomatoes; cooked white rice; cheddar cheese, shredded (optional). A range stays a range in the line and in its Reading.
- **Step facts (6):** beef fried on medium until brown, grease drained; peppers and onions fried until the onions are transparent; tomatoes simmered 1 hour; beans, 10 minutes more; chili powder and garlic salt to taste; served over rice with cheddar.
- **Photographs:** the page's picture of the served dish as the main photograph. Its gallery pictures steps; using them is right but not required.
- **Fails on:** a range turned into one number; an amount given to the chili powder or garlic salt.

### 10. A story-first page that refuses Kamosu

- **Source:** Simply Recipes, "Grandma's Sheet Pan Sliders", <https://www.simplyrecipes.com/grandmas-sheet-pan-sliders-recipe-11919325>, which answered `402` to this machine. Its Wayback Machine copy: <https://web.archive.org/web/20260427064409/https://www.simplyrecipes.com/grandmas-sheet-pan-sliders-recipe-11919325>. Handed over as the page's own link.
- **What makes it hard:** about 4,000 characters of story and tips before the recipe, toppings named in the story, a headline that is not the recipe's name, and recipe data whose first step is empty: the oven's temperature is in the page's text alone.
- **Outcome:** `completed`, read from the Wayback Machine's copy, with the Source still the page's own link and the note saying where it was read.
- **Title:** Grandma's Sheet Pan Sliders, not "My Grandma's 5-Ingredient Sheet Pan Sliders Are the Easiest You'll Ever Make".
- **Yield and times:** 6 servings, 12 sliders; prep 10, cook 30, total 40 minutes.
- **Lines (6):** 1½ pounds 93% lean ground sirloin; 3 tablespoons seasoned breadcrumbs; ½ cup finely chopped onions; 1 teaspoon kosher salt; 6 slices cheddar or American cheese; 1 package (12-count) sweet dinner rolls or slider buns.
- **Step facts (4):** oven at 400 °F; pressed into a 9 x 13-inch pan and poked with a fork; baked 25 minutes and the liquid drained; cheese melted about 2 minutes, cut into 2-inch squares on the rolls.
- **Fails on:** the 400 °F missing; pickles or ketchup from the story as lines; another site's sliders.

### 11. A page whose own comments correct the recipe

- **Source:** [`banana-bread-with-comments.html`](agent-test-sources/banana-bread-with-comments.html), served for the test as a page with no recipe data at `https://httpbin.org/base64/<the file, base64url>`. Handed over as that link. The recipe says 1 tbsp bicarbonate of soda; the author's reply in the comments says it is a typo for 1 tsp. Two readers' comments change other things on their own say.
- **Outcome:** `completed`.
- **Title:** Brown butter banana bread.
- **Yield:** one 900 g (2 lb) loaf, about 10 slices.
- **Lines (9):** 115 g unsalted butter; 3 very ripe bananas (about 350 g peeled); 150 g light brown sugar; 2 eggs; 1 tsp vanilla extract; 220 g plain flour; **1 tsp** bicarbonate of soda; ½ tsp salt; 75 g walnuts, chopped.
- **Step facts (5):** oven at 180 °C (160 °C fan), a 900 g tin; butter browned about 5 minutes and cooled 10; bananas, then sugar, eggs and vanilla; baked 40 minutes; 15 minutes in the tin.
- **Note or unsure:** that the page says 1 tbsp and its author corrected it to 1 tsp in the comments.
- **Fails on:** 1 tbsp kept with no word of the correction; the readers' chocolate chips, 170 °C or hour taken into the recipe.

### 12. A menu

- **Source:** "DINNER MENU (held by) A.M. SWEET & SON HOTEL & RESTAURANT (at) NEW YORK, NY", 1900, New York Public Library, public domain, <https://commons.wikimedia.org/wiki/File:DINNER_MENU_(held_by)_A.M._SWEET_%26_SON_HOTEL_%26_RESTAURANT_(at)_NEW_YORK,_NY_(HOTEL-REST;)_(NYPL_Hades-272167-4000007092).jpg>, at 1920 pixels wide. Handed over as that picture.
- **Outcome:** `could_not_be_read`, with a reason saying it is a menu. No recipe is created.

### 13. A restaurant review

- **Source:** Grace Dent's review of Dante, Mayfair, The Guardian, 4 October 2026, <https://www.theguardian.com/food/2026/oct/04/dante-mayfair-claridges-london-w1-restaurant-review-grace-dent>. Handed over as that link. It has article data and no recipe data, so it lands bare and goes to the agent.
- **Outcome:** `could_not_be_read`, with a reason saying it is a review. The bare recipe is left as it landed: no lines or steps.

### 14. A list of links

- **Source:** Wikibooks, "Cookbook:Omelet Recipes", revision 4606042, <https://en.wikibooks.org/w/index.php?title=Cookbook:Omelet_Recipes&oldid=4606042>: a short paragraph and links to other pages. Handed over as that link.
- **Outcome:** `could_not_be_read`. No page it links to is read in its place.

### 15. A shopping list

- **Source:** [`shopping-list.txt`](agent-test-sources/shopping-list.txt), pasted.
- **Outcome:** `could_not_be_read`. No recipe is created.

### 16. A refused page with no copy to read

- **Source:** EatingWell, "Paul Newman's original vinaigrette", <https://www.eatingwell.com/paul-newman-original-vinaigrette-recipe-12144264>. It answers `402` to this machine, and the Wayback Machine's only copy (29 September 2026) is that same refusal. Handed over as the link.
- **Outcome:** `failed`, with a reason saying the site refused and no readable copy exists. The bare recipe keeps its title and Source and gains nothing.
- **Fails on:** any recipe for the dish read from another site.

### 17. Reading again when nothing is wrong

- **Source:** whichever of the recipes above passed and came from a kept Original or a link, read again with `read_recipe_again` and nothing changed since.
- **Outcome:** `completed`. No line, step or number changes; the report says the recipe matched its source.

### 18. Reading again when the cook's edit is right and the source is not

- **Source:** [`lemon-shortbread.txt`](agent-test-sources/lemon-shortbread.txt), pasted. Its first step says "18 °C (350 °F)", a typo for 180 °C that its own Fahrenheit figure shows.
- **The first reading:** `completed`, with the step as the source writes it and `unsure` naming the mismatch. Title Lemon shortbread; makes 16 fingers. Lines (5): 225 g butter, softened; 110 g caster sugar; finely grated zest of 1 lemon; 325 g plain flour; 1 tbsp demerara sugar. Step facts (3): a 20 x 30 cm tin; baked 25 to 30 minutes; cut into 16 fingers while warm.
- **Then the cook's edit:** the first step changed to "180 °C (350 °F)" through the recipe's own editor.
- **Then reading again:** `completed`. The step keeps 180 °C, and the report says the source has 18 °C, which its own 350 °F contradicts, and that the cook's 180 °C was kept.
- **Fails on:** 18 °C put back.
