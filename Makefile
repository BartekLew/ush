CFLAGS += -g -std=c11 -Wall -Wextra -fstack-protector -pthread
 
.PHONY: all clean

all: ush

ush: fds.o ush.o prompt.o cmdhint.o misc.o

clean:
	rm ush ush.o fds.o prompt.o misc.o
