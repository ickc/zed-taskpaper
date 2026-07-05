// External scanner for tree-sitter-taskpaper.
//
// Responsibilities:
//   * INDENT / DEDENT tracking (python-style indent stack; tabs per the
//     TaskPaper spec, spaces tolerated, one column per character),
//   * _eol (consumes one line ending),
//   * line classification: _project_begin / _task_begin / _note_begin
//     markers (they absorb the leading indentation and any preceding blank
//     lines) emitted after looking ahead over the whole line,
//   * `text`: the body of a line, up to (but excluding) the trailing run of
//     @tags — and for projects, excluding the terminating ":" as well. The
//     boundary is computed during classification (the lexer cannot rewind,
//     so it is remembered in `pending` and consumed when the parser asks).
//
// Classification rules (following TaskPaper 3, but stricter about tags —
// only a trailing, whitespace-preceded run of @tag / @tag(value) counts):
//   task:    "-" followed by space, tab, or end of line
//   project: line ends with ":" once trailing whitespace and trailing
//            @tags are stripped
//   note:    anything else

#include "tree_sitter/parser.h"

#include <stdint.h>
#include <stdlib.h>

enum TokenType {
    INDENT,
    DEDENT,
    EOL,
    PROJECT_BEGIN,
    TASK_BEGIN,
    NOTE_BEGIN,
    TEXT,
    ERROR_SENTINEL,
};

#define MAX_DEPTH 128
// Lines longer than this are classified as notes/tasks by prefix only, with
// the whole remainder as text (the trailing-tag scan needs the line end).
#define MAX_LINE 4096
// `pending` sentinel: text runs to end of line.
#define PENDING_TO_EOL 0xFFFFu

typedef struct {
    uint32_t stack[MAX_DEPTH]; // stack[0] == 0 always
    uint32_t depth;            // number of valid entries, >= 1
    uint32_t dedents;          // dedents still owed to the parser
    uint32_t pending;          // text length for the current line's body
    uint32_t start_col;        // column where the current line's content starts
} Scanner;

static inline void advance(TSLexer *lexer) { lexer->advance(lexer, false); }

static bool is_inline_ws(int32_t c) { return c == ' ' || c == '\t'; }

// Must stay in sync with the `tag_name` regex in grammar.js:
// ASCII alphanumerics, "_", ".", "-", and any non-ASCII codepoint.
static bool is_tag_name_char(int32_t c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
           (c >= '0' && c <= '9') || c == '_' || c == '.' || c == '-' ||
           c > 127;
}

// Index just past the line's body once trailing whitespace and the trailing
// run of @tags (each optionally with a "(value)") is stripped. A tag must be
// preceded by whitespace or start the line.
static uint32_t body_end_without_tags(const int32_t *buf, uint32_t len) {
    int64_t end = len;
    while (end > 0 && is_inline_ws(buf[end - 1])) end--;
    for (;;) {
        int64_t e = end;
        if (e > 0 && buf[e - 1] == ')') {
            int64_t p = e - 2;
            while (p >= 0 && buf[p] != '(' && buf[p] != ')') p--;
            if (p < 0 || buf[p] != '(') break;
            e = p;
        }
        int64_t j = e;
        while (j > 0 && is_tag_name_char(buf[j - 1])) j--;
        if (j == e || j == 0) break; // no name chars, or no room for '@'
        if (buf[j - 1] != '@') break;
        if (j - 1 > 0 && !is_inline_ws(buf[j - 2])) break;
        end = j - 1;
        while (end > 0 && is_inline_ws(buf[end - 1])) end--;
    }
    return (uint32_t)end;
}

