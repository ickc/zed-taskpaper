; Base styling. Later patterns take precedence in Zed, so the ordering in
; this file is: base colors, then the @done/@cancelled "wash", then the
; accents that must survive the wash.
;
; De-emphasis palette (three distinct kinds of "recede"):
;   notes            -> @comment     (theme comment gray)
;   @done/@cancelled -> @predictive  (ghost-text style: dimmer + italic)
;   @done tag        -> @hint        (legible muted blue: "finished")
;   @cancelled tag   -> @string.special (muted amber: "stopped")
; The task bullet doubles as a checkbox-ish signal: @punctuation.special
; (an accent color) while open, ghost-faded once done/cancelled.
;
; NOTE: Zed runs highlight queries with a query-cursor match limit of 64;
; deep wildcard patterns silently drop matches. Subtree fading is therefore
; not done here at all: the scanner tracks done-ness per indent level and
; parses items under a @done/@cancelled ancestor as dim_* nodes, so every
; pattern in this file is flat and cheap, at unlimited nesting depth.

(project name: (text) @title)
(project ":" @title)
(note (text) @comment)
(marker) @punctuation.special

(tag (tag_name) @tag)
(tag (tag_value) @string.special)
(tag ["(" ")"] @punctuation.bracket)

; --- @done / @cancelled wash ---------------------------------------------
; Zed theme syntax styles have no strikethrough, so completed/cancelled
; items fade into the @predictive ghost style. Captures target leaf nodes
; (inner captures beat outer ones in Zed).

; Items under a @done/@cancelled ancestor: the scanner already did the
; hard work, these nodes just need painting.
(dim_project [
  (text) @predictive
  ":" @predictive
  (tag [(tag_name) (tag_value) "(" ")"] @predictive)
])
(dim_task [
  (marker) @predictive
  (text) @predictive
  (tag [(tag_name) (tag_value) "(" ")"] @predictive)
])
(dim_note [
  (text) @predictive
  (tag [(tag_name) (tag_value) "(" ")"] @predictive)
])

; The tagged item's own line. Tags are always a trailing run of siblings,
; so "this item is done/cancelled" is "has such a (tag) child"; the two
; patterns cover material before and after the state tag.
((_ [
     (marker) @predictive
     (text) @predictive
     ":" @predictive
     (tag [(tag_name) (tag_value) "(" ")"] @predictive)
    ]
    (tag (tag_name) @_t))
 (#match? @_t "^@(done|cancelled)$"))

((_ (tag (tag_name) @_t)
    (tag [(tag_name) (tag_value) "(" ")"] @predictive))
 (#match? @_t "^@(done|cancelled)$"))

; --- state-tag accents (last, so they survive the wash) -------------------
((tag (tag_name) @hint) (#eq? @hint "@done"))
((tag (tag_name) @string.special) (#eq? @string.special "@cancelled"))
