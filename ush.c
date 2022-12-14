#include <linux/fs.h>
#include <sys/stat.h>
#include <unistd.h>
#include <threads.h>

#include "misc.h"
#include "fds.h"
#include "prompt.h"

void ch_cat(char * const args[], PTY *ptys) {
    PTY *pty = args_to_pty(args, ptys);
    if(pty != NULL) {
        pty->out_bth->sleep = true;
        FlatBuff *buff = &(pty->out_bth->buff);
        write(STDOUT_FILENO, buff->data, buff->pos);
        buff->pos = 0;
        pty->out_bth->sleep = false;

        if(pty->out_bth->finished) {
            int status;
            waitpid(pty->pid, &status, 0);
            printf("process %d finished with status %d\n", pty->pid, status);
            *pty = NO_PTY;
        }
    }
}

void ch_fg(char *const args[], PTY *ptys) {
    PTY *pty = args_to_pty(args, ptys);
    if(pty != NULL) {
        int status = pty_foreground(pty);
        if(status != STOP_DEADMASK) {
            if(status != 0)
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
    pty_readkey();
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

