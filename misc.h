#ifndef __H_MISC
#define __H_MISC 1

#define _XOPEN_SOURCE 600
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
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

#endif
