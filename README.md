# bodge
Convert a patch for plain text prose to something more human readable.

I write most of my prose in version controlled markdown, and I use
LLMs to help with minor edits, like grammar, active voice, and 
consistences, never content wholesale.  I take suggestions under 
advisement, not as mandate.

I've found that a good workflow is to have an LLM read my material,
then suggest changes via a git style `patch`.  This makes it easy to
see what the differences are.

However sometimes it can be a little difficult to make out what 
exactly is changing in those patches.

For example: 

```diff
--- a/sample.markdown
+++ b/sample.markdown
@@ -1,20 +1,20 @@
-CHAPTER I.
-==========
-The Period
-----------
+CHAPTER I, DUDE.
+================
+The Vibe
+--------
 
-It was the best of times, it was the worst of times, it was the age of
-wisdom, it was the age of foolishness, it was the epoch of belief, it
-was the epoch of incredulity, it was the season of Light, it was the
+It was the most tubular of times, it was the most bogus of times, it was the age of
+wisdom, bro, it was the age of total foolishness, it was the epoch of belief, it
+was the epoch of like, no way, it was the season of Light, it was the
 season of Darkness, it was the spring of hope, it was the winter of
 despair, we had everything before us, we had nothing before us, we were
 all going direct to Heaven, we were all going direct the other way--in
-short, the period was so far like the present period, that some of its
+short, dude, the period was so far like the present period, that some of its
 noisiest authorities insisted on its being received, for good or for
-evil, in the superlative degree of comparison only.
+evil, in the superlative degree of comparison only. Gnarly.
 
 There were a king with a large jaw and a queen with a plain face, on the
 throne of England; there were a king with a large jaw and a queen with
-a fair face, on the throne of France. In both countries it was clearer
+a fair face, on the throne of France, totally. In both countries it was clearer
 than crystal to the lords of the State preserves of loaves and fishes,
-that things in general were settled for ever.
+that things in general were settled for ever, like, forever.
```

This program re-formats the diff by line, and aligns the changes by 
word:

```diff
--- a/sample.markdown
+++ b/sample.markdown
@@ -1,20 +1,20 @@
-CHAPTER I.
+CHAPTER I, DUDE.
-==========
+================
-The Period
+The Vibe
-----------
+--------
-It was the best         of times, it was the worst      of times, it was the age of
+It was the most tubular of times, it was the most bogus of times, it was the age of
-wisdom,      it was the age of       foolishness, it was the epoch of belief, it
+wisdom, bro, it was the age of total foolishness, it was the epoch of belief, it
-was the epoch of incredulity,  it was the season of Light, it was the
+was the epoch of like, no way, it was the season of Light, it was the
-short,       the period was so far like the present period, that some of its
+short, dude, the period was so far like the present period, that some of its
-evil, in the superlative degree of comparison only.
+evil, in the superlative degree of comparison only. Gnarly.
-a fair face, on the throne of France.          In both countries it was clearer
+a fair face, on the throne of France, totally. In both countries it was clearer
-that things in general were settled for ever.
+that things in general were settled for ever, like, forever.
```

## Usage

```
bodge OLD NEW           compare two files
bodge PATCH             convert a unified diff (diff -u, git diff)
git diff | bodge -      same, from stdin
```

Output is printed and also copied to the clipboard with `pbcopy`. 
Pass `--display-only` to skip the clipboard.

## Build

```
cargo build --release    # binary at target/release/bodge
cargo test
```
