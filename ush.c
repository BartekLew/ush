#define _XOPEN_SOURCE 600
#include <ctype.h>
#include <fcntl.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <linux/fs.h>
#include <sys/select.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <termios.h>
#include <signal.h>
#include <unistd.h>
#include <threads.h>

typedef struct PTY
{
    int   master, slave;
    pid_t pid;
} PTY;

#define NO_PTY (PTY){-1,-1,-1}
#define PTY_ISOK(PTY) (PTY.master > 0 && PTY.slave > 0)

const size_t MAX_PTYS = 10;
const size_t BUFF_SIZE = 1024;
const size_t MAX_ARGS = 1024;

typedef void (*CommandHandler) (char * const args[], PTY *ptys);
typedef struct {
    const char     *cmd;
    CommandHandler handler;
} CHLine;

typedef unsigned int uint;

PTY newPty() {
    PTY ans;
    ans.master = posix_openpt(O_RDWR | O_NOCTTY);
    if(ans.master == -1 || grantpt(ans.master) == -1 || unlockpt(ans.master) == -1) {
        perror("openpt");
        exit(1);
    }

    char *ptsn = ptsname(ans.master);
    if(ptsn == NULL) {
        perror("ptsname");
        exit(1);
    }
    ans.slave = open(ptsn, O_RDWR);
    if(ans.slave == -1) {
        perror("open(pts_slave)");
        exit(1);
    }

    return ans;
}

bool spawn(char *const args[], PTY *ptys)
{
    int ptyi = -1;
    for(size_t i = 0; i < MAX_PTYS; i++) {
        if(!PTY_ISOK(ptys[i])) {
            ptyi = i;
            break;
        }
    }

    if(ptyi == -1) {
        fprintf(stderr, "Can't spawn more ptys :(\n");
        exit(1);
    }

    PTY *pty = ptys + ptyi;
    *pty = newPty();

    pid_t p = fork();
    if (p == 0)
    {
        close(pty->master);

        setsid();
        if (ioctl(pty->slave, TIOCSCTTY, NULL) == -1)
        {
            perror("ioctl(TIOCSCTTY)");
            return false;
        }

        dup2(pty->slave, 0);
        dup2(pty->slave, 1);
        dup2(pty->slave, 2);
        close(pty->slave);

        execvp(args[0], args);
        return true;
    }
    else if (p > 0)
    {
        close(pty->slave);
        pty->pid = p;
        fprintf(stderr, "spawned new pty #%d\n", ptyi);
        return true;
    }

    perror("fork");
    return false;
}

bool reprint (PTY *pty) {
    int fd = pty->master;
    fd_set rds;
    int n;
    char buff[1024];
    struct timeval tv = { .tv_sec = 2, .tv_usec= 0 };
    bool any = false;

    while(1) {
        FD_ZERO(&rds);
        FD_SET(fd, &rds);
        if(select(fd+1, &rds, NULL, NULL, &tv) < 0) {
            perror("select");
            exit(1);
        }
        if(!(FD_ISSET(fd, &rds))) break;

        if((n = read(fd, buff, 1024)) > 0) {
            printf("%.*s", n, buff);
            any = true;
        } else {
            /* This case happens when program behind finishes.
               So I need to clean up. */
            close(fd);
            *pty = NO_PTY;
            break;
        }
    }

    return any;
}

PTY *args_to_pty (char *const args[], PTY *ptys) {
    if(args[1] == NULL) {
        fprintf(stderr, "cat: needed PTY number\n");
        return NULL;
    }

    int ptyi;
    if(sscanf(args[1], "%d", &ptyi) != 1 || !PTY_ISOK(ptys[ptyi])) {
        fprintf(stderr, "cat: wrong PTY id: %s\n", args[1]);
        return NULL;
    }

    return ptys + ptyi;
}

void ch_cat(char * const args[], PTY *ptys) {
    PTY *pty = args_to_pty(args, ptys);
    if(pty != NULL)
        reprint(pty);
}

