; Base styling. Later patterns take precedence in Zed, so the @done fading
; rules live at the bottom of this file.

(project (text) @title)
(note (text) @comment)
(marker) @punctuation.special

(tag (tag_name) @tag)
(tag (tag_value) @string.special)
(tag ["(" ")"] @punctuation.bracket)

; --- @done styling ------------------------------------------------------
; Zed theme syntax styles have no strikethrough, so completed items are
; faded like comments instead. Text both before and after the @done tag is
; covered, as is the tag itself.

((task (text) @comment.unused (tag (tag_name) @_done)) (#eq? @_done "@done"))
((task (tag (tag_name) @_done) (text) @comment.unused) (#eq? @_done "@done"))
((task (marker) @comment.unused (tag (tag_name) @_done)) (#eq? @_done "@done"))
((project (text) @comment.unused (tag (tag_name) @_done)) (#eq? @_done "@done"))
((project (tag (tag_name) @_done) (text) @comment.unused) (#eq? @_done "@done"))
((note (text) @comment.unused (tag (tag_name) @_done)) (#eq? @_done "@done"))
((note (tag (tag_name) @_done) (text) @comment.unused) (#eq? @_done "@done"))

((tag_name) @comment.unused (#eq? @comment.unused "@done"))
