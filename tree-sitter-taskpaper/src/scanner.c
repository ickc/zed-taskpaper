// External scanner for tree-sitter-taskpaper.
//
// Responsibilities:
//   * INDENT / DEDENT tracking (python-style indent stack; tabs per the
//     TaskPaper spec, spaces tolerated, one column per character),
//   * _eol (consumes one line ending),
//   * line classification: _project_begin / _task_begin / _note_begin are
//     zero-width-content markers (they absorb the leading indentation and
//     any preceding blank lines) emitted after looking ahead over the line.
//
// Classification rules (matching TaskPaper 3):
//   task:    "-" followed by space, tab, or end of line
//   project: last non-whitespace run of the line, after stripping trailing
//            @tags (with optional "(value)"), ends with ":"
//   note:    anything else

#include "tree_sitter/parser.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <wctype.h>

enum TokenType {
    INDENT,
    DEDENT,
    EOL,
    PROJECT_BEGIN,
    TASK_BEGIN,
    NOTE_BEGIN,
    ERROR_SENTINEL,
};

#define MAX_DEPTH 128
// Lines longer than this are classified as notes/tasks only by their prefix;
// the trailing-colon check needs the end of the line. Generous for real docs.
#define MAX_LINE 4096

typedef struct {
    uint32_t stack[MAX_DEPTH]; // stack[0] == 0 always
    uint32_t depth;            // number of valid entries, >= 1
    uint32_t dedents;          // dedents still owed to the parser
} Scanner;

static inline void advance(TSLexer *lexer) { lexer->advance(lexer, false); }

static bool is_tag_name_char(int32_t c) {
    return iswalnum((wint_t)c) || c == '_' || c == '.' || c == '-';
}

static bool is_inline_ws(int32_t c) { return c == ' ' || c == '\t'; }

// True if the line ends with ":" once trailing whitespace and trailing
// @tags (optionally with a parenthesized value) are stripped.
static bool line_is_project(const int32_t *buf, uint32_t len) {
    int64_t i = (int64_t)len - 1;
    for (;;) {
        while (i >= 0 && is_inline_ws(buf[i])) i--;
        if (i < 0) return false;
        if (buf[i] == ':') return true;
        // Try to strip one trailing tag: @name or @name(value).
        if (buf[i] == ')') {
            int64_t j = i - 1;
            while (j >= 0 && buf[j] != '(' && buf[j] != ')') j--;
            if (j < 0 || buf[j] != '(') return false;
            i = j - 1;
        }
        int64_t j = i;
        while (j >= 0 && is_tag_name_char(buf[j])) j--;
        if (j < 0 || j == i || buf[j] != '@') return false;
        if (j > 0 && !is_inline_ws(buf[j - 1])) return false;
        i = j - 1;
    }
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

    if (valid[EOL]) {
        lexer->mark_end(lexer); // zero-width unless we consume a newline
        if (lexer->lookahead == '\r') advance(lexer);
        if (lexer->lookahead == '\n') {
            advance(lexer);
            lexer->mark_end(lexer);
            lexer->result_symbol = EOL;
            return true;
        }
        if (lexer->eof(lexer)) {
            lexer->result_symbol = EOL;
            return true;
        }
        return false;
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
    } else if (!truncated && line_is_project(buf, len)) {
        kind = PROJECT_BEGIN;
    } else {
        kind = NOTE_BEGIN;
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
    uint32_t depth = s->depth;
    if (depth > TREE_SITTER_SERIALIZATION_BUFFER_SIZE - 1) {
        depth = TREE_SITTER_SERIALIZATION_BUFFER_SIZE - 1;
    }
    for (uint32_t i = 0; i < depth && size < TREE_SITTER_SERIALIZATION_BUFFER_SIZE; i++) {
        buffer[size++] = (char)(s->stack[i] > 255 ? 255 : s->stack[i]);
    }
    return size;
}

void tree_sitter_taskpaper_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
    Scanner *s = payload;
    s->depth = 1;
    s->stack[0] = 0;
    s->dedents = 0;
    if (length == 0) return;
    s->dedents = (uint8_t)buffer[0];
    s->depth = 0;
    for (unsigned i = 1; i < length && s->depth < MAX_DEPTH; i++) {
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
