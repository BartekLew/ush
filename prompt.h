#ifndef __H_PROMPT
#define __H_PROMPT

#include "misc.h"
#include "fds.h"

#include <sys/wait.h>

extern volatile bool has_sigstop;

typedef void (*CommandHandler) (char * const args[], PTY *ptys);
typedef struct {
    const char     *cmd;
    CommandHandler handler;
} CHLine;

PTY *args_to_pty (char *const args[], PTY *ptys);

#define STOP_DEADMASK (int)0xffffffff
int pty_foreground(PTY *pty);
int pty_readkey();

void prompt(const CHLine *handlers, size_t h_size);
void reset_tty();

#endif
