#ifndef __H_MISC
#define __H_MISC 1

#define _XOPEN_SOURCE 600
#define _DEFAULT_SOURCE 1
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>
#include <signal.h>
#include <ctype.h>

#define UNUSED(x) (void)x

#define CTRL_D 0x04
#define CTRL_X 0x18
#define ESC 0x1b
#define IN_BACKSPACE 0x7f
#define ERRNO_SIGCAUGHT 0x04
#define UP_ARROW 0x00415b1b
#define DOWN_ARROW 0x00425b1b
#define RIGHT_ARROW 0x00435b1b
#define LEFT_ARROW 0x00445b1b

typedef unsigned int uint;

#define BUFF_SIZE 1024
#define MAX_ARGS 1024
#define MAX_CMDHINTS 4000
#define CMD_NAMEBUFF_LEN 40000

typedef struct {
    const char *str;
    size_t len;
} ConstStr;

#define nostr (ConstStr) {NULL,0}
int ConstStr_pathcmp(const void* a, const void*b);
bool writestr(int fd, ConstStr str);

typedef struct {
    char     *charbuff;
    ConstStr *strbuff;
    size_t   charlen, strlen;
    size_t    charpos, strpos;
} StrList;

#define STATIC_STRLIST(NAME, CHARLEN, STRLEN) \
    static char NAME##_charbuff[CHARLEN+1];\
    static ConstStr NAME##_strbuff[STRLEN]; \
    static StrList NAME = (StrList) {\
                .charbuff = NAME##_charbuff,\
                .strbuff = NAME##_strbuff, \
                .charlen = CHARLEN, .strlen = STRLEN,\
                .charpos = 0, .strpos = 0\
            }

bool pushstr (StrList *tgt, ConstStr str); 
void resetlist (StrList *tgt);
void uniq(StrList *tgt);

typedef uint64_t Hash;
Hash hashof(const char *txt, size_t len);
Hash hashofstr(ConstStr s);

#endif
