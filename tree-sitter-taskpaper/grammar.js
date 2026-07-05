/**
 * TaskPaper grammar for tree-sitter.
 *
 * The line model follows TaskPaper 3:
 *   - task:    a line whose text begins with "- "
 *   - project: a non-task line that ends with ":" (optionally followed by
 *              trailing @tags)
 *   - note:    any other non-blank line
 * Tags are recognized only as a trailing, whitespace-preceded run of
 * @name / @name(value) at the end of a line — "user@example.com" mid-text
 * is never a tag. Indentation (tabs, per the TaskPaper spec; spaces
 * tolerated) nests items under the item above.
 *
 * Line classification, INDENT/DEDENT tracking, and the text/tag boundary
 * all live in the external scanner (src/scanner.c), so the grammar itself
 * is LR(1) with no conflicts.
 */

module.exports = grammar({
  name: 'taskpaper',

  externals: $ => [
    $._indent,
    $._dedent,
    $._eol,
    $._project_begin,
    $._task_begin,
    $._note_begin,
    $.text,
    $._error_sentinel,
  ],

  // Whitespace is structurally significant and is consumed by the external
  // scanner at line boundaries; this extra only mops up otherwise-unclaimed
  // whitespace (blank-only files, the gap between body text and tags).
  extras: $ => [/\s/],

  rules: {
    document: $ => repeat($._item),

    _item: $ => choice($.project, $.task, $.note),

    project: $ => seq(
      $._project_begin,
      optional(field('name', $.text)),
      ':',
      repeat($.tag),
      $._eol,
      optional($._children),
    ),

    task: $ => seq(
      $._task_begin,
      $.marker,
      optional($.text),
      repeat($.tag),
      $._eol,
      optional($._children),
    ),

    note: $ => seq(
      $._note_begin,
      optional($.text),
      repeat($.tag),
      $._eol,
      optional($._children),
    ),

    _children: $ => seq($._indent, repeat1($._item), $._dedent),

    // The task bullet, including its following space/tab.
    marker: $ => token(/-[ \t]?/),

    // @name or @name(value). Values may not contain ")" or newlines. The
    // name charset must stay in sync with is_tag_name_char in scanner.c.
    tag: $ => seq(
      field('name', alias(token(/@([A-Za-z0-9_.\-]|[^\x00-\x7F])+/), $.tag_name)),
      optional(seq(
        token.immediate('('),
        optional(field('value', alias(token.immediate(/[^)\r\n]*/), $.tag_value))),
        token.immediate(')'),
      )),
    ),
  },
});
