; Projects only, like Markdown headings: the outline shows the project
; name (colon and trailing tags excluded), nested as in the document.
; dim_project is a project under a @done/@cancelled ancestor.

(project
  name: (text) @name) @item

(dim_project
  name: (text) @name) @item
