#include "cmdhint.h"

CmdHint new_cmdhint(const CHLine *builtins, size_t builtins_count) {
    return (CmdHint) { .path = NULL, .dh = NULL, .prefix_hash = 0, .prefix_len = 0,
                       .builtins = builtins, .builtins_count = builtins_count,
                       .ht_flags = HT_CMD | HT_DIR };
}

static bool apply_prefix(CmdHint *ch, const char *prefix) {
    size_t plen = strlen(prefix);
    Hash nhash = hashof(prefix, plen);
    if(ch->prefix_len != plen || nhash != ch->prefix_hash) {
        if(ch->ht_flags & HT_CMD) {
            if(ch->next_path != NULL)
                ch->next_path[-1] = ':';
            ch->path = getenv("PATH");
        }

        #ifdef DEBUG
        fprintf(stderr, "HOME>%s\n", ch->path);
        #endif 
        
        ch->current_hint = nostr;
        ch->next_path = NULL;
        ch->prefix_hash = nhash;
        ch->prefix_len = plen;
        if((ch->ht_flags & HT_CUSTOMDIR) == 0)
            ch->dh = NULL;
        ch->builtins_cur = 0;
        ch->hintpos = 0;
        
        return true;
    } 

    return false;
}

static ConstStr get_builtin_hint(CmdHint *ch) {
    while(ch->builtins_cur < ch->builtins_count) {
        const char *name = ch->builtins[ch->builtins_cur++].cmd;
        Hash h = hashof(name, ch->prefix_len);
        if(h == ch->prefix_hash) {
            size_t len = strlen(name);
            return ch->current_hint = (ConstStr) { .str = name, .len = len };
        }
    }

    return nostr;
}

static bool has_path(CmdHint *ch) {
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

        return ch->dh != NULL;
    }
    return true;
}

static ConstStr try_de(CmdHint *ch, struct dirent *de) {
    Hash hash2 = hashof(de->d_name, ch->prefix_len);

    #ifdef DEBUG
    fprintf(stderr, "%lx %lx %s\n", hash2, ch->prefix_hash, de->d_name);
    #endif

    if(hash2 == ch->prefix_hash) {
        const char *name = de->d_name;
        size_t len = strlen(name);
        return ch->current_hint = (ConstStr) { .str = name, .len = len };
    }

    return nostr;
}

static ConstStr try_next_file(CmdHint *ch) {
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
    } else if(de->d_type != DT_DIR)
        return try_de(ch, de);

    return nostr;
}

ConstStr next_cmd(CmdHint *ch) {
    while(1) {
        if(ch->path == NULL || ch->prefix_len == 0)
            break;
    
        ConstStr ans = get_builtin_hint(ch);
        if(ans.str != NULL)
            return ans;

        if(!has_path(ch))
            break;

        ans = try_next_file(ch);
        if(ans.str != NULL)
            return ans;
    }

    return nostr;
}

bool test_hint_file(struct dirent *de, CmdHint *ch) {
    if(ch->ht_flags & (HT_DIR|HT_ANYFILE) && de->d_type == DT_DIR)
        return true;

    if((ch->ht_flags & HT_ANYFILE) == HT_ANYFILE && de->d_type == DT_REG)
        return true;

    if(ch->ht_flags & HT_EXEC && de->d_type == DT_REG) {
        struct stat mod;
        size_t nlen = strlen(de->d_name);
        char path[ch->current_path.len + nlen + 1];
        sprintf(path, "%.*s%s", (int)ch->current_path.len, ch->current_path.str, de->d_name);
        if(stat(path, &mod) == 0 && mod.st_mode & 00111)
            return true;
    }
            
    return false;
}


STATIC_STRLIST(cmdhints, 40000, 4000);

ConstStr next_cmdhint(CmdHint *ch, const char *prefix) {
    if(apply_prefix(ch, prefix)) {
        resetlist(&cmdhints);
        ConstStr str;
        if(ch->ht_flags & HT_CMD)
            while(str = next_cmd(ch), str.str != NULL)
                pushstr(&cmdhints, str);

        if(ch->ht_flags & ~(HT_CMD | HT_CUSTOMDIR)) {
            if((ch->ht_flags & HT_CUSTOMDIR) == 0)
                ch->dh = opendir(".");

            if(ch->dh) {
                struct dirent *de;
                while((de = readdir(ch->dh)) != NULL) {
                    if(!test_hint_file(de, ch))
                        continue;
    
                    ConstStr next = try_de(ch,de);
                    if(next.len == 0)
                        continue;

                    if(de->d_type == DT_DIR) {
                        char buff[next.len+2];
                        sprintf(buff, "%s/", next.str);
                        next.str = buff;
                        next.len++;
    
                        pushstr(&cmdhints, next);
                    } else
                        pushstr(&cmdhints,
                                (ConstStr){
                                    .str = de->d_name,
                                    .len = strlen(de->d_name)
                                });
                                
                }
            }

            if(ch->ht_flags & HT_CUSTOMDIR) {
                rewinddir(ch->dh);
            } else {
                closedir(ch->dh);
                ch->dh = NULL;
            }
        }

        if(cmdhints.strpos == 0)
            return nostr;

        ch->hints = &cmdhints;
        qsort(cmdhints.strbuff, cmdhints.strpos, sizeof(ConstStr), &ConstStr_pathcmp);
        uniq(&cmdhints);

        return ch->current_hint = cmdhints.strbuff[ch->hintpos];
    }

    if(ch->hintpos+1 < cmdhints.strpos) {
        return ch->current_hint = cmdhints.strbuff[++ch->hintpos];
    }

    return nostr;
}

ConstStr prev_cmdhint(CmdHint *ch) {
    if(ch->hintpos > 0) {
        return ch->current_hint = cmdhints.strbuff[--ch->hintpos];
    }

    return nostr;
}