typedef int (*TsbFun)(struct termios termopt, void *ctx);
int term_sandbox(TsbFun action, void *ctx) {
    struct termios ito, oto, eto;
    tcgetattr(STDIN_FILENO, &ito);
    tcgetattr(STDOUT_FILENO, &oto);
    tcgetattr(STDERR_FILENO, &eto);

    struct termios termopt = ito;
    int ans = action(termopt, ctx);

    tcsetattr(STDIN_FILENO, TCSAFLUSH, &ito);
    tcsetattr(STDOUT_FILENO, TCSAFLUSH, &oto);
    tcsetattr(STDERR_FILENO, TCSAFLUSH, &eto);
    
    return ans;
}

int rewrite_fds(uint count, int *ifds, int *ofds, int waitmask) {
    char buff[BUFF_SIZE+1];
    buff[BUFF_SIZE] = 0;

    fd_set rds;
    struct timeval tv = { .tv_sec = 2, .tv_usec= 0 };
    int maxfd = ifds[0];
    for(uint i = 1; i < count; i++)
        if(ifds[i] > maxfd)
            maxfd = ifds[i];
    maxfd++;

    int deadmask = 0;
    do {
        FD_ZERO(&rds);
        for(uint i = 0; i < count; i++) {
            FD_SET(ifds[i], &rds);
        }

        if(select(maxfd, &rds, NULL, NULL, &tv) < 0) {
            perror("select");
            return -1;
        }

        deadmask = 0;
        for(uint i = 0; i < count; i++) {
            if(FD_ISSET(ifds[i], &rds)) {
                int n = read(ifds[i], buff, BUFF_SIZE);
                if(n > 0) {
                    write(ofds[i], buff, n);
                } else {
                    deadmask |= 1 << i;
                }
            }
        }
    } while((deadmask & waitmask) == 0);

    return deadmask;
}

int pty_foreground(struct termios termopt, PTY *pty) {
    termopt.c_lflag &= ~(ICANON | ECHO);
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &termopt);

    rewrite_fds(2, (int[]) { STDIN_FILENO, pty->master },
                   (int[]) { pty->master, STDOUT_FILENO },
                   0x02);

    int status;
    waitpid(pty->pid, &status, 0);
    return status;
}

void ch_fg(char *const args[], PTY *ptys) {
    PTY *pty = args_to_pty(args, ptys);
    if(pty != NULL) {
        int status = term_sandbox((TsbFun)&pty_foreground, pty);
        printf("pid #%d finished with status %d\n", (int)pty->pid, status);
        *pty = NO_PTY;
    }
}

CHLine handlers[] = {
    {".cat", &ch_cat},
    {".fg", &ch_fg}
};

void prompt() {
    PTY ptys[MAX_PTYS];
    for(size_t i = 0; i < MAX_PTYS; i++)
        ptys[i] = NO_PTY;

    char buff[BUFF_SIZE+1];
    buff[BUFF_SIZE] = 0;
    char *args[MAX_ARGS];

    while(printf("> "), fgets(buff, BUFF_SIZE, stdin)) {
        char *icur = buff;
        char **ocur = args;
        while(isspace(*icur)) icur++;
        while(*icur != 0) {
            *ocur = icur;
            while(*icur > 0 && !isspace(*icur)) icur++;
            if(*icur > 0) {
                *icur = 0;
                icur++;
                while(isspace(*icur)) icur++;
            }
            ocur++;
        }

        *ocur = NULL;
        if(*args == NULL) continue;

        CommandHandler ch = NULL;
        for(size_t i = 0; i < sizeof(handlers) / sizeof(CHLine); i++) {
            if(strcmp(args[0], handlers[i].cmd) == 0) {
                ch = handlers[i].handler;
                break;
            }
        }

        if(ch != NULL) {
            ch(args, ptys);
        } else {
            spawn(args, ptys);
        }
    }
}

int main() {
    sigignore(SIGTSTP);
    prompt();

    return 0;
}

