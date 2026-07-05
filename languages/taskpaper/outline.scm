; Projects and tasks appear in the outline panel; the item hierarchy comes
; from the grammar's nesting, so the panel mirrors the document tree.
; Tags are included as dimmed context, which makes them searchable in the
; outline panel's filter box — filtering there keeps ancestors visible,
; giving "filter without losing context".

(project
  (text) @name
  (tag)* @context) @item

(task
  (marker) @context
  (text)? @name
  (tag)* @context) @item
