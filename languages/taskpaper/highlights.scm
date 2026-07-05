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

(project name: (text) @title)
(project ":" @title)
(note (text) @comment)
(marker) @punctuation.special

(tag (tag_name) @tag)
(tag (tag_value) @string.special)
(tag ["(" ")"] @punctuation.bracket)

; --- @done / @cancelled wash ---------------------------------------------
; Zed theme syntax styles have no strikethrough, so completed/cancelled
; items are faded into the @predictive ghost style. Tags are always a
; trailing run of siblings, so "this item is done/cancelled" is simply
; "has such a (tag) child"; child items always come after the tags, which
; is what makes the descendant patterns below safe.

; The item's own line: bullet, body text, project colon, and any other
; tags before/after the @done/@cancelled tag.
((_ (marker) @predictive (tag (tag_name) @_t))
 (#match? @_t "^@(done|cancelled)$"))
((_ (text) @predictive (tag (tag_name) @_t))
 (#match? @_t "^@(done|cancelled)$"))
((project ":" @predictive (tag (tag_name) @_t))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @predictive) (tag (tag_name) @_t))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_value) @predictive) (tag (tag_name) @_t))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag ["(" ")"] @predictive) (tag (tag_name) @_t))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (tag (tag_name) @predictive))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (tag (tag_value) @predictive))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (tag ["(" ")"] @predictive))
 (#match? @_t "^@(done|cancelled)$"))

; Descendants: a @done/@cancelled project or task fades its whole subtree.
; Queries cannot recurse, so this is spelled out for 1-6 levels of nesting
; below the tagged item.

; depth 1
((_ (tag (tag_name) @_t) (_ (marker) @predictive))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (text) @predictive))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (project ":" @predictive))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (tag (tag_name) @predictive)))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (tag (tag_value) @predictive)))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (tag ["(" ")"] @predictive)))
 (#match? @_t "^@(done|cancelled)$"))

; depth 2
((_ (tag (tag_name) @_t) (_ (_ (marker) @predictive)))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (text) @predictive)))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (project ":" @predictive)))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (tag (tag_name) @predictive))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (tag (tag_value) @predictive))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (tag ["(" ")"] @predictive))))
 (#match? @_t "^@(done|cancelled)$"))

; depth 3
((_ (tag (tag_name) @_t) (_ (_ (_ (marker) @predictive))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (text) @predictive))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (project ":" @predictive))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (tag (tag_name) @predictive)))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (tag (tag_value) @predictive)))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (tag ["(" ")"] @predictive)))))
 (#match? @_t "^@(done|cancelled)$"))

; depth 4
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (marker) @predictive)))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (text) @predictive)))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (project ":" @predictive)))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (tag (tag_name) @predictive))))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (tag (tag_value) @predictive))))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (tag ["(" ")"] @predictive))))))
 (#match? @_t "^@(done|cancelled)$"))

; depth 5
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (_ (marker) @predictive))))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (_ (text) @predictive))))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (project ":" @predictive))))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (_ (tag (tag_name) @predictive)))))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (_ (tag (tag_value) @predictive)))))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (_ (tag ["(" ")"] @predictive)))))))
 (#match? @_t "^@(done|cancelled)$"))

; depth 6
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (_ (_ (marker) @predictive)))))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (_ (_ (text) @predictive)))))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (_ (project ":" @predictive)))))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (_ (_ (tag (tag_name) @predictive))))))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (_ (_ (tag (tag_value) @predictive))))))))
 (#match? @_t "^@(done|cancelled)$"))
((_ (tag (tag_name) @_t) (_ (_ (_ (_ (_ (_ (tag ["(" ")"] @predictive))))))))
 (#match? @_t "^@(done|cancelled)$"))

; --- state-tag accents (last, so they survive the wash) -------------------
((tag (tag_name) @hint) (#eq? @hint "@done"))
((tag (tag_name) @string.special) (#eq? @string.special "@cancelled"))
