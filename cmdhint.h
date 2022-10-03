#ifndef __H_CMDHINT
#define __H_CMDHINT

/* This unit provides interface for command
   autocompletion. */

#include "fds.h"
#include "misc.h"
#include "prompt.h"

#include <sys/types.h>
#include <dirent.h>

typedef uint64_t Hash;

typedef struct {
    char *path, *next_path;
    ConstStr current_hint;
    Hash prefix_hash;
    size_t prefix_len;
    char *path_cur;
    DIR *dh;
    const CHLine *builtins;
    size_t builtins_count, builtins_cur;
} CmdHint;

CmdHint new_cmdhint(const CHLine *builtins, size_t builtins_count);
ConstStr next_cmdhint(CmdHint *ch, const char *prefix);

#endif
