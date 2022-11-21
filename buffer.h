#ifndef __H_BUFFER
#define __H_BUFFER 1

#include "misc.h"
#include <pthread.h>

typedef struct {  
    FlatBuff  buff;
    int       fh;
    pthread_t th;
    bool      canceled, sleep, finished, free;
} BufferThread;

BufferThread *buffer_thread(int fh);
bool buffer_wait(BufferThread *buff, int nanosec);

#endif
