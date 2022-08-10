#define _XOPEN_SOURCE 600
#include <ctype.h>
#include <fcntl.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/select.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <termios.h>
#include <unistd.h>

#define SHELL "/bin/bash"

typedef struct PTY
{
    int master, slave;
} PTY;

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
    ans.slave = open(ptsn, O_RDWR | O_NOCTTY);
    if(ans.slave == -1) {
        perror("open(pts_slave)");
        exit(1);
    }

    return ans;
}

bool spawn(PTY *pty)
{
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

        execvp(SHELL, (char *[]){SHELL, NULL});
        return true;
    }
    else if (p > 0)
    {
        close(pty->slave);
        return true;
    }

    perror("fork");
    return false;
}

int newShell() {
    PTY pty = newPty();
    if(spawn(&pty)) {
        return pty.master;
    } else 
        return -1;
}

bool reprint (int fd) {
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
        }
    }

    return any;
}

int main() {
    int shell = newShell();
    reprint(shell);

    char buff[1024];
    while(fgets(buff, 1024, stdin)) {
        if(buff[0] == '!')
            printf("%s\n", buff);
        else {
            write(shell, buff, strlen(buff));
            reprint(shell);
        }
    }

    return 0;
}

