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
; IMPORTANT: Zed runs highlight queries with a query-cursor match limit of
; 64 (syntax_map.rs); too many concurrently-matching patterns silently drop
; matches. That is why the wash below is packed into few patterns via
; alternations instead of one pattern per leaf kind.

(project name: (text) @title)
(project ":" @title)
(note (text) @comment)
(marker) @punctuation.special

(tag (tag_name) @tag)
(tag (tag_value) @string.special)
(tag ["(" ")"] @punctuation.bracket)

; --- @done / @cancelled wash ---------------------------------------------
; Zed theme syntax styles have no strikethrough, so completed/cancelled
; items fade into the @predictive ghost style. Tags are always a trailing
; run of siblings, so "this item is done/cancelled" is simply "has such a
; (tag) child", and child items always come after the tags. Captures must
; target leaf nodes (inner captures beat outer ones in Zed).

; The item's own line, before the state tag: bullet, body text, project
; colon, and any earlier tags.
((_ [
     (marker) @predictive
     (text) @predictive
     ":" @predictive
     (tag [(tag_name) (tag_value) "(" ")"] @predictive)
    ]
    (tag (tag_name) @_t))
 (#match? @_t "^@(done|cancelled)$"))

; Tags after the state tag.
((_ (tag (tag_name) @_t)
    (tag [(tag_name) (tag_value) "(" ")"] @predictive))
 (#match? @_t "^@(done|cancelled)$"))

; Descendants: a @done/@cancelled item fades its whole subtree. Queries
; cannot recurse, so this is spelled out per depth, 1-6 levels below the
; tagged item — one pattern per depth.

((_ (tag (tag_name) @_t)
    (_ [
        (marker) @predictive
        (text) @predictive
        ":" @predictive
        (tag [(tag_name) (tag_value) "(" ")"] @predictive)
       ]))
 (#match? @_t "^@(done|cancelled)$"))

((_ (tag (tag_name) @_t)
    (_ (_ [
        (marker) @predictive
        (text) @predictive
        ":" @predictive
        (tag [(tag_name) (tag_value) "(" ")"] @predictive)
       ])))
 (#match? @_t "^@(done|cancelled)$"))

((_ (tag (tag_name) @_t)
    (_ (_ (_ [
        (marker) @predictive
        (text) @predictive
        ":" @predictive
        (tag [(tag_name) (tag_value) "(" ")"] @predictive)
       ]))))
 (#match? @_t "^@(done|cancelled)$"))

((_ (tag (tag_name) @_t)
    (_ (_ (_ (_ [
        (marker) @predictive
        (text) @predictive
        ":" @predictive
        (tag [(tag_name) (tag_value) "(" ")"] @predictive)
       ])))))
 (#match? @_t "^@(done|cancelled)$"))

((_ (tag (tag_name) @_t)
    (_ (_ (_ (_ (_ [
        (marker) @predictive
        (text) @predictive
        ":" @predictive
        (tag [(tag_name) (tag_value) "(" ")"] @predictive)
       ]))))))
 (#match? @_t "^@(done|cancelled)$"))

((_ (tag (tag_name) @_t)
    (_ (_ (_ (_ (_ (_ [
        (marker) @predictive
        (text) @predictive
        ":" @predictive
        (tag [(tag_name) (tag_value) "(" ")"] @predictive)
       ])))))))
 (#match? @_t "^@(done|cancelled)$"))

; --- state-tag accents (last, so they survive the wash) -------------------
((tag (tag_name) @hint) (#eq? @hint "@done"))
((tag (tag_name) @string.special) (#eq? @string.special "@cancelled"))
