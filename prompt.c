#include "prompt.h"

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

volatile bool has_sigstop = false;

bool fg_filter(const char *buff, int size) {
    UNUSED(buff);
    UNUSED(size);
    bool ans = has_sigstop;
    has_sigstop = false;
    return ans;
}

int pty_foreground(PTY *pty) {
    if(pty->suspended) {
        kill(pty->pid, SIGCONT);
        pty->suspended = false;
    }

    int deadmask = rewrite_fds(2, (int[]) { STDIN_FILENO, pty->master },
                                  (int[]) { pty->master, STDOUT_FILENO },
                               0x02, &fg_filter);

    int status;
    if(deadmask != 0) {
        waitpid(pty->pid, &status, 0);
        return status;
    }

    return STOP_DEADMASK;
}

int pty_readkey() {
    int c;
    while(read(STDIN_FILENO, &c, 4) > 0 && c != CTRL_D) {
        printf("0x%.8x\r", c);
        fflush(stdout);
        c = 0;
    }

    return 0;
}

char buff[BUFF_SIZE+1];
int buff_pos = 0;
int line_start = 0;

char *readl(int fd) {
    int n;
    int old_line = line_start;
    while((n = read(fd, buff+buff_pos, BUFF_SIZE-buff_pos)) > 0) {
        write(STDOUT_FILENO, buff+buff_pos, n);

        int end = -1;
        for(int i = 0; i < n; i++) {
            if(buff[buff_pos+i] == '\n') {
                end = i;
                break;
            }
        }

        buff_pos += n;
        if(end >= 0) {
            if(end == n-1) {
                line_start = buff_pos = 0;
            }
            break;
        }
    }

    return buff + old_line;
}

struct termios ito, oto, eto;
void prompt(const CHLine *handlers, size_t handlers_cnt) {
    PTY ptys[MAX_PTYS];
    for(size_t i = 0; i < MAX_PTYS; i++)
        ptys[i] = NO_PTY;

    char *args[MAX_ARGS];
    char *icur;

    tcgetattr(STDIN_FILENO, &ito);
    tcgetattr(STDOUT_FILENO, &oto);
    tcgetattr(STDERR_FILENO, &eto);

    struct termios termopt = ito;
    termopt.c_lflag &= ~(ICANON | ECHO);
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &termopt);

    while(write(STDOUT_FILENO, "> ", 2), icur = readl(STDIN_FILENO)) {
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
        for(size_t i = 0; i < handlers_cnt; i++) {
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

    reset_tty();
}

void reset_tty() {
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &ito);
    tcsetattr(STDOUT_FILENO, TCSAFLUSH, &oto);
    tcsetattr(STDERR_FILENO, TCSAFLUSH, &eto);
}