static bool scan(Scanner *s, TSLexer *lexer, const bool *valid) {
    // In error recovery every token is marked valid; bail out and let the
    // internal lexer handle it.
    if (valid[ERROR_SENTINEL]) return false;

    if (s->dedents > 0 && valid[DEDENT]) {
        s->dedents--;
        lexer->mark_end(lexer);
        lexer->result_symbol = DEDENT;
        return true;
    }

    if (valid[EOL] &&
        (lexer->lookahead == '\r' || lexer->lookahead == '\n' || lexer->eof(lexer))) {
        lexer->mark_end(lexer);
        if (lexer->lookahead == '\r') advance(lexer);
        if (lexer->lookahead == '\n') advance(lexer);
        lexer->mark_end(lexer);
        lexer->result_symbol = EOL;
        return true;
    }

    if (valid[TEXT]) {
        lexer->mark_end(lexer);
        uint32_t n;
        if (s->pending == PENDING_TO_EOL) {
            n = UINT32_MAX; // consume to end of line
        } else {
            // Characters of this line's content already consumed by other
            // tokens (the task marker), measured by column.
            uint32_t col = lexer->get_column(lexer);
            if (col < s->start_col) return false;
            uint32_t already = col - s->start_col;
            if (s->pending <= already) return false;
            n = s->pending - already;
        }
        uint32_t taken = 0;
        while (taken < n && !lexer->eof(lexer) && lexer->lookahead != '\n' &&
               lexer->lookahead != '\r') {
            advance(lexer);
            taken++;
        }
        if (taken == 0) return false;
        lexer->mark_end(lexer);
        lexer->result_symbol = TEXT;
        return true;
    }

    bool line_start = valid[INDENT] || valid[DEDENT] || valid[PROJECT_BEGIN] ||
                      valid[TASK_BEGIN] || valid[NOTE_BEGIN];
    if (!line_start) return false;

    // Everything below starts as pure lookahead; mark_end() is only moved
    // forward when a token that should own the scanned text is emitted.
    lexer->mark_end(lexer);

    // Measure the indentation of the next non-blank line, skipping blank
    // (whitespace-only) lines entirely. The scanner may be re-entered after
    // an INDENT token already consumed this line's whitespace, so the
    // current column is the baseline.
    uint32_t indent = lexer->get_column(lexer);
    for (;;) {
        int32_t c = lexer->lookahead;
        if (c == ' ' || c == '\t') {
            indent++;
            advance(lexer);
        } else if (c == '\r') {
            advance(lexer);
        } else if (c == '\n') {
            indent = 0; // blank line
            advance(lexer);
        } else {
            break;
        }
    }

    uint32_t cur = s->stack[s->depth - 1];

    if (lexer->eof(lexer)) {
        if (s->depth > 1 && valid[DEDENT]) {
            s->depth--;
            lexer->result_symbol = DEDENT;
            return true;
        }
        return false; // trailing whitespace is absorbed by `extras`
    }

    if (indent > cur) {
        if (valid[INDENT] && s->depth < MAX_DEPTH) {
            s->stack[s->depth++] = indent;
            lexer->mark_end(lexer); // INDENT owns the whitespace
            lexer->result_symbol = INDENT;
            return true;
        }
        // INDENT not valid here (e.g. over-indented first line): fall
        // through and treat the line as being at the current level.
    } else if (indent < cur && valid[DEDENT]) {
        // Only mutate the stack when DEDENT can actually be emitted.
        uint32_t n = 0;
        while (s->depth > 1 && s->stack[s->depth - 1] > indent) {
            s->depth--;
            n++;
        }
        // If `indent` matches no level on the stack, we stop at the nearest
        // shallower level and let the line live there.
        if (n > 0) {
            s->dedents = n - 1;
            lexer->result_symbol = DEDENT; // zero-width
            return true;
        }
    }

    // Same level: classify the line. The marker token owns the leading
    // whitespace/blank lines consumed above.
    lexer->mark_end(lexer);
    s->start_col = lexer->get_column(lexer);

    // Buffer the line (lookahead only; nothing past mark_end is consumed).
    int32_t buf[MAX_LINE];
    uint32_t len = 0;
    bool truncated = false;
    while (!lexer->eof(lexer) && lexer->lookahead != '\n' && lexer->lookahead != '\r') {
        if (len < MAX_LINE) {
            buf[len++] = lexer->lookahead;
        } else {
            truncated = true;
            break;
        }
        advance(lexer);
    }

    enum TokenType kind;
    if (len > 0 && buf[0] == '-' && (len == 1 || is_inline_ws(buf[1]))) {
        kind = TASK_BEGIN;
        s->pending = truncated ? PENDING_TO_EOL : body_end_without_tags(buf, len);
    } else if (truncated) {
        kind = NOTE_BEGIN;
        s->pending = PENDING_TO_EOL;
    } else {
        uint32_t body_end = body_end_without_tags(buf, len);
        if (body_end > 0 && buf[body_end - 1] == ':') {
            kind = PROJECT_BEGIN;
            s->pending = body_end - 1; // name excludes the ":"
        } else {
            kind = NOTE_BEGIN;
            s->pending = body_end;
        }
    }
    if (!valid[kind]) return false;
    lexer->result_symbol = (unsigned)kind;
    return true;
}

void *tree_sitter_taskpaper_external_scanner_create(void) {
    Scanner *s = calloc(1, sizeof(Scanner));
    s->depth = 1;
    return s;
}

void tree_sitter_taskpaper_external_scanner_destroy(void *payload) {
    free(payload);
}

unsigned tree_sitter_taskpaper_external_scanner_serialize(void *payload, char *buffer) {
    Scanner *s = payload;
    unsigned size = 0;
    buffer[size++] = (char)(s->dedents > 255 ? 255 : s->dedents);
    uint32_t pending = s->pending > 0xFFFF ? 0xFFFF : s->pending;
    buffer[size++] = (char)(pending & 0xFF);
    buffer[size++] = (char)(pending >> 8);
    uint32_t start_col = s->start_col > 0xFFFF ? 0xFFFF : s->start_col;
    buffer[size++] = (char)(start_col & 0xFF);
    buffer[size++] = (char)(start_col >> 8);
    for (uint32_t i = 0; i < s->depth && size < TREE_SITTER_SERIALIZATION_BUFFER_SIZE; i++) {
        buffer[size++] = (char)(s->stack[i] > 255 ? 255 : s->stack[i]);
    }
    return size;
}

void tree_sitter_taskpaper_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
    Scanner *s = payload;
    s->depth = 1;
    s->stack[0] = 0;
    s->dedents = 0;
    s->pending = 0;
    s->start_col = 0;
    if (length < 5) return;
    s->dedents = (uint8_t)buffer[0];
    s->pending = (uint32_t)(uint8_t)buffer[1] | ((uint32_t)(uint8_t)buffer[2] << 8);
    s->start_col = (uint32_t)(uint8_t)buffer[3] | ((uint32_t)(uint8_t)buffer[4] << 8);
    s->depth = 0;
    for (unsigned i = 5; i < length && s->depth < MAX_DEPTH; i++) {
        s->stack[s->depth++] = (uint8_t)buffer[i];
    }
    if (s->depth == 0) {
        s->depth = 1;
        s->stack[0] = 0;
    }
}

bool tree_sitter_taskpaper_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid) {
    return scan(payload, lexer, valid);
}
