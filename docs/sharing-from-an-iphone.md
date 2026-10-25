# The iPhone Shortcut, step by step

An iPhone lists no web app in its Share menu, so a cook on an iPhone adds a
recipe from any app through an Apple Shortcut that Kamosu publishes (ADR 0055,
[#452](https://github.com/BatterWorks/Kamosu/issues/452)). Apple publishes a
Shortcut only from an Apple device, so it cannot be built from this
repository. This page is how it is built, action by action, so that it can be
built again exactly if its iCloud link is ever lost.

The Shortcut holds as little as it can. It sends what it was given to
`add_this` and shows the message `add_this` answers with. Every rule lives in
Kamosu, where a release can change it: the Shortcut on someone's phone is the
one piece of Kamosu a release cannot update.

## What the Shortcut sends

- **A link**, when the Share menu hands it a URL or a Safari web page. It
  sends the link and not the page, as `{ "link": …, "shortcut_version": 1 }`.
- **Everything else as files.** Each item is sent to `POST /api/uploads` on
  its own, and their ids go to `add_this` together, in order and separated
  by commas, as `{ "upload_ids": "…,…", "shortcut_version": 1 }`: a
  Shortcut builds flat fields more reliably than a list. Kamosu
  decides what they are: one PDF, one or more photos of one recipe, or text,
  which a Shortcut turns into a file of plain text.

Every request carries the header `Authorization: Bearer <key>`.

## The setup code

Settings → Share from your phone shows one code, once: the format's name,
`kamosu1`, the address the phone reaches Kamosu at, and the Add-only key, with
one space between each.

```
kamosu1 https://recipes.kamosu.example 3f9c…e1a0
```

No part holds a space. The address has no `/` at its end. The Shortcut splits
the code at the spaces and takes the second part as the address and the last
as the key. A change to this format is a change to every installed Shortcut,
so it waits for a new Shortcut version, and its name becomes `kamosu2`.

## Why photos are converted

An iPhone keeps its photos as HEIC, which Kamosu does not read: `add_this`
refuses one as neither text, a PDF nor a picture. Safari turns one into a JPEG
when a page asks for a file; a Shortcut hands over the photo as it is kept. So
the Shortcut converts each HEIC or HEIF to JPEG first, and drops where it was
taken while it does.

It converts nothing else. JPEG has no transparency, so a PNG with a clear
background, a recipe card with dark text on nothing, would arrive black.

## The version

The Shortcut sends its own version with every `add_this`, starting at 1. It
is `SHORTCUT_VERSION` in `src/core/sharing.rs`. A Shortcut that changes how it
calls Kamosu gets the next number, and `SHORTCUT_VERSION` moves with it in the
same release, together with the iCloud link in `ui/src/lib/sharing.ts`.
`OLDEST_SHORTCUT_SERVED` moves only when Kamosu can no longer answer an older
Shortcut: one older than that is told to install the new one.

## Building it

In the Shortcuts app, make a new Shortcut named **Add to Kamosu**, and add
these actions in this order. Words in **bold** are what the action or its
fields are called in Shortcuts. Variables are set with **Set Variable** where
a step says to name one.

### Its details

Open the Shortcut's details (the ⓘ at the bottom):

1. Turn on **Show in Share Sheet**.
2. Under **Share Sheet Types**, leave only **Images**, **PDFs**, **Safari web
   pages**, **Text** and **URLs** on.
3. Turn off **Show in Spotlight** if it is on. The Shortcut needs something
   shared with it.

The first action Shortcuts adds becomes **Receive Images, PDFs, Safari web
pages, Text and URLs input from Share Sheet**. Set its **If there's no input**
to **Stop and Respond**, with the response
`Share a page, a photo, text or a PDF to Kamosu.`

### The setup code

1. **Text**, holding the word `code`. In the Shortcut's details, under
   **Setup**, choose **Add Import Question** on this Text action, with the
   question below and no default answer. Set Variable **Code** to the Text.

   > Kamosu → Settings → Share from your phone: paste the setup code. ·
   > Kamosu → Réglages → Partager depuis votre téléphone : collez le code de
   > réglage. · Kamosu → Ajustes → Compartir desde tu teléfono: pega el
   > código de configuración.

2. **Split Text** Code by **Spaces**.
3. **Get Item from List**: **Item At Index** `2` of Split Text. Set
   Variable **Address**.
4. **Get Item from List**: **Last Item** of Split Text. Set Variable **Key**.
5. **Text**: `Bearer ` followed by the variable Key (one space between).
   Set Variable **Authorization**.

### A link, or files

6. **Get Type** of Shortcut Input. Set Variable **Type**.
7. **If** Type **is** `URL`. Inside it: **Get Item from List**: **First
   Item** of Shortcut Input. Set Variable **Link**.
8. **Otherwise**: **If** Type **contains** `Safari`. Inside it: **Get Details
   of Safari Web Page**: **Page URL** of Shortcut Input. Set Variable **Link**.
   **End If**, then **End If**.

   Get Type's names are translated and the comparison minds capitals: in
   English a Safari page is `Safari Web Page`, not `Safari web page`, and every
   language keeps the word `Safari`. **Get URLs from Input** finds nothing in a
   Safari page, and **Get Details of Safari Web Page** stops the Shortcut on
   anything else, which is why it waits behind the test.
9. **If** Link **has any value**:
   1. **Get Contents of URL**: the Text `Address/api/op/add_this` (the
      variable Address, then `/api/op/add_this`). **Method** POST.
      **Headers**: `Authorization` set to the variable Authorization.
      **Request Body** JSON, with two fields: `link` (Text) set to Link, and
      `shortcut_version` (Number) set to `1`.
10. **Otherwise**:
    1. **Repeat with Each** item in Shortcut Input. Inside it:
       1. **Get Type** of Repeat Item. Set Variable **Item Type**.
       2. **If** Item Type **contains** `Image`, and **Otherwise**, **If**
          Item Type **contains** `Photo`:
          1. **Get Details of Files**: **File Extension** of Repeat Item,
             then **Change Case** to **lowercase**.
          2. **If** the Updated Text **contains** `hei`: **Convert Image**
             Repeat Item to **JPEG**, with **Preserve Metadata** off, and Set
             Variable **Page** to the Converted Image. **Otherwise**: Set
             Variable **Page** to Repeat Item. **End If**.

          **Otherwise**: Set Variable **Page** to Repeat Item. **End If**,
          **End If**.
       3. **Get Contents of URL**: `Address/api/uploads`. **Method** POST.
          **Headers**: `Authorization` set to Authorization. **Request
          Body** File, set to Page.
       4. **Get Dictionary Value**: **Value** for key `result` in Contents
          of URL.
       5. **If** Dictionary Value **has any value**: **Get Dictionary
          Value**: **Value** for key `upload_id` in it. **Otherwise**: **Get
          Dictionary Value** `error` in Contents of URL, then `message` in
          that, **Show Notification** with it, and **Stop This Shortcut**.
          **End If**.

          One key at a time, here and in steps 12 to 14: a key path such as
          `result.upload_id` that misses stops the Shortcut with Apple's own
          "Could not evaluate the key path", where a single missing key is
          only empty. Kamosu's refusal carries `error`, not `result`.
    2. **End Repeat**.
    3. **Combine Text**: Repeat Results, with **Custom** separator `,`.
    4. **Get Contents of URL**: `Address/api/op/add_this`. **Method** POST.
       **Headers**: `Authorization` set to Authorization. **Request Body**
       JSON, with two fields: `upload_ids` (Text) set to the Combined Text,
       and `shortcut_version` (Number) set to `1`.
11. **End If**.

### The message

12. **Get Dictionary Value**: **Value** for key `result` in the **If
    Result** of step 9 to 11.
13. **If** Dictionary Value **has any value**: **Get Dictionary Value**
    `message` in it, and **Show Notification** with that as its body and
    `Kamosu` as its title.
14. **Otherwise**: **Get Dictionary Value** `error` in the If Result, then
    `message` in that, and **Show Notification** with it.
15. **End If**.

That is the whole Shortcut. It holds no other rule.

When Kamosu cannot be reached at all, **Get Contents of URL** stops the
Shortcut and iOS shows its own alert saying the server could not be reached.
A Shortcut has no way to catch that and word it itself.

## Checking it before publishing

On the iPhone, against the dev instance, share each of these to **Add to
Kamosu** and read the notification: a recipe site from Safari, an Instagram
post through **Share to…**, one photo, three photos, text copied from Notes,
and a PDF from Files. Each should arrive on the shelf, and each notification
should be Kamosu's message.

Where a link from Safari is sent as a file instead, look at what **Get Type**
calls a Safari page on that iOS: a Shortcut that takes the same input and
shows **Get Type** of it in an alert says. Step 8 needs only a word that name
always holds.

## Publishing it

1. In the Shortcut's details, **Share** → **Copy iCloud Link**, and choose to
   share it with **Anyone**. The import question clears the code from the
   copy, so no key goes with the link.
2. Open the link in a private Safari window and read the page it shows. The
   repository is public: the page must name no person and no Apple account.
3. Put the link in `SHORTCUT_URL` in `ui/src/lib/sharing.ts` and below.

The published Shortcut: <https://www.icloud.com/shortcuts/4e9859c8609a4221846f0f55915c6423>
