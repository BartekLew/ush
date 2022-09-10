#include <linux/fs.h>
#include <sys/stat.h>
#include <unistd.h>
#include <threads.h>

#include "misc.h"
#include "fds.h"
#include "prompt.h"

void ch_cat(char * const args[], PTY *ptys) {
    PTY *pty = args_to_pty(args, ptys);
    if(pty != NULL)
        reprint(pty);
}

void ch_fg(char *const args[], PTY *ptys) {
    PTY *pty = args_to_pty(args, ptys);
    if(pty != NULL) {
        int status = term_sandbox((TsbFun)&pty_foreground, pty);
        if(status != STOP_DEADMASK) {
            printf("pid #%d finished with status %d\n", (int)pty->pid, status);
            *pty = NO_PTY;
        } else {
            kill(pty->pid, SIGSTOP);
            pty->suspended = true;
            printf("pid %d suspended\n", (int)pty->pid);
        }
    }
}

void ch_readkey(char *const args[], PTY *ptys) {
    UNUSED(args);
    UNUSED(ptys);
    term_sandbox((TsbFun)&pty_readkey, NULL);
}

CHLine handlers[] = {
    {".cat", &ch_cat},
    {".fg", &ch_fg},
    {".readkey", &ch_readkey}
};

void sigh_tstp(int signo) {
    UNUSED(signo);
    has_sigstop = true;
}

int main() {
    struct sigaction act = { .sa_handler = &sigh_tstp };
    sigemptyset(&act.sa_mask);
    sigaction(SIGTSTP, &act, NULL);

    prompt(handlers, sizeof(handlers)/sizeof(CHLine));

    return 0;
}

