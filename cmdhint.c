#include "cmdhint.h"

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

CmdHint new_cmdhint(const CHLine *builtins, size_t builtins_count) {
    return (CmdHint) { .path = NULL, .dh = NULL, .prefix_hash = 0, .prefix_len = 0,
                       .builtins = builtins, .builtins_count = builtins_count };
}

const char *next_cmdhint(CmdHint *ch, const char *prefix) {
    size_t plen = strlen(prefix);
    Hash nhash = hashof(prefix, plen);
    if(ch->prefix_len != plen || nhash != ch->prefix_hash) {
        if(ch->next_path != NULL)
            ch->next_path[-1] = ':';
        ch->path = getenv("PATH");

        #ifdef DEBUG
        fprintf(stderr, "HOME>%s\n", ch->path);
        #endif 
        
        ch->current_hint = NULL;
        ch->next_path = NULL;
        ch->prefix_hash = nhash;
        ch->prefix_len = plen;
        ch->dh = NULL;
        ch->builtins_cur = 0;
    } else if(ch->path == NULL) {
        if(ch->next_path != NULL)
            ch->next_path[-1] = ':';
        ch->path = getenv("PATH");
        ch->next_path = NULL;
        ch->builtins_cur = 0;
    }

    while(1) {
        if(ch->path == NULL || ch->prefix_len == 0)
            break;
    
        while(ch->builtins_cur < ch->builtins_count) {
            const char *name = ch->builtins[ch->builtins_cur++].cmd;
            Hash h = hashof(name, ch->prefix_len);
            if(h == ch->prefix_hash) {
                ch->current_hint = name;
                return name;
            }
        }

        if(ch->dh == NULL) {
            uint i;
            for(i = 0; ch->path[i] != 0 && ch->path[i] != ':'; i++);
            if(ch->path[i] == ':') {
                ch->next_path = ch->path + i + 1;
                ch->path[i] = 0;
            } else {
                ch->next_path = NULL;
            }
    
            ch->dh = opendir(ch->path);

            #ifdef DEBUG
            fprintf(stderr, "open %s\n", ch->path);
            #endif
    
            if(ch->dh == NULL) break;
        }
    
        struct dirent *de = readdir(ch->dh);
        if(de == NULL) {
            closedir(ch->dh);
            ch->dh = NULL;
            ch->path = ch->next_path;
            /* getenv() command is likely to point at the same
               place remaining unchanged at all times, so I must
               undo my putting '\0' instead of ':' : */
            if(ch->next_path != NULL)
                ch->next_path[-1] = ':';
        } else {
            Hash hash2 = hashof(de->d_name, plen);

            #ifdef DEBUG
            fprintf(stderr, "%lx %lx %s\n", hash2, ch->prefix_hash, de->d_name);
            #endif

            // 0x2e = '.' - excluding . & ..
            if(hash2 != 0x2e && hash2 != 0x2e2e && hash2 == ch->prefix_hash)
                return ch->current_hint = de->d_name;
        }
    }

    return NULL;
}
