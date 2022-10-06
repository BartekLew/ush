#!/bin/bash

if [ -e .git/hooks/pre-commit ];then
    echo "hook already exists"
    exit 0
fi

cat <<'EOF' > .git/hooks/pre-commit
#!/bin/bash

uncommited=`git status --porcelain | awk '/^\?\?/ && $2 ~ /.[ch]$/ { print $2; ret=1} END {exit(ret)}'`
if [ -n "$uncommited" ]; then
    echo "uncommited source files: $uncommited" > /dev/stderr
    exit 1
fi
EOF

chmod +x .git/hooks/pre-commit
