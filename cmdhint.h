#ifndef __H_CMDHINT
#define __H_CMDHINT

/* This unit provides interface for command
   autocompletion. */

#include "fds.h"
#include "misc.h"
#include "prompt.h"

#include <sys/types.h>
#include <sys/stat.h>
#include <dirent.h>

typedef enum _HintType {
    HT_CMD = 0x01, HT_DIR = 0x02, HT_EXEC = 0x04,
    HT_ANYFILE = 0x0c, HT_CUSTOMDIR = 0x10
} HintType;

typedef struct {
    char *path, *next_path;
    ConstStr current_hint, current_path;
    size_t hintpos;
    Hash prefix_hash;
    size_t prefix_len;
    char *path_cur;
    DIR *dh;
    const CHLine *builtins;
    size_t builtins_count, builtins_cur;
    StrList *hints;
    HintType ht_flags;
} CmdHint;

CmdHint new_cmdhint(const CHLine *builtins, size_t builtins_count);
ConstStr next_cmdhint(CmdHint *ch, const char *prefix);
ConstStr prev_cmdhint(CmdHint *ch);

#endif
