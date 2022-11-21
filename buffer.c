#include "buffer.h"

static void *bufferfun(BufferThread *ctx) {
    fd_set fds;
    FD_ZERO(&fds);
    FD_SET(ctx->fh, &fds);
    while(1) {
        if(select(ctx->fh+1, &fds, NULL, NULL, NULL) != 1
           || ctx->canceled)
            break;
        while(ctx->sleep);
        int ret = FlatBuff_readfh(&(ctx->buff), ctx->fh);
        if(ret < 0 || ctx->canceled)
            break;
    }

    ctx->finished = true;
    return NULL;
}

BufferThread threads[BUFFTHREAD_MAX];
uint n_threads = 0;

BufferThread *buffer_thread(int fh) {
    BufferThread *thread = NULL;
    for(uint i = 0; i < n_threads; i++)
        if(threads[i].free == true) {
            thread = threads + i;
            thread->buff.pos = 0;
            break;
        }

    if(thread == NULL) {
        if(n_threads < BUFFTHREAD_MAX) {
            thread = threads + (n_threads++);
            thread->buff = FlatBuff_new(PTY_OUTBUFF_LEN);
        } else {
            fprintf(stderr, "no more buffer threads\n");
            return NULL;
        }
    }

    thread->finished = thread->free = thread->canceled = thread->sleep = false;
    thread->fh = fh;

    int ret = pthread_create(&(thread->th), NULL, (void *(*)(void*)) &bufferfun, (void*)thread);

    if(ret == 0) {
        return thread;
    } else {
        n_threads--;
        return NULL;
    }
}

bool buffer_wait(BufferThread *buff, int usec) {
    usleep(usec);
    return (buff->finished);
}
