CFLAGS += -g -std=c11 -Wall -Wextra -pedantic -pthread 
 
.PHONY: all clean

all: ush

ush: fds.o ush.o prompt.o

clean:
	rm ush ush.o fds.o prompt.o
