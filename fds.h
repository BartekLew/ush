#ifndef __H_FDS
#define __H_FDS 1

#include "misc.h"

#include <fcntl.h>
#include <unistd.h>
#include <sys/types.h>
#include <termios.h>
#include <sys/ioctl.h>
#include <sys/select.h>

typedef struct PTY
{
    int   master, slave;
    pid_t pid;
    bool  suspended;
} PTY;

#define NO_PTY (PTY){-1,-1,-1, 0}
#define PTY_ISOK(PTY) (PTY.master > 0 && PTY.slave > 0)

#define MAX_PTYS 10

PTY newPty();
bool spawn(char *const args[], PTY *ptys);
bool reprint (PTY *pty);

typedef bool (*RewriteFilter)(const char *buff, int size);
int rewrite_fds(uint count, int *ifds, int *ofds, int waitmask, RewriteFilter filter);

#endif
