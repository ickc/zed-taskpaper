; Base styling. Later patterns take precedence in Zed, so the @done fading
; rules live at the bottom of this file.

(project name: (text) @title)
(project ":" @title)
(note (text) @comment)
(marker) @punctuation.special

(tag (tag_name) @tag)
(tag (tag_value) @string.special)
(tag ["(" ")"] @punctuation.bracket)

; --- @done fading --------------------------------------------------------
; Zed theme syntax styles have no strikethrough, so completed items are
; faded like comments instead. Tags are always a trailing run of siblings,
; so an item is done iff it has a (tag) child named "@done"; fade the
; marker, the text, the other tags, and the @done tag itself.

((task (marker) @comment.unused (tag name: (tag_name) @_done))
 (#eq? @_done "@done"))
((task (text) @comment.unused (tag name: (tag_name) @_done))
 (#eq? @_done "@done"))
((task (tag) @comment.unused (tag name: (tag_name) @_done))
 (#eq? @_done "@done"))
((task (tag name: (tag_name) @_done) (tag) @comment.unused)
 (#eq? @_done "@done"))

((project name: (text) @comment.unused (tag name: (tag_name) @_done))
 (#eq? @_done "@done"))
((project ":" @comment.unused (tag name: (tag_name) @_done))
 (#eq? @_done "@done"))
((project (tag) @comment.unused (tag name: (tag_name) @_done))
 (#eq? @_done "@done"))
((project (tag name: (tag_name) @_done) (tag) @comment.unused)
 (#eq? @_done "@done"))

((note (text) @comment.unused (tag name: (tag_name) @_done))
 (#eq? @_done "@done"))
((note (tag) @comment.unused (tag name: (tag_name) @_done))
 (#eq? @_done "@done"))
((note (tag name: (tag_name) @_done) (tag) @comment.unused)
 (#eq? @_done "@done"))

((tag name: (tag_name) @_done) @comment.unused
 (#eq? @_done "@done"))
