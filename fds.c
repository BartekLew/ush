#include "fds.h"

PTY newPty() {
    PTY ans = {.suspended = false};

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

int rewrite_fds(uint count, int *ifds, int *ofds, int waitmask, RewriteFilter filter) {
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
            if(errno == ERRNO_SIGCAUGHT) {
                if(filter(buff, 0))
                    return 0;
                continue;
            }

            perror("select");
            return -1;
        }

        deadmask = 0;
        for(uint i = 0; i < count; i++) {
            if(FD_ISSET(ifds[i], &rds)) {
                int n = read(ifds[i], buff, BUFF_SIZE);
                if(filter != NULL && filter(buff, n))
                    return 0;

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

