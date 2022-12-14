#include "misc.h"

bool pushstr (StrList *tgt, ConstStr str) {
    if(tgt->strpos == tgt->strlen
       || tgt->charpos + str.len + 1 > tgt->charlen)
        return false;

    memcpy(tgt->charbuff + tgt->charpos, str.str, str.len);
    ConstStr instr = (ConstStr) {
        .str = tgt->charbuff + tgt->charpos,
        .len = str.len
    };
                                
    tgt->charpos += str.len;
    tgt->charbuff[tgt->charpos++] = '\0';
    tgt->strbuff[tgt->strpos++] = instr;

    return true;
}

void resetlist (StrList *tgt) {
    tgt->charpos = tgt->strpos = 0;
}

ConstStr path_str(ConstStr s) {
    while(s.str[s.len-1] == '/') s.len--;
    return s;
}

int ConstStr_pathcmp(const void* aptr, const void *bptr) {
    ConstStr a = path_str(*((const ConstStr*)aptr));
    ConstStr b = path_str(*((const ConstStr*)bptr));

    size_t cmplen = (a.len > b.len)?b.len:a.len;
    for(size_t i = 0; i < cmplen; i++) {
        int rel = a.str[i] - b.str[i];
        if(rel != 0) return rel;
    }

    return a.len - b.len;
}

bool writestr(int fd, ConstStr str) {
    return write(fd, str.str, str.len) == (ssize_t)str.len;
}

Hash hashof(const char *txt, size_t len) {
    Hash ans = 0;
    int l = len > 8? 8: len;
    for(int i = 0; i < l; i++)
        ans |= txt[i] << (8*i);

    l = len - 8;
    txt += 8;
    uint off2 = 1;
    while(l > 0) {
        int l2 = l > 7? 7: l;
        for(int i = 0; i < l2; i++) {
            ans ^= ~(txt[i]) << (8*i + off2);
        }
        
        l -= 7;
        off2 = (off2 + 1)%8;
        txt += 7;
    }

    return ans;
}

Hash hashofstr(ConstStr str) {
    return hashof(str.str, str.len);
}

void uniq(StrList *tgt) {
    if(tgt->strpos < 2) return;

    Hash chash = hashofstr(tgt->strbuff[0]);
    size_t cur = 1;
    for(size_t i = 1; i < tgt->strpos; i++) {
        Hash nhash = hashofstr(tgt->strbuff[i]);
        if(chash != nhash) {
            tgt->strbuff[cur++] = tgt->strbuff[i];
            chash = nhash;
        }
    }

    tgt->strpos = cur;
}

FlatBuff FlatBuff_new(size_t len) {
    return (FlatBuff) {
        .data = malloc(len),
        .len = len,
        .pos = 0
    };
}

void FlatBuff_free(FlatBuff buff) {
    free(buff.data);
}

int FlatBuff_readfh(FlatBuff *buff, int fh) {
    int rem = (int)buff->len - (int)buff->pos;
    if(rem <= 0)
        return 0;

    int len = read(fh, buff->data + buff->pos, rem);
    if(len > 0)
        buff->pos += len;

    return len;
}

