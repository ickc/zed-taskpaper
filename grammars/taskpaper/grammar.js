/**
 * TaskPaper grammar for tree-sitter.
 *
 * The line model follows TaskPaper 3:
 *   - task:    a line whose text begins with "- "
 *   - project: a non-task line that ends with ":" (optionally followed by
 *              trailing @tags)
 *   - note:    any other non-blank line
 * Indentation (tabs, per the TaskPaper spec; spaces tolerated) nests items
 * under the item above. Line classification and INDENT/DEDENT tracking live
 * in the external scanner (src/scanner.c), so the grammar itself is LR(1)
 * with no conflicts.
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
    $._error_sentinel,
  ],

  // Whitespace is structurally significant and is consumed by the external
  // scanner at line boundaries; this extra only mops up otherwise-unclaimed
  // whitespace (blank-only files, spaces between inline tokens).
  extras: $ => [/\s/],

  rules: {
    document: $ => repeat($._item),

    _item: $ => choice($.project, $.task, $.note),

    project: $ => seq(
      $._project_begin,
      repeat1($._inline),
      $._eol,
      optional($._children),
    ),

    task: $ => seq(
      $._task_begin,
      $.marker,
      repeat($._inline),
      $._eol,
      optional($._children),
    ),

    note: $ => seq(
      $._note_begin,
      repeat1($._inline),
      $._eol,
      optional($._children),
    ),

    _children: $ => seq($._indent, repeat1($._item), $._dedent),

    _inline: $ => choice($.tag, $.text),

    // The task bullet, including its following space/tab.
    marker: $ => token(/-[ \t]?/),

    // Plain run of line text. A lone "@" that does not start a valid tag
    // falls through to text as well.
    text: $ => token(prec(-1, /[^@\r\n]+|@/)),

    // @name or @name(value). Values may not contain ")" or newlines.
    tag: $ => seq(
      field('name', alias(token(/@[\p{L}\p{N}_.\-]+/), $.tag_name)),
      optional(seq(
        token.immediate('('),
        optional(field('value', alias(token.immediate(/[^)\r\n]*/), $.tag_value))),
        token.immediate(')'),
      )),
    ),
  },
});
