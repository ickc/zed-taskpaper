; A run (▶) button on every project heading. Zed exposes the name as
; $ZED_CUSTOM_project_name and the button's position as $ZED_ROW to any
; task tagged `taskpaper-project` (see README: scripts/taskpaper_count.py
; counts the open/done/cancelled tasks in the project's subtree).

(
  (project name: (text) @run @project_name)
  (#set! tag taskpaper-project)
)

(
  (dim_project name: (text) @run @project_name)
  (#set! tag taskpaper-project)
)
