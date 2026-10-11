# VSF media type registrations — draft

Two vendor-tree registrations under [RFC 6838](https://www.rfc-editor.org/rfc/rfc6838) §3.2: one for any VSF document, one for a VSF document whose content is visual (a still image or an image sequence). They are submitted through the IANA form at <https://www.iana.org/form/media-types>, one submission per type; each section below is a template ready to paste.

Before submitting:

- **Vendor name.** `vnd.vsf` names the product rather than a company, which RFC 6838 permits when the name is unambiguous. The alternative is `vnd.verichrome.vsf`, but VSF is not only VERICHROME's format: photon uses it for messages, logs and contacts, so the product name is the better fit.
- **Contact.** Fill in a name and an address that will still be read in ten years; IANA publishes it.
- **Spec link.** The "Published specification" field should point to a tagged release, not to `main`, so the reviewer reads what was registered. Tag one before submitting.
- **`video/vnd.vsf`.** Not part of this application. Register it once the streaming form (fragments, audio tracks, timecode) is implemented and published, as MP4 did with `video/mp4` beside `application/mp4`.

---

## 1. `application/vnd.vsf`

**Type name:** application

**Subtype name:** vnd.vsf

**Required parameters:** N/A

**Optional parameters:** N/A

**Encoding considerations:** binary

**Security considerations:**

A VSF document contains no executable content and defines no mechanism for one. It is a self-describing binary container: a header, a table of contents listing named sections by absolute byte offset and length, and the sections themselves, built from typed values.

1. *Parsing.* Lengths, offsets, counts and tensor shapes are read from the document and must be treated as hostile. Integers are variable-width (each value states its own width), so a reader cannot assume a fixed field size. A reader must check every product used to size an allocation (element count × bits per element, offset + length) for overflow before allocating, and must reject offsets or lengths that point outside the document. The reference implementation fails with an error rather than allocating or wrapping, and its test suite covers forged shapes and lengths.
2. *Integrity and authenticity.* A document may carry a BLAKE3 hash of its content, a BLAKE3 hash of each section in the table of contents, and Ed25519 signatures. A hash or signature only means something to a reader that verifies it; a reader that does not verify must not present the content as authenticated.
3. *Confidentiality.* Sections may be encrypted (ChaCha20-Poly1305 / XChaCha20-Poly1305 with X25519 key agreement). The table of contents, section names and sizes are not encrypted and may reveal the document's structure.
4. *Retained history.* A VSF document is edited by appending: each edit adds new sections and a new table of contents at the end of the file, and earlier bytes are never rewritten. The content of every earlier version therefore remains in the file, recoverable by any reader, until the file is truncated or rewritten. Sharing an edited document shares its history. Applications that offer redaction must rewrite the document, not append to it.
5. *Personal data.* Image documents commonly carry capture time, geographic location, device make and model, and a device identity. These are in the clear unless encrypted.

**Interoperability considerations:**

The first bytes of every VSF document are the magic `52 C3 85 3C` (`RÅ<`, UTF-8), followed by the format version and the oldest version a reader of this document must support. The current format version is 9. Changes within a version are additive: a reader skips sections and fields it does not recognise.

A document that has been edited carries several headers. The last 16 bytes of such a document are a fixed trailer (`3E C3 85 52 00 56 53 46` followed by a 64-bit big-endian offset) naming the current header. A reader that does not know about trailers reads the header at offset 0 and sees the document as originally written, which is a complete and valid document.

**Published specification:** <https://github.com/nickspiker/vsf> — `README.md` (wire format, value types, header and table of contents), `idt/image.md` (visual documents), `idt/` (colour characterization). Reference implementation: the `vsf` Rust crate, <https://crates.io/crates/vsf>.

**Applications that use this media type:** photon (messaging), opsin (spectral image viewer and converter), lumis (camera), chameleon (colour calibration), fluor (UI toolkit), and the `vsfinfo` / `vsflog` command-line inspectors.

**Fragment identifier considerations:** N/A

**Additional information:**

- Deprecated alias names for this type: N/A
- Magic number(s): `52 C3 85 3C` at offset 0
- File extension(s): `.vsf`
- Macintosh file type code(s): N/A; Uniform Type Identifier `com.nickspiker.vsf`

**Person & email address to contact for further information:** _[name, email]_

**Intended usage:** COMMON

**Restrictions on usage:** None

**Author:** Nick Spiker

**Change controller:** Nick Spiker, for the VSF project

---

## 2. `image/vnd.vsf`

A VSF document whose content is visual: one still image, a burst, or a stack, with the sensor data, its colour characterization, capture metadata and edit history. It is the same container as `application/vnd.vsf`; this type tells a receiver that the document holds an image, so it can be routed to image software. A document of this type always contains a section named `visual` (or, for documents written before October 2026, `spectral_image`).

**Type name:** image

**Subtype name:** vnd.vsf

**Required parameters:** N/A

**Optional parameters:** N/A

**Encoding considerations:** binary

**Security considerations:** As for `application/vnd.vsf` (section 1), and in addition:

1. Image planes store samples at their native bit depth, from 1 to 128 bits per sample, with dimensions stated in the document. A reader must bound the decoded size before allocating and should decode large planes by rows or tiles rather than whole.
2. Edits are stored as instructions applied when the image is rendered (a list of operations with parameters), never as changed pixels. Those instructions are data: a reader evaluates them with a fixed set of operations, and an operation it does not recognise is reported, not guessed. Nothing in the document is executed.
3. Retained history (section 1, item 4) applies with particular force: a cropped, recoloured or otherwise edited image still contains the original, unedited sensor data, because edits are appended and never destructive. This is by design: it keeps the capture verifiable. It means an edited image always reveals what was cut out of it.

**Interoperability considerations:** As for `application/vnd.vsf`. The image model is a set of named sections: tracks, frame spans, planes (each with its own content hash), capture metadata, a colour characterization mapping the camera's channels to the VSF RGB colour space, and the rendering program. A reader that understands the planes but not the rendering program can still show the sensor data as characterized. Documents written by the earlier single-section image form (`spectral_image`) are read as a one-frame image with linear samples.

**Published specification:** <https://github.com/nickspiker/vsf> — `idt/image.md` (normative for visual documents), `idt/README.md` and `idt/relative.md` (colour characterization), `colour.md` (the VSF RGB colour space).

**Applications that use this media type:** opsin (writes and reads), lumis (writes), chameleon (reads, for calibration targets), photon (image attachments).

**Fragment identifier considerations:** N/A

**Additional information:**

- Deprecated alias names for this type: N/A
- Magic number(s): `52 C3 85 3C` at offset 0
- File extension(s): `.vsf`
- Macintosh file type code(s): N/A; Uniform Type Identifier `com.nickspiker.vsf`

**Person & email address to contact for further information:** _[name, email]_

**Intended usage:** COMMON

**Restrictions on usage:** None

**Author:** Nick Spiker

**Change controller:** Nick Spiker, for the VSF project
